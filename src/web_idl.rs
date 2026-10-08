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
/// The global object by its names (`window` only on a page): the scope itself, bundled only with the members warp
/// types (WindowOrWorkerGlobalScope whole: fetch, setTimeout…), so a member it does not declare stays unchecked
const SELF_GLOBALS: [&str; 3] = ["self", "globalThis", "window"];
const PAGE_ONLY_SELF: &str = "window";
/// What a program in a Worker writes for a page's global
const WORKER_ALTERNATIVES: [(&str, &str); 3] = [("localStorage", "local[k] keeps values in the page's localStorage"), ("sessionStorage", "session[k] keeps values in its sessionStorage"), ("window", "self is the Worker's global")];
/// Definitions that declare no members a program reaches
const SKIPPED_KINDS: [&str; 4] = ["dictionary", "enum", "typedef", "callback"];
const DEFINITION_WORDS: [&str; 4] = ["partial", "interface", "mixin", "namespace"];
/// Member forms checked as no attribute or operation
const SKIPPED_MEMBERS: [&str; 6] = ["const", "iterable", "maplike", "setlike", "constructor", "async"];
/// Words before a member that do not change its name or arguments
const QUALIFIERS: [&str; 7] = ["static", "stringifier", "getter", "setter", "deleter", "inherit", "readonly"];
const VARIADIC: &str = "...";
const CONSTRUCTOR: &str = "constructor";
/// `canvas.getContext("2d")`: the interface of the context a literal context id gives (RenderingContext is their union)
const CONTEXT_MEMBER: &str = "getContext";
const CONTEXT_INTERFACES: [(&str, &str); 5] = [("2d", "CanvasRenderingContext2D"), ("bitmaprenderer", "ImageBitmapRenderingContext"),
	("webgl", "WebGLRenderingContext"), ("webgl2", "WebGL2RenderingContext"), ("webgpu", "GPUCanvasContext")];
/// WebIDL's primitive types as warp's (the typedefs among them as their spec defines them: DOMHighResTimeStamp is a
/// double, EpochTimeStamp an unsigned long long)
const PRIMITIVE_TYPES: [(&[&str], &str); 4] = [
	(&["DOMString", "USVString", "ByteString", "CSSOMString"], "text"),
	(&["boolean"], "bool"),
	(&["byte", "octet", "short", "unsigned short", "long", "unsigned long", "long long", "unsigned long long", "EpochTimeStamp"], "int"),
	(&["float", "unrestricted float", "double", "unrestricted double", "DOMHighResTimeStamp"], "float"),
];

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
	Operation(Vec<Overload>),
}

#[derive(Clone, Debug)]
pub struct Overload {
	pub returns: String,
	pub arguments: Vec<Argument>,
}

#[derive(Default)]
struct Definition {
	parent: Option<String>,
	members: BTreeMap<String, Member>,
	includes: Vec<String>,
	namespace: bool,
	/// each constructor overload's arguments (`new URL(url, base)`); none: not constructible
	constructors: Vec<Vec<Argument>>,
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
	if SELF_GLOBALS.contains(&global) && (global != PAGE_ONLY_SELF || scope == PAGE_SCOPE) {
		return Some(scope.to_string());
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

/// The interface of what a member gives, read or called (`navigator.clipboard`: Clipboard,
/// `document.getElementById(id)`: Element), when WebIDL declares it whole; a nullable one (`Element?`) too
pub fn result_interface(interface: &str, member: &str, call: bool) -> Option<String> {
	let declared = declared_result(interface, member, call)?;
	let type_name = declared.trim_end_matches('?');
	definitions().contains_key(type_name).then(|| type_name.to_string())
}

/// The warp type of what `member` gives (read, or called when `call`), when WebIDL declares a value that is always one:
/// a text for DOMString, a bool, an int or a float; None for anything else, a nullable type (ø) included
pub fn result_type(interface: &str, member: &str, call: bool) -> Option<&'static str> {
	primitive_type(&declared_result(interface, member, call)?)
}

/// result_type, or the optional type for a nullable one: `text?` for `DOMString?` (ø or a text, optional_casts.rs)
pub fn optional_result_type(interface: &str, member: &str, call: bool) -> Option<String> {
	let declared = declared_result(interface, member, call)?;
	match declared.strip_suffix('?') {
		Some(nullable) => primitive_type(nullable).map(|warp_type| format!("{warp_type}?")),
		None => primitive_type(&declared).map(str::to_string),
	}
}

/// The WebIDL type of what `member` gives, when all its overloads agree; a promise's value, which a foreign call awaits
fn declared_result(interface: &str, member: &str, call: bool) -> Option<String> {
	// an Element's getContext is its HTMLCanvasElement's (check_member accepts the derived interfaces' members)
	let declaring = || derived_interfaces(interface).into_iter().find_map(|derived| members_of(&derived).remove(member));
	let declared = match (members_of(interface).remove(member).or_else(declaring)?, call) {
		(Member::Attribute(type_name), false) => type_name,
		(Member::Operation(overloads), true) => {
			let returns: Vec<String> = overloads.into_iter().map(|overload| overload.returns).collect();
			returns.iter().all(|other| *other == returns[0]).then(|| returns[0].clone())?
		}
		_ => return None,
	};
	Some(declared.strip_prefix("Promise<").and_then(|promised| promised.strip_suffix('>')).map(str::to_string).unwrap_or(declared))
}

fn primitive_type(declared: &str) -> Option<&'static str> {
	PRIMITIVE_TYPES.iter().find(|(names, _)| names.contains(&declared)).map(|(_, warp_type)| *warp_type)
}

/// `path.member` of a value of `interface` (path as the program writes it: `navigator.clipboard`)
pub fn check_member(interface: &str, path: &str, member: &str, call: Option<usize>) -> Result<(), String> {
	let mut members = members_of(interface);
	let Some(declared) = members.remove(member) else {
		// `document.getElementById(id).value`: an Element that is an HTMLInputElement at run time
		let derived = derived_interfaces(interface);
		if let Some(declaring) = derived.iter().find(|derived| members_of(derived).contains_key(member)) {
			return check_member(declaring, path, member, call);
		}
		if [PAGE_SCOPE, PROGRAM_SCOPE].contains(&interface) {
			return Ok(()); // `window.innerWidth`: the scope is bundled in part
		}
		let mut names: Vec<String> = members.into_keys().collect();
		names.extend(derived.iter().flat_map(|derived| members_of(derived).into_keys()));
		let same_letters = names.iter().find(|name| name.eq_ignore_ascii_case(member)).cloned();
		let near = same_letters.or_else(|| crate::extensions::strings::near_miss(member, names)).map(|near| format!("; did you mean {near}?")).unwrap_or_default();
		return Err(format!("{path} ({interface} in WebIDL) has no member {member}{near}"));
	};
	match (declared, call) {
		(Member::Attribute(type_name), Some(_)) => Err(format!("{path}.{member} is an attribute ({type_name}), read without a call: {path}.{member}")),
		(Member::Operation(overloads), Some(count)) if !overloads.iter().any(|overload| takes(&overload.arguments, count)) => {
			let forms: Vec<String> = overloads.iter().map(|overload| format!("{member}({})", signature(&overload.arguments))).collect();
			Err(format!("{path}.{member} takes {}, not {count} argument{}", forms.join(" or "), if count == 1 { "" } else { "s" }))
		}
		_ => Ok(()),
	}
}

/// The interfaces deriving from `interface`, at any depth (HTMLInputElement of Element), sorted by name
fn derived_interfaces(interface: &str) -> Vec<String> {
	let derives = |name: &str| {
		let mut parent = definitions().get(name).and_then(|definition| definition.parent.as_deref());
		while let Some(ancestor) = parent {
			if ancestor == interface {
				return true;
			}
			parent = definitions().get(ancestor).and_then(|definition| definition.parent.as_deref());
		}
		false
	};
	let mut derived: Vec<String> = definitions().keys().filter(|name| derives(name)).cloned().collect();
	derived.sort();
	derived
}

/// `URL(text)` of `use js URL`, the constructor called (JavaScript's `new`): the interface the value is, None when
/// WebIDL does not declare one of that name; Err for no constructor or an argument count none takes
pub fn check_constructor(interface: &str, count: usize) -> Result<Option<String>, String> {
	let Some(definition) = definitions().get(interface).filter(|definition| !definition.namespace) else { return Ok(None) };
	if definition.constructors.is_empty() {
		return Err(format!("{interface} has no constructor in WebIDL: its values come from the platform"));
	}
	if !definition.constructors.iter().any(|arguments| takes(arguments, count)) {
		let forms: Vec<String> = definition.constructors.iter().map(|arguments| format!("{interface}({})", signature(arguments))).collect();
		return Err(format!("{interface} takes {}, not {count} argument{}", forms.join(" or "), if count == 1 { "" } else { "s" }));
	}
	Ok(Some(interface.to_string()))
}

/// The interface `getContext(id)` gives for a literal id when the bundle declares it (`"2d"`: CanvasRenderingContext2D;
/// an OffscreenCanvas's "2d" is OffscreenCanvasRenderingContext2D)
pub fn context_interface(interface: &str, member: &str, context_id: &str) -> Option<String> {
	if member != CONTEXT_MEMBER {
		return None;
	}
	let (_, context) = CONTEXT_INTERFACES.iter().find(|(id, _)| *id == context_id)?;
	let offscreen = format!("Offscreen{context}");
	let context = if interface.starts_with("Offscreen") && definitions().contains_key(&offscreen) { offscreen } else { context.to_string() };
	definitions().contains_key(&context).then_some(context)
}

/// Does WebIDL declare a constructor of `interface` (`URL(text)` is a URL)
pub fn constructible(interface: &str) -> bool {
	definitions().get(interface).is_some_and(|definition| !definition.constructors.is_empty())
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
			match without_extended_attributes(&member).trim().strip_prefix(CONSTRUCTOR) {
				Some(signature) if signature.trim_start().starts_with('(') => definition.constructors.push(arguments_of(signature)),
				_ => add_member(&mut definition.members, &member),
			}
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
			let Some((returns, name)) = typed_name(&source[..open]) else { return }; // a nameless getter
			let arguments = arguments_of(&source[open..]);
			match members.entry(name).or_insert_with(|| Member::Operation(vec![])) {
				Member::Operation(overloads) => overloads.push(Overload { returns, arguments }),
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

/// The arguments of `(DOMString url, optional DOMString base)`
fn arguments_of(signature: &str) -> Vec<Argument> {
	let open = signature.find('(').map_or(0, |open| open + 1);
	let close = signature.rfind(')').unwrap_or(signature.len());
	split_top_level(&signature[open..close], ',').iter().filter_map(|argument| parse_argument(argument)).collect()
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
