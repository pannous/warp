//! WebIDL, the browser platform's own declarations (lib/web.webidl, bundled from w3c/webref by
//! scripts/webidl_bundle.py): `use js crypto` is typed by the interface of the global crypto as `use c` is by the C
//! headers (card web-apis). A member the interface lacks, an attribute called, or a call with an argument count no
//! overload takes is an error before the program runs. A global WebIDL does not declare (`Math`, a node module) stays
//! unchecked.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

const BUNDLE: &str = include_str!("../lib/web.webidl");
/// The interface whose attributes are a page's globals (`navigator`, `localStorage`, `crypto` through its mixins)
const PAGE_SCOPE: &str = "Window";
/// Where a program's globals come from: a page natively (node mirrors it, a built site runs there); the playground and
/// the browser suite run programs in a Worker, where `localStorage` does not exist and `navigator` is a WorkerNavigator
#[cfg(feature = "native")]
const PROGRAM_SCOPE: &str = PAGE_SCOPE;
#[cfg(not(feature = "native"))]
const PROGRAM_SCOPE: &str = "WorkerGlobalScope";
/// What a program in a Worker writes for a page's global
const WORKER_ALTERNATIVES: [(&str, &str); 2] = [("localStorage", "local[k] keeps values in the page's localStorage"), ("sessionStorage", "session[k] keeps values in its sessionStorage")];
/// Definitions that declare no members a program reaches
const SKIPPED_KINDS: [&str; 4] = ["dictionary", "enum", "typedef", "callback"];
const DEFINITION_WORDS: [&str; 4] = ["partial", "interface", "mixin", "namespace"];
/// Member forms checked as no attribute or operation
const SKIPPED_MEMBERS: [&str; 6] = ["const", "iterable", "maplike", "setlike", "constructor", "async"];
/// Words before a member that do not change its name or arguments
const QUALIFIERS: [&str; 7] = ["static", "stringifier", "getter", "setter", "deleter", "inherit", "readonly"];
const VARIADIC: &str = "...";

#[derive(Clone, Debug)]
pub struct Argument {
	pub name: String,
	pub type_name: String,
	pub optional: bool,
	pub variadic: bool,
}

#[derive(Clone, Debug)]
pub enum Member {
	Attribute(String),
	/// each overload's arguments
	Operation(Vec<Vec<Argument>>),
}

#[derive(Default)]
struct Definition {
	parent: Option<String>,
	members: BTreeMap<String, Member>,
	includes: Vec<String>,
	namespace: bool,
}

fn definitions() -> &'static HashMap<String, Definition> {
	static PARSED: OnceLock<HashMap<String, Definition>> = OnceLock::new();
	PARSED.get_or_init(|| parse(BUNDLE))
}

/// The interface a global of `scope` is: a namespace of that name (`console`), else the type of the scope's attribute
pub fn interface_of_global(global: &str, scope: &str) -> Option<String> {
	if definitions().get(global).is_some_and(|definition| definition.namespace) {
		return Some(global.to_string());
	}
	match members_of(scope).remove(global)? {
		Member::Attribute(type_name) => Some(type_name.trim_end_matches('?').to_string()),
		Member::Operation(_) => None,
	}
}

/// Every member of an interface: its own, its mixins' and its parents'
pub fn members_of(interface: &str) -> BTreeMap<String, Member> {
	let Some(definition) = definitions().get(interface) else { return BTreeMap::new() };
	let mut members = definition.parent.as_deref().map(members_of).unwrap_or_default();
	for mixin in &definition.includes {
		members.extend(members_of(mixin));
	}
	members.extend(definition.members.clone());
	members
}

/// `global.member` read (`call` None) or called with `call` arguments: Err says what WebIDL declares instead
pub fn check(global: &str, member: &str, call: Option<usize>) -> Result<(), String> {
	match global_interface(global)? {
		Some(interface) => check_member(&interface, global, member, call),
		None => Ok(()),
	}
}

/// The interface a global is in the program's scope: None when WebIDL does not declare it, Err when only a page has it
pub fn global_interface(global: &str) -> Result<Option<String>, String> {
	match interface_of_global(global, PROGRAM_SCOPE) {
		Some(interface) => Ok(Some(interface)),
		None => page_only(global).map(|_| None),
	}
}

/// The interface of an attribute's value (`navigator.clipboard`: Clipboard), when WebIDL declares it whole
pub fn attribute_interface(interface: &str, member: &str) -> Option<String> {
	let Member::Attribute(type_name) = members_of(interface).remove(member)? else { return None };
	let type_name = type_name.trim_end_matches('?');
	definitions().contains_key(type_name).then(|| type_name.to_string())
}

/// `path.member` of a value of `interface` (path as the program writes it: `navigator.clipboard`)
pub fn check_member(interface: &str, path: &str, member: &str, call: Option<usize>) -> Result<(), String> {
	let mut members = members_of(interface);
	let Some(declared) = members.remove(member) else {
		let same_letters = members.keys().find(|name| name.eq_ignore_ascii_case(member)).cloned();
		let near = same_letters.or_else(|| crate::extensions::strings::near_miss(member, members.into_keys())).map(|near| format!("; did you mean {near}?")).unwrap_or_default();
		return Err(format!("{path} ({interface} in WebIDL) has no member {member}{near}"));
	};
	match (declared, call) {
		(Member::Attribute(type_name), Some(_)) => Err(format!("{path}.{member} is an attribute ({type_name}), read without a call: {path}.{member}")),
		(Member::Operation(overloads), Some(count)) if !overloads.iter().any(|arguments| takes(arguments, count)) => {
			let forms: Vec<String> = overloads.iter().map(|arguments| format!("{member}({})", signature(arguments))).collect();
			Err(format!("{path}.{member} takes {}, not {count} argument{}", forms.join(" or "), if count == 1 { "" } else { "s" }))
		}
		_ => Ok(()),
	}
}

/// A global of a page that the program's scope lacks (a Worker's: localStorage), else unchecked
fn page_only(global: &str) -> Result<(), String> {
	let Some(interface) = interface_of_global(global, PAGE_SCOPE).filter(|_| PROGRAM_SCOPE != PAGE_SCOPE) else { return Ok(()) };
	let instead = WORKER_ALTERNATIVES.iter().find(|(name, _)| *name == global).map(|(_, instead)| format!(": {instead}")).unwrap_or_default();
	Err(format!("{global} ({interface}) exists on a page, not in the Worker this program runs in{instead}"))
}

fn takes(arguments: &[Argument], count: usize) -> bool {
	let required = arguments.iter().filter(|argument| !argument.optional && !argument.variadic).count();
	count >= required && (count <= arguments.len() || arguments.iter().any(|argument| argument.variadic))
}

fn signature(arguments: &[Argument]) -> String {
	let shown = |argument: &Argument| {
		let optional = if argument.optional { "optional " } else { "" };
		let variadic = if argument.variadic { VARIADIC } else { "" };
		format!("{optional}{}{variadic} {}", argument.type_name, argument.name)
	};
	arguments.iter().map(shown).collect::<Vec<_>>().join(", ")
}

// ---- parsing -----------------------------------------------------------------------------------------------------

fn parse(text: &str) -> HashMap<String, Definition> {
	let mut parsed: HashMap<String, Definition> = HashMap::new();
	for chunk in split_top_level(&without_comments(text), ';') {
		let chunk = without_extended_attributes(&chunk);
		let words: Vec<&str> = chunk.split(|c: char| c.is_whitespace() || c == '{').filter(|word| !word.is_empty()).collect();
		if let [target, "includes", mixin] = words.as_slice() {
			parsed.entry(target.to_string()).or_default().includes.push(mixin.to_string());
			continue;
		}
		let Some(open) = chunk.find('{') else { continue };
		let header = chunk[..open].replace(':', " : ");
		let words: Vec<&str> = header.split_whitespace().collect();
		if words.iter().any(|word| SKIPPED_KINDS.contains(word)) {
			continue;
		}
		let named: Vec<&str> = words.into_iter().filter(|word| !DEFINITION_WORDS.contains(word)).collect();
		let (name, parent) = match named.as_slice() {
			[name] => (name.to_string(), None),
			[name, ":", parent] => (name.to_string(), Some(parent.to_string())),
			_ => continue,
		};
		let body = &chunk[open + 1..chunk.rfind('}').unwrap_or(chunk.len())];
		let definition = parsed.entry(name).or_default();
		definition.namespace |= header.split_whitespace().any(|word| word == "namespace");
		definition.parent = parent.or(definition.parent.take());
		for member in split_top_level(body, ';') {
			add_member(&mut definition.members, &member);
		}
	}
	parsed
}

fn add_member(members: &mut BTreeMap<String, Member>, source: &str) {
	let source = without_extended_attributes(source);
	let mut words: Vec<&str> = source.split_whitespace().collect();
	if words.first().is_none_or(|word| SKIPPED_MEMBERS.iter().any(|skipped| word.starts_with(skipped))) {
		return;
	}
	while words.first().is_some_and(|word| QUALIFIERS.contains(word)) {
		words.remove(0);
	}
	let source = words.join(" ");
	match source.find('(') {
		Some(open) if !source[..open].contains("attribute ") => {
			let Some((_, name)) = typed_name(&source[..open]) else { return }; // a nameless getter
			let close = source.rfind(')').unwrap_or(source.len());
			let arguments = split_top_level(&source[open + 1..close], ',').iter().filter_map(|argument| parse_argument(argument)).collect();
			match members.entry(name).or_insert_with(|| Member::Operation(vec![])) {
				Member::Operation(overloads) => overloads.push(arguments),
				Member::Attribute(_) => {}
			}
		}
		_ => {
			let Some(declared) = source.split_once("attribute ").map(|(_, declared)| declared) else { return };
			if let Some((type_name, name)) = typed_name(declared) {
				members.insert(name, Member::Attribute(type_name));
			}
		}
	}
}

/// `optional DOMString label = "default"`, `any... data`
fn parse_argument(source: &str) -> Option<Argument> {
	let source = without_extended_attributes(source);
	let source = source.split_once('=').map_or(source.as_str(), |(declared, _)| declared).trim();
	let (optional, source) = match source.strip_prefix("optional ") {
		Some(rest) => (true, rest),
		None => (false, source),
	};
	let (type_name, name) = typed_name(source)?;
	let variadic = type_name.ends_with(VARIADIC);
	Some(Argument { name, type_name: type_name.trim_end_matches(VARIADIC).to_string(), optional, variadic })
}

/// `FrozenArray<DOMString> languages` → (type, name); WebIDL escapes a name that is a keyword with `_`
fn typed_name(source: &str) -> Option<(String, String)> {
	let source = source.trim();
	let split = source.rfind(char::is_whitespace)?;
	let name = source[split + 1..].trim_start_matches('_');
	Some((source[..split].trim().to_string(), name.to_string()))
}

fn without_comments(text: &str) -> String {
	let lines = text.lines().map(|line| line.split_once("//").map_or(line, |(code, _)| code));
	lines.collect::<Vec<_>>().join("\n")
}

/// `[Exposed=Window] interface …` → `interface …`: every bracketed list of extended attributes dropped
fn without_extended_attributes(text: &str) -> String {
	let mut depth = 0;
	let kept: String = text.chars().filter(|c| {
		match c {
			'[' => depth += 1,
			']' => {
				depth -= 1;
				return false;
			}
			_ => {}
		}
		depth == 0
	}).collect();
	kept.trim().to_string()
}

/// `text` split at `separator` outside brackets, braces, parentheses and angle brackets
fn split_top_level(text: &str, separator: char) -> Vec<String> {
	let (mut parts, mut current, mut depth) = (vec![], String::new(), 0i32);
	for c in text.chars() {
		match c {
			'{' | '[' | '(' | '<' => depth += 1,
			'}' | ']' | ')' | '>' => depth -= 1,
			_ if c == separator && depth == 0 => {
				parts.push(std::mem::take(&mut current).trim().to_string());
				continue;
			}
			_ => {}
		}
		current.push(c);
	}
	parts.push(current.trim().to_string());
	parts.into_iter().filter(|part| !part.is_empty()).collect()
}
