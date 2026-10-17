//! Reflection words (card reflection, notes/reflection.md), read as dot forms (`effects of f` is `f.effects`,
//! of_lookups.rs): `f.effects` is answered by effects.rs, `e.listeners` by event_signals.rs and signal_values.rs.
//! `listeners of x` for a name nothing listens to is loud (card listeners-tick).
//! Objects known at compile time (step 2): an instance's `p.class` and `p.type` are `type(p)`; `p.fields` (aliases
//! `attributes`, `members`), `p.methods` and `dir(p)` of an instance or a class are the names of its class layout,
//! inherited fields first; of a map literal's variable they are its keys, read at run time (`m.keys`).

use super::nodes::{call, key};
use crate::words::{DIR_WORD, MAP_WORD};
use std::collections::{HashMap, HashSet};

use crate::class_methods::ClassLayout;
use crate::node::{symbol, text, Bracket, Node, Separator};
use crate::operators::Op;

use crate::analyzer::effects::EFFECTS_WORD;
const LISTENERS_WORD: &str = "listeners";
/// `on alarm {…}`, `once tick {…}`, `on set x {…}`: the words before a listened name
const LISTENING_WORDS: [&str; 2] = ["on", "once"];
const VARIABLE_EVENTS: [&str; 2] = ["set", "change"];
const CLASS_WORDS: [&str; 2] = ["class", crate::type_tests::TYPE_WORD];
/// `fields(p)`, `attributes of p`, `p.properties`: the field names, one word in several spellings (card reflection-synonyms)
const FIELDS_WORDS: [&str; 4] = ["fields", "attributes", "members", "properties"];
const METHODS_WORD: &str = "methods";
/// `methods(p)`, `functions of p`: the functions taking p first (typed first parameter only, card one-method-form)
const METHODS_WORDS: [&str; 2] = [METHODS_WORD, "functions"];
/// A class's definition as written, in its warp.meta entry: a program importing the module declares the class too
pub const DEFINITION_WORD: &str = "definition";
/// `help(x)`: a class's, an instance's or a function's description as text (card help-help)
const HELP_WORD: &str = "help";
const KEYS_WORD: &str = "keys";
const EXPORTS_WORD: &str = "exports";
/// The run-time choice of a class's names (Objects::dispatched)
const DISPATCH_TEMPLATE: &str = "if RECEIVER is CLASS then NAMES else OTHERWISE";
const DIR_OTHERWISE_TEMPLATE: &str = "if RECEIVER is map then RECEIVER.keys else OTHERWISE";
pub(crate) const PARAMS_WORDS: [&str; 2] = ["params", "parameters"];
const SIGNATURE_WORD: &str = "signature";
/// `f.body`: the body as written, as data (card g_X_3s)
const BODY_WORD: &str = "body";
/// `f.dump`: the whole definition as warp text, to copy and paste (card reflection-area)
const DUMP_WORD: &str = "dump";
/// The words whose bare argument naming a function without parameters is its call: `print now`, `type now`
pub(crate) const CALLING_WORDS: [&str; 2] = ["print", crate::type_tests::TYPE_WORD];
/// `f.name`, `f.return_type`: a function object's name and the kind of its result (card module-methods)
const NAME_WORD: &str = "name";
const RETURN_TYPE_WORD: &str = "return_type";
/// `function f`: the function f itself (P82)
const FUNCTION_WORD: &str = "function";
/// `f.doc`: the function's doc string or the comment above it, empty without (doc_strings.rs)
const DOC_WORD: &str = "doc";
/// `f.full`: what `print f` shows, the doc as comment lines above the definition (user, card module-methods)
const FULL_WORD: &str = "full";
/// Other names of a function's words (user, card module-methods): each read as its canonical word with a note
pub(crate) const FUNCTION_WORD_ALIASES: [(&str, &str); 3] =
	[("description", DOC_WORD), ("returns", RETURN_TYPE_WORD), ("representation", DUMP_WORD)];
/// What `dir(f)` lists of a function: the words reflection answers for it
const FUNCTION_WORDS: [&str; 4] = [PARAMS_WORDS[0], SIGNATURE_WORD, BODY_WORD, EFFECTS_WORD];
/// `dir(sqrt)`: a library word has no body to show
const LIBRARY_FUNCTION_WORDS: [&str; 2] = [PARAMS_WORDS[0], SIGNATURE_WORD];
/// The entries of the module's warp.meta section that name its functions and classes
pub const META_FUNCTIONS: &str = "functions";
pub const META_CLASSES: &str = "classes";

type Functions = std::collections::BTreeMap<String, crate::compiler::context::UserFunctionDef>;

pub fn lower(node: Node) -> Node {
	if node.mentions_any(&CALLING_WORDS) {
		let defined = crate::library_words::defined_names(&node);
		let functions = crate::analyzer::function_definitions(&node).user_functions;
		if let Err(error) = crate::errors::diagnostic::report(&called_functions(&node, &functions, &defined)) {
			return error;
		}
	}
	let run_time_words: Vec<&str> = crate::wasm_emitter::functions::function_words().map(|(word, _)| word).collect();
	if !node.mentions_any(&run_time_words)
		&& !node.mentions_any(&[
			EFFECTS_WORD,
			LISTENERS_WORD,
			SIGNATURE_WORD,
			BODY_WORD,
			DUMP_WORD,
			DOC_WORD,
			FULL_WORD,
			FUNCTION_WORD_ALIASES[0].0,
			FUNCTION_WORD_ALIASES[1].0,
			FUNCTION_WORD_ALIASES[2].0,
			NAME_WORD,
			RETURN_TYPE_WORD,
			PARAMS_WORDS[0],
			PARAMS_WORDS[1],
			DIR_WORD,
			HELP_WORD,
			crate::type_tests::TYPE_WORD,
		]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let mut listened = HashSet::new();
	node.visit(&mut |child| {
		if let Some(name) = listened_name(child) {
			listened.insert(name);
		}
	});
	if let Some(name) = unlistened_reflection(&node, &listened, &defined) {
		return crate::node::error(&format!(
			"nothing listens to {name}: `listeners of {name}` needs an `on {name} {{…}}` handler or a variable {name}"
		));
	}
	let functions = functions_and_lambdas(&node);
	// `h(x) := g(x)` of a bool function g: the signatures need the bool functions known (card signature-int)
	crate::analyzer::note_bool_functions(functions.values());
	as_reflection_words(node.clone(), &functions, &defined)
}

/// `print hi` of a function hi without parameters calls it (decision reflection: a bare name never becomes the
/// function value): a warning saying so, with `function hi` for the function itself
fn called_functions(
	node: &Node,
	functions: &Functions,
	defined: &HashSet<String>,
) -> Vec<crate::errors::diagnostic::Diagnostic> {
	let mut warnings = vec![];
	node.visit(&mut |part| {
		let Node::List(items, Bracket::Round | Bracket::None, _) = part.drop_meta() else { return };
		let [word, subject] = items.as_slice() else { return };
		let (Some(word), Some(name)) = (word.symbol_name(), subject.symbol_name()) else { return };
		if !CALLING_WORDS.contains(&word)
			|| defined.contains(word)
			|| crate::closures::referenced_function(subject).is_some()
			|| !is_parameterless(name, functions, defined)
		{
			return;
		}
		let written = format!("{word} {name}");
		let warning = crate::errors::diagnostic::Diagnostic::at(
			part,
			format!("`{written}` calls {name}, a function without parameters; write `{word} function {name}` for the function itself"),
		)
		.offer("the function itself", &written, format!("{word} function {name}"));
		if !crate::errors::diagnostic::warned_before(&warning) {
			warnings.push(warning);
		}
	});
	warnings
}

fn is_parameterless(name: &str, functions: &Functions, defined: &HashSet<String>) -> bool {
	match functions.get(name) {
		Some(function) => function.params.is_empty(),
		None => library_signature(name, functions, defined).is_some_and(|(params, _)| params.is_empty()),
	}
}

/// The name an `on` or `once` statement listens to
fn listened_name(node: &Node) -> Option<String> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	// `h = on alarm {…}` arrives grouped `on (alarm {…})`
	let words: Vec<&str> = items
		.iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(parts, Bracket::None, _) => parts.iter().collect(),
			_ => vec![item],
		})
		.map(|item| item.symbol_name().unwrap_or(""))
		.collect();
	match words.as_slice() {
		[listening, event, variable, ..]
			if LISTENING_WORDS.contains(listening) && VARIABLE_EVENTS.contains(event) && !variable.is_empty() =>
		{
			Some(variable.to_string())
		}
		[listening, event, ..] if LISTENING_WORDS.contains(listening) && !event.is_empty() => Some(event.to_string()),
		_ => None,
	}
}

/// A main-level `listeners of x` (`x.listeners`) whose x is neither listened to nor a variable (a function's parameter
/// may be)
fn unlistened_reflection(node: &Node, listened: &HashSet<String>, defined: &HashSet<String>) -> Option<String> {
	if let Node::Key(_, Op::Define, _) = node.drop_meta() {
		return None;
	}
	let unlistened = crate::signal_values::listeners_subject(node)
		.and_then(Node::symbol_name)
		.filter(|name| !listened.contains(*name) && !defined.contains(*name));
	match unlistened {
		Some(name) => Some(name.to_string()),
		None => node.children().iter().find_map(|child| unlistened_reflection(child, listened, defined)),
	}
}

/// A function's `f.params`, `f.signature` and `f.body` are constants read off its definition, as are `dir(f)` and `type(f)`: the
/// reflection words take the function itself, `f` as `&f`, never its call (P83, card dir-function)
fn as_reflection_words(node: Node, functions: &Functions, defined: &HashSet<String>) -> Node {
	if let Some(reflected) = function_word(&node, functions, defined) {
		return reflected;
	}
	if let Node::Key(subject, Op::Dot, word) = node.drop_meta() {
		let subject = super::ambiguous_forms::written_word(subject);
		if let (Some(name), Some(word)) = (function_named(&subject), word.symbol_name()) {
			let function = functions.get(name).filter(|_| !defined.contains(word));
			let library = || library_signature(name, functions, defined).filter(|_| !defined.contains(word));
			let word = match canonical_word(word) {
				Some(canonical) if function.is_some() || library().is_some() => {
					crate::normalize::set_position_of(&node);
					crate::errors::diagnostic::note_alias(word, canonical);
					canonical
				}
				_ => word,
			};
			let reflected = match (word, function) {
				(word, Some(function)) if PARAMS_WORDS.contains(&word) => {
					Some(text_list(&function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>()))
				}
				(SIGNATURE_WORD, Some(function)) => Some(Node::Text(signature(function))),
				(DUMP_WORD, Some(function)) => Some(Node::Text(definition(function))),
				(DOC_WORD, Some(function)) => Some(Node::Text(function.doc.clone().unwrap_or_default())),
				(FULL_WORD, Some(function)) => Some(Node::Text(printed(function))),
				(NAME_WORD, Some(function)) => Some(Node::Text(function.name.clone())),
				(RETURN_TYPE_WORD, Some(function)) => Some(Node::Text(result_type(function))),
				(BODY_WORD, Some(function)) => Some(crate::blocks::quoted_code(function.body.as_ref().clone())),
				(word, None) if PARAMS_WORDS.contains(&word) => library().map(|(params, _)| text_list(&params)),
				(SIGNATURE_WORD, None) => library().map(|(_, signature)| Node::Text(signature)),
				(NAME_WORD, None) => library().map(|_| Node::Text(name.to_string())),
				(RETURN_TYPE_WORD, None) => library().map(|(_, signature)| Node::Text(return_type(&signature))),
				// `use reflection`: what only the emitter knows (wasm index) is read off the function value at run time
				(word, Some(_)) if is_run_time_word(word) => {
					Some(key(crate::closures::function_reference(name.to_string()), Op::Dot, symbol(word)))
				}
				_ => None,
			};
			if let Some(reflected) = reflected {
				return reflected;
			}
		}
	}
	node.map_children(|child| as_reflection_words(child, functions, defined))
}

/// A word a function value answers at run time only: the reflection mode's (function_words)
fn is_run_time_word(word: &str) -> bool {
	crate::wasm_emitter::functions::function_words().any(|(answered, _)| answered == word)
		&& ![NAME_WORD, DOC_WORD, FULL_WORD, SIGNATURE_WORD, RETURN_TYPE_WORD, DUMP_WORD].contains(&word)
}

/// The canonical word of an alias of a function's word: `doc` of `description`
pub(crate) fn canonical_word(word: &str) -> Option<&'static str> {
	FUNCTION_WORD_ALIASES.into_iter().find(|(alias, _)| *alias == word).map(|(_, canonical)| canonical)
}

/// `print f`, `f.full`: the definition, the doc as comment lines above it (decision reflection)
pub(crate) fn printed(function: &crate::compiler::context::UserFunctionDef) -> String {
	let definition = definition(function);
	match &function.doc {
		Some(doc) => format!("# {}\n{definition}", doc.replace('\n', "\n# ")),
		None => definition,
	}
}

/// `f`, `function f`, `(function f)`: the name of the function the reflection words look at
fn function_named(subject: &Node) -> Option<&str> {
	if let Some(name) = subject.symbol_name() {
		return Some(name);
	}
	match subject.drop_meta() {
		// `(function f)`; `(f)` is the call `f()`, whose result answers (card name-function)
		Node::List(items, Bracket::Round, _)
			if items.len() == 1
				&& (items[0].symbol_name().is_none() || crate::closures::referenced_function(&items[0]).is_some()) =>
		{
			function_named(&items[0])
		}
		Node::List(items, Bracket::Round | Bracket::None, _) => match items.as_slice() {
			[word, name] if word.symbol_name() == Some(FUNCTION_WORD) => name.symbol_name(),
			_ => None,
		},
		_ => None,
	}
}

/// `type(gcd)`: `function: (a, b) -> int`, the kind and the signature (user decision on card module-methods)
pub(crate) fn function_type(signature: &str) -> Node {
	Node::Text(format!("{}: {signature}", crate::type_kinds::Kind::Function))
}

/// `type(function floor)`: `builtin: (x:number) -> int` (decision dir-lists)
pub(crate) fn builtin_type(signature: &str) -> Node {
	Node::Text(format!("{}: {signature}", crate::type_tests::BUILTIN_TYPE))
}

/// `(a, b) -> int` → `int`
fn return_type(signature: &str) -> String {
	signature.rsplit_once(" -> ").map_or("", |(_, result)| result).to_string()
}

/// A library word's parameter names and signature, unless the program defines that name itself
fn library_signature(name: &str, functions: &Functions, defined: &HashSet<String>) -> Option<(Vec<String>, String)> {
	if functions.contains_key(name) || defined.contains(name) {
		return None;
	}
	super::library_signatures::library_signature(name)
}

/// `dir(f)`: the reflection words of a user function; `type(f)`: function. A program's own dir or type wins
fn function_word(node: &Node, functions: &Functions, defined: &HashSet<String>) -> Option<Node> {
	let Node::List(items, Bracket::Round | Bracket::None, _) = node.drop_meta() else {
		return None;
	};
	let [word, subject] = items.as_slice() else { return None };
	let subject = super::ambiguous_forms::written_word(subject);
	let (word, name) = (word.symbol_name()?, subject.symbol_name()?);
	if defined.contains(word) {
		return None;
	}
	// `type a` of a function a without parameters: the type of its result (called_functions warns)
	let referenced = crate::closures::referenced_function(&items[1]).is_some();
	if word == crate::type_tests::TYPE_WORD && !referenced && is_parameterless(name, functions, defined) {
		return Some(call(word, vec![call(name, vec![])]));
	}
	// `type(number)`, `dir(map)`: a type word is the type, `function map` the method
	if crate::type_tests::is_builtin_type_word(name) && !referenced {
		return None;
	}
	if !functions.contains_key(name) {
		let (_, signature) = library_signature(name, functions, defined)?;
		return match word {
			DIR_WORD => Some(text_list(&LIBRARY_FUNCTION_WORDS.map(String::from))),
			HELP_WORD => Some(Node::Text(format!("{name}{signature}"))),
			crate::type_tests::TYPE_WORD => Some(builtin_type(&signature)),
			_ => None,
		};
	}
	match word {
		DIR_WORD => Some(text_list(&FUNCTION_WORDS.map(String::from))),
		crate::type_tests::TYPE_WORD => Some(function_type(&signature(&functions[name]))),
		HELP_WORD => Some(Node::Text(help_text(&functions[name]))),
		_ => None,
	}
}

/// The warp.meta entries of a program (card reflection-foreign-meta, meta_section.rs): `functions` {f: {params: ["a"]
/// signature: "(a:int) -> int"}}, read off the lowered program (every definition form is one by then, the kinds are
/// inferred), and `classes` {P: {fields: [x] methods: [sum] definition: "class P{…}"}} off the source, inherited
/// fields first; absent when the program defines none. Names are symbols: a one-letter text reads back as a character
pub fn meta_entries(source: &Node, lowered: &Node, code: &str) -> Vec<(&'static str, Node)> {
	let objects = Objects { layouts: crate::class_methods::class_layouts(source), ..Objects::default() };
	let mut classes: Vec<&String> = objects.layouts.keys().collect();
	classes.sort();
	// a class-body method travels in its class's definition; one defined outside (`width(p:P)`) stays a function
	let declared = crate::class_methods::declared_layouts(source);
	let is_method = |name: &String| declared.values().any(|layout| layout.methods.contains(name));
	// the program's own functions, not the library's its lowering brought in (html_render, prelude·replace)
	let own = user_functions(source);
	let functions: Vec<(String, Node)> = user_functions(lowered)
		.values()
		.filter(|function| own.contains_key(&function.name) && !is_method(&function.name))
		.map(|function| {
			let params = function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>();
			(
				function.name.clone(),
				meta_map(vec![
					(PARAMS_WORDS[0], text_list(&params)),
					(SIGNATURE_WORD, Node::Text(signature(function))),
				]),
			)
		})
		.collect();
	let classes: Vec<(String, Node)> = classes
		.into_iter()
		.map(|class| {
			(
				class.clone(),
				meta_map(vec![
					(FIELDS_WORDS[0], text_list(&objects.fields(class))),
					(METHODS_WORD, text_list(&objects.methods(class))),
					(DEFINITION_WORD, class_definition(code, class).unwrap_or(Node::Empty)),
				]),
			)
		})
		.collect();
	[(META_FUNCTIONS, functions), (META_CLASSES, classes)]
		.into_iter()
		.filter(|(_, entries)| !entries.is_empty())
		.map(|(entry, entries)| {
			(entry, meta_map(entries.iter().map(|(name, value)| (name.as_str(), value.clone())).collect()))
		})
		.collect()
}

/// A class's definition as written in the code: from its keyword (`class P`, `struct P`) to its balanced closing brace
/// (a parsed class does not serialize back to its source)
fn class_definition(code: &str, class: &str) -> Option<Node> {
	let keywords = crate::warp_parser::TYPE_DECLARATION_WORDS.iter().chain(CLASS_WORDS.iter());
	let starts = keywords.flat_map(|keyword| {
		code.match_indices(&format!("{keyword} {class}")).map(|(at, written)| (at, written.len())).collect::<Vec<_>>()
	});
	let (start, _) = starts
		.filter(|(at, length)| !code[at + length..].starts_with(|next: char| next.is_alphanumeric() || next == '_'))
		.min()?;
	balanced_end(&code[start..]).map(|end| text(&code[start..start + end]))
}

/// The length of the text up to the brace closing its first `{`, quoted text skipped
fn balanced_end(text: &str) -> Option<usize> {
	let (mut depth, mut quoted, mut escaped) = (0, false, false);
	for (index, character) in text.char_indices() {
		match character {
			_ if escaped => escaped = false,
			'\\' if quoted => escaped = true,
			'"' => quoted = !quoted,
			'{' if !quoted => depth += 1,
			'}' if !quoted => {
				depth -= 1;
				if depth == 0 {
					return Some(index + 1);
				}
			}
			_ => {}
		}
	}
	None
}

pub(super) fn user_functions(program: &Node) -> Functions {
	let mut context = crate::compiler::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, program);
	context.user_functions
}

/// The program's functions, those `f = x => x*2` defines too (lambdas.rs makes it `f(x) := x*2` after this pass)
fn functions_and_lambdas(program: &Node) -> Functions {
	let mut lambdas = vec![];
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Some(name) = target.symbol_name() {
				lambdas.extend(crate::lambdas::lambda_definition(name, value));
			}
		}
	});
	let mut functions = user_functions(&Node::List(lambdas, Bracket::None, Separator::Semicolon));
	functions.extend(user_functions(program));
	functions
}

/// `{key: value …}`
fn meta_map(entries: Vec<(&str, Node)>) -> Node {
	Node::List(
		entries.into_iter().map(|(name, value)| key(symbol(name), Op::Colon, value)).collect(),
		Bracket::Curly,
		Separator::Space,
	)
}

/// `area(width:int, height:int) := width * height`: the parameters as declared (no inferred types), the body as written
pub(crate) fn definition(function: &crate::compiler::context::UserFunctionDef) -> String {
	let params: Vec<String> = function
		.params
		.iter()
		.map(|param| {
			let typed = param
				.annotation
				.as_ref()
				.map_or_else(|| param.name.clone(), |annotation| format!("{}: {}", param.name, annotation.serialize()));
			param.default.as_ref().map_or(typed.clone(), |default| format!("{typed} = {}", default.serialize()))
		})
		.collect();
	format!("{}({}) := {}", function.name, params.join(", "), function.body.serialize())
}

/// `help(area)`: `area(width:int, height:int) -> int`, its doc on the lines below
pub(crate) fn help_text(function: &crate::compiler::context::UserFunctionDef) -> String {
	let head = format!("{}{}", function.name, signature(function));
	match &function.doc {
		Some(doc) => format!("{head}\n{doc}"),
		None => head,
	}
}

/// `(a: int, b: int) -> int`: each parameter with its declared or demanded type (spaced as warp format writes it), the
/// result's kind when known
pub(crate) fn signature(function: &crate::compiler::context::UserFunctionDef) -> String {
	let params: Vec<String> = function
		.params
		.iter()
		.map(|param| {
			let type_name = param
				.annotation
				.as_ref()
				.map(|annotation| annotation.drop_meta().name())
				.or_else(|| param.used_as.map(|kind| kind.to_string()));
			type_name.map_or_else(|| param.name.clone(), |type_name| format!("{}: {type_name}", param.name))
		})
		.collect();
	let result = match function.return_kind {
		crate::type_kinds::Kind::Empty => String::new(),
		_ => format!(" -> {}", result_type(function)),
	};
	format!("({}){result}", params.join(", "))
}

/// `f.return_type`: bool for a bool function, held as an Int (card bool-type), else the kind it returns
pub(crate) fn result_type(function: &crate::compiler::context::UserFunctionDef) -> String {
	match function.return_kind {
		crate::type_kinds::Kind::Int if crate::analyzer::is_bool_function(&function.name) => {
			crate::analyzer::BOOL_TYPE.to_string()
		}
		kind => kind.to_string(),
	}
}

/// What the compile-time object words know: each class's layout, the classes of instance variables, the keys of map
/// literals' variables, the exports of the imported core modules by their alias
#[derive(Default)]
struct Objects {
	layouts: HashMap<String, ClassLayout>,
	instances: HashMap<String, String>,
	maps: HashMap<String, Vec<String>>,
	modules: HashMap<String, Vec<String>>,
	/// the `functions` and `classes` entries of an imported module's warp.meta section, by its alias
	module_functions: HashMap<String, Node>,
	module_classes: HashMap<String, Node>,
	defined: HashSet<String>,
	/// what is known of expressions that are no variable: `teams#1` of `teams: [Team]`
	shapes: Option<crate::traits::InstanceTypes>,
	/// the type module of each variable given a text or list literal: `s = "hi"` → text
	literal_types: HashMap<String, &'static str>,
}

pub fn lower_objects(node: Node) -> Node {
	let words: Vec<&str> =
		CLASS_WORDS.iter().chain(&FIELDS_WORDS).chain(&METHODS_WORDS).chain(&[DIR_WORD, HELP_WORD]).copied().collect();
	if !node.mentions_any(&words) {
		return node;
	}
	let layouts = crate::class_methods::class_layouts(&node);
	let classes: Vec<String> = layouts.keys().cloned().collect();
	let objects = Objects {
		instances: crate::class_methods::instance_classes(&node, &classes),
		maps: map_keys(&node),
		literal_types: literal_types(&node),
		defined: crate::library_words::defined_names(&node),
		layouts,
		shapes: Some(crate::traits::InstanceTypes::of(&node)),
		..Objects::default()
	};
	with_object_words(node, &objects)
}

/// `m.exports`, `dir(m)` of an imported core module m, once modules::resolve has found its file; of a module compiled
/// from warp also `m.f.params`, `m.f.signature` of its functions and `m.P.fields`, `m.P.methods`, `dir(m.P)` of its classes
pub fn lower_module_words(node: Node) -> Node {
	let words: Vec<&str> = PARAMS_WORDS
		.iter()
		.chain(&FIELDS_WORDS)
		.chain(&METHODS_WORDS)
		.chain(&[EXPORTS_WORD, DIR_WORD, SIGNATURE_WORD])
		.copied()
		.collect();
	if !node.mentions_any(&words) {
		return node;
	}
	let objects = module_objects(&node);
	if objects.modules.is_empty() {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	with_object_words(node, &Objects { defined, ..objects })
}

/// `calc.exports`, `dir(calc)` of a component (`use wasm "calc.wasm" as calc`, card reflection-components): its export
/// names, read off the component natively before foreign_modules makes `calc.exports` a call into it; the page reads
/// components only at run time (components.js)
pub fn lower_component_words(node: Node) -> Node {
	if !node.mentions_any(&[EXPORTS_WORD, DIR_WORD]) {
		return node;
	}
	let modules = component_exports(&node);
	if modules.is_empty() {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	with_object_words(node, &Objects { modules, defined, ..Objects::default() })
}

fn component_exports(node: &Node) -> HashMap<String, Vec<String>> {
	crate::foreign_modules::component_modules(node)
		.into_iter()
		.filter_map(|(alias, path)| match crate::modules::wasm_modules::component_export_names(&path) {
			Ok(names) => Some((alias, names)),
			Err(failure) => {
				eprintln!("[wasm] {failure}");
				None
			}
		})
		.collect()
}

/// `import lib/fourty_two`: the names fourty_two exports (modules/wasm_modules/), sorted; not the setters derived for its globals.
/// A module warp compiled names its own functions in warp.meta: its exports are those (not warp's runtime exports), and
/// that entry is kept by the module's alias for `m.f.params`, its `classes` entry for `m.P.fields`
fn module_objects(node: &Node) -> Objects {
	let mut context = crate::compiler::context::Context::new();
	crate::analyzer::extract_ffi_imports(&mut context, node);
	let paths: HashSet<&str> = context
		.ffi_imports
		.values()
		.map(|import| import.library)
		.filter(|library| crate::modules::wasm_modules::is_module_path(library))
		.collect();
	let mut objects = Objects::default();
	for path in paths {
		let alias = crate::modules::wasm_modules::module_alias(path);
		let bytes = crate::modules::wasm_modules::module_bytes(path).ok();
		let entry = |key: &str| bytes.as_ref().and_then(|bytes| crate::formats::meta_section::entry(bytes, key));
		let functions = entry(META_FUNCTIONS);
		let is_own = |name: &String| {
			functions.as_ref().is_none_or(|functions| !matches!(functions[name.as_str()].drop_meta(), Node::Empty))
		};
		let mut names: Vec<String> = crate::modules::wasm_modules::exports(path)
			.keys()
			.filter(|name| !name.contains(' ') && is_own(name))
			.cloned()
			.collect();
		names.sort();
		objects.module_classes.extend(entry(META_CLASSES).map(|classes| (alias.clone(), classes)));
		objects.module_functions.extend(functions.map(|functions| (alias.clone(), functions)));
		objects.modules.insert(alias, names);
	}
	objects
}

/// What `dir` lists of a variable known at compile time (members.rs): an instance's fields and methods, a map's keys
pub(super) fn instance_names(program: &Node, variable: &str) -> Option<Vec<String>> {
	let layouts = crate::class_methods::class_layouts(program);
	let classes: Vec<String> = layouts.keys().cloned().collect();
	let objects =
		Objects { instances: crate::class_methods::instance_classes(program, &classes), layouts, ..Objects::default() };
	objects.class_of(variable).and_then(|class| objects.listed(class, DIR_WORD))
}

pub(super) fn map_keys_of(program: &Node, variable: &str) -> Option<Vec<String>> {
	map_keys(program).remove(variable)
}

/// `m = {a:1, b:2}`: m's keys as written
fn map_keys(node: &Node) -> HashMap<String, Vec<String>> {
	let mut maps = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let (Some(name), Node::List(items, Bracket::Curly, _)) = (target.symbol_name(), value.drop_meta()) {
				let keys: Option<Vec<String>> = items
					.iter()
					.map(|item| match item.drop_meta() {
						Node::Key(key, Op::Colon, _) => key.symbol_name().map(String::from),
						_ => None,
					})
					.collect();
				if let Some(keys) = keys.filter(|keys| !keys.is_empty()) {
					maps.insert(name.to_string(), keys);
				}
			}
		}
	});
	maps
}

impl Objects {
	/// `m.f.params`, `m.f.signature` of an imported module's function, `m.P.fields`, `m.P.methods`, `dir(m.P)` of its
	/// class, from the module's warp.meta section; modules.rs has made `m.f` the qualified name of the import
	/// (wasm_modules::qualified)
	fn module_member_word(&self, subject: &Node, word: &str) -> Option<Node> {
		let (module, member) = match subject.drop_meta() {
			Node::Key(module, Op::Dot, member) => (module.symbol_name()?, member.symbol_name()?),
			qualified => qualified.symbol_name()?.split_once('.')?,
		};
		let class =
			self.module_classes.get(module).map(|classes| &classes[member]).filter(|class| !class.is_empty_node());
		if let Some(class) = class {
			return listed_names(entry_names(&class[FIELDS_WORDS[0]]), entry_names(&class[METHODS_WORD]), word)
				.map(|names| text_list(&names));
		}
		let word = if PARAMS_WORDS.contains(&word) { PARAMS_WORDS[0] } else { word };
		match self.module_functions.get(module)?[member][word].drop_meta() {
			Node::Empty => None,
			names if word == PARAMS_WORDS[0] => Some(text_list(&entry_names(names))),
			value => Some(value.clone()),
		}
	}

	/// The class an instance variable holds, or the class a name is
	fn class_of<'a>(&'a self, name: &'a str) -> Option<&'a str> {
		self.instances.get(name).map(String::as_str).or_else(|| self.layouts.contains_key(name).then_some(name))
	}

	/// Inherited names first, a name the class redeclares kept in its parent's place
	fn names(&self, class: &str, own: fn(&ClassLayout) -> &Vec<String>) -> Vec<String> {
		let Some(layout) = self.layouts.get(class) else {
			return vec![];
		};
		let mut names = layout.parent.as_deref().map(|parent| self.names(parent, own)).unwrap_or_default();
		names.extend(own(layout).iter().filter(|name| !names.contains(name)).cloned().collect::<Vec<_>>());
		names
	}

	fn fields(&self, class: &str) -> Vec<String> {
		self.names(class, |layout| &layout.fields)
	}

	fn methods(&self, class: &str) -> Vec<String> {
		self.names(class, |layout| &layout.methods)
	}

	/// `subject.word` as a reflection word, unless subject has a real field or key of that name
	fn dot_word(&self, subject: &Node, word: &str) -> Option<Node> {
		if self.defined.contains(word) {
			return None;
		}
		let value_methods = || METHODS_WORDS.contains(&word).then(|| self.value_methods(subject)).flatten();
		let written = || value_methods().unwrap_or_else(|| key(subject.clone(), Op::Dot, symbol(word)));
		if let Some(reflected) = self.module_member_word(subject, word) {
			return Some(reflected);
		}
		if self.known_class(subject).is_some_and(|class| self.has_member(&class, word)) {
			return None;
		}
		let Some(name) = subject.symbol_name() else {
			// `P(3).methods`: a construction's class is known, no type test needed
			let constructed = self.known_class(subject).or_else(|| self.constructed_class(subject));
			if let Some(class) = constructed.filter(|_| !CLASS_WORDS.contains(&word)) {
				if self.has_member(&class, word) {
					return None;
				}
				return self.listed(&class, word).map(|names| word_list(word, &class, &names));
			}
			return self.dispatched(subject, word, written()).or_else(value_methods);
		};
		if let Some(class) = self.class_of(name) {
			let is_instance = self.instances.contains_key(name);
			return match word {
				_ if self.has_member(class, word) => None,
				_ if CLASS_WORDS.contains(&word) && is_instance => {
					Some(call(crate::type_tests::TYPE_WORD, vec![subject.clone()]))
				}
				_ => self.listed(class, word).map(|names| word_list(word, class, &names)),
			};
		}
		if let Some(exports) = self.modules.get(name) {
			return (word == EXPORTS_WORD).then(|| text_list(exports));
		}
		match self.maps.get(name) {
			Some(keys) => {
				(FIELDS_WORDS.contains(&word) && !keys.iter().any(|key| key == word)).then(|| self.map_keys(subject))
			}
			None => self.dispatched(subject, word, written()).or_else(value_methods),
		}
	}

	/// `dir(x)`: the fields and methods of an instance or class, a module's exports, the keys of a map
	fn dir(&self, subject: &Node) -> Option<Node> {
		if self.defined.contains(DIR_WORD) {
			return None;
		}
		if let Some(reflected) = self.module_member_word(subject, DIR_WORD) {
			return Some(reflected);
		}
		// no class's instance at run time: a map's keys, else no names
		let Some(name) = subject.symbol_name() else {
			return self.dispatched(subject, DIR_WORD, dir_otherwise(subject));
		};
		match (self.class_of(name), self.modules.get(name)) {
			(Some(class), _) => self.listed(class, DIR_WORD).map(|names| text_list(&names)),
			(None, Some(exports)) => Some(text_list(exports)),
			(None, None) if self.maps.contains_key(name) => Some(self.map_keys(subject)),
			(None, None) => self.dispatched(subject, DIR_WORD, dir_otherwise(subject)),
		}
	}

	/// `help(P)`, `help(p)`: the class, its parent, fields and methods, one per line (an empty line left out)
	fn help(&self, subject: &Node) -> Option<Node> {
		let class = self.class_of(subject.symbol_name()?).filter(|_| !self.defined.contains(HELP_WORD))?;
		let parent = self.layouts[class]
			.parent
			.as_ref()
			.map(|parent| format!(" {} {parent}", crate::warp_parser::EXTENDS_KEYWORD))
			.unwrap_or_default();
		let members = [(FIELDS_WORDS[0], self.fields(class)), (METHODS_WORD, self.methods(class))];
		let lines = members
			.into_iter()
			.filter(|(_, names)| !names.is_empty())
			.map(|(word, names)| format!("{word}: {}", names.join(" ")));
		Some(Node::Text(
			std::iter::once(format!("{} {class}{parent}", CLASS_WORDS[0])).chain(lines).collect::<Vec<_>>().join("\n"),
		))
	}

	/// The names a reflection word lists for a class: its fields, its methods, or both for `dir`
	fn listed(&self, class: &str, word: &str) -> Option<Vec<String>> {
		listed_names(self.fields(class), self.methods(class), word)
	}

	fn has_member(&self, class: &str, word: &str) -> bool {
		self.fields(class).iter().chain(&self.methods(class)).any(|name| name == word)
	}

	/// The class of the instance an expression holds, when known at compile time
	fn known_class(&self, subject: &Node) -> Option<String> {
		match self.shapes.as_ref()?.shape(subject)? {
			crate::traits::Shape::Instance(class) => Some(class),
			crate::traits::Shape::ListOf(_) => None,
		}
	}

	/// `P(3)`, a call of one of the program's classes: its class
	fn constructed_class(&self, subject: &Node) -> Option<String> {
		let Node::List(items, Bracket::Round, _) = subject.drop_meta() else {
			return None;
		};
		let class = items.first()?.symbol_name()?;
		self.layouts.contains_key(class).then(|| class.to_string())
	}

	/// A subject whose class is known only at run time (`f(o) := o.fields`, `make().class`): a type test over the
	/// program's classes picks the names (or the class for `class`/`type`), a class's own member of that name is read
	/// as written, anything else stays `written`
	fn dispatched(&self, subject: &Node, word: &str, written: Node) -> Option<Node> {
		let names_class = CLASS_WORDS.contains(&word);
		// a class without a layout lists nothing: whether the word lists names at all
		if self.layouts.is_empty() || (!names_class && self.listed("", word).is_none()) {
			return None;
		}
		let mut classes: Vec<&String> = self.layouts.keys().collect();
		classes.sort();
		Some(classes.into_iter().rev().fold(written.clone(), |otherwise, class| {
			let names = match (self.has_member(class, word), names_class) {
				(true, _) => written.clone(),
				(false, true) => call(crate::type_tests::TYPE_WORD, vec![subject.clone()]),
				(false, false) => word_list(word, class, &self.listed(class, word).unwrap_or_default()),
			};
			type_dispatch(subject, class, names, otherwise)
		}))
	}

	/// `methods("hi")`, `xs.methods` of a value that is no instance of the program's classes: the words of the module
	/// named after its type, the functions themselves (`methods([1 2])` lists the list module's). Only a type known
	/// before the run: a parameter's `o.methods` or an imported class's `shapes.P.methods` stays as written
	fn value_methods(&self, subject: &Node) -> Option<Node> {
		let known = literal_type(subject).or_else(|| self.literal_types.get(subject.symbol_name()?).copied())?;
		let (module, words) = crate::modules::type_modules().into_iter().find(|(module, _)| *module == known)?;
		Some(function_list(&super::members::type_dir_names(module, words)))
	}

	fn map_keys(&self, subject: &Node) -> Node {
		key(subject.clone(), Op::Dot, symbol(KEYS_WORD))
	}
}

/// `dir(x)` no earlier pass answered (no module, class, map, function or task it knows): a map's keys at run time, else
/// no names, so `dir pi` answers instead of being "undefined: dir" (card error-undefined-dir)
pub fn lower_unknown_dir(node: Node) -> Node {
	if !node.mentions_any(&[DIR_WORD]) || crate::library_words::defined_names(&node).contains(DIR_WORD) {
		return node;
	}
	answered_dir(node)
}

fn answered_dir(node: Node) -> Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _)
			if items.len() == 2 && items[0].symbol_name() == Some(DIR_WORD) =>
		{
			dir_otherwise(&answered_dir(items[1].clone()))
		}
		_ => node.map_children(answered_dir),
	}
}

/// `if subject is class then names else otherwise`
fn type_dispatch(subject: &Node, class: &str, names: Node, otherwise: Node) -> Node {
	super::nodes::filled(
		DISPATCH_TEMPLATE,
		[
			("RECEIVER", subject.clone()),
			("CLASS", Node::Symbol(class.to_string())),
			("NAMES", names),
			("OTHERWISE", otherwise),
		],
	)
}

/// What `dir` lists of a value that is no instance of the program's classes: a map's keys, else its type's methods
/// as Python's dir([1, 2]) does, the words of the module named after the type (`dir([1 2])` == `dir(list)`), else none
fn dir_otherwise(subject: &Node) -> Node {
	let methods = crate::modules::type_modules().into_iter().rev().filter(|(module, _)| *module != MAP_WORD).fold(
		text_list(&[]),
		|otherwise, (module, words)| {
			type_dispatch(subject, module, text_list(&super::members::type_dir_names(module, words)), otherwise)
		},
	);
	super::nodes::filled(DIR_OTHERWISE_TEMPLATE, [("RECEIVER", subject.clone()), ("OTHERWISE", methods)])
}

/// The type modules whose words `methods([1 2])` or `xs.methods` of a list value lists as functions: the program uses
/// them, so the functions are there, as for `methods(list)` (card methods-last-n). Not yet the text module: all of
/// its functions together don't compile (card methods-text), so `methods("hi")` still lists their names
pub(crate) fn value_methods_modules(program: &Node) -> Vec<&'static str> {
	if !program.mentions_any(&[METHODS_WORD]) {
		return vec![];
	}
	let variables = literal_types(program);
	let mut modules = vec![];
	program.visit(&mut |part| {
		let subject = match part.drop_meta() {
			Node::List(items, Bracket::Round | Bracket::None, _)
				if items.len() == 2 && items[0].symbol_name() == Some(METHODS_WORD) =>
			{
				&items[1]
			}
			Node::Key(subject, Op::Dot, word) if word.symbol_name() == Some(METHODS_WORD) => subject.as_ref(),
			_ => return,
		};
		modules.extend(literal_type(subject).or_else(|| variables.get(subject.symbol_name()?).copied()));
	});
	modules
}

/// The type module of a text or list literal
fn literal_type(value: &Node) -> Option<&'static str> {
	match value.drop_meta() {
		Node::Text(_) => Some(crate::words::TEXT_WORD),
		Node::List(_, Bracket::Square, _) => Some(crate::words::LIST_WORD),
		_ => None,
	}
}

/// The variables a text or list literal is assigned to, with that literal's type module
fn literal_types(node: &Node) -> HashMap<String, &'static str> {
	let mut types = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define | Op::Colon, value) = part {
			if let (Some(name), Some(module)) = (target.symbol_name(), literal_type(value)) {
				types.insert(name.to_string(), module);
			}
		}
	});
	types
}

fn with_object_words(node: Node, objects: &Objects) -> Node {
	// `o.members = v`, `o.members.add(p)`: a field as a place, no reflection word (card orm-members)
	if let Node::Key(place, op, rest) = node.drop_meta() {
		if is_field_place(place, op, rest) {
			let Node::Key(subject, Op::Dot, word) = place.drop_meta() else { unreachable!("guarded") };
			let place =
				Node::Key(Box::new(with_object_words(subject.as_ref().clone(), objects)), Op::Dot, word.clone());
			return key(place, *op, with_object_words(rest.as_ref().clone(), objects));
		}
	}
	let reflected = match node.drop_meta() {
		Node::Key(subject, Op::Dot, word) => word.symbol_name().and_then(|word| objects.dot_word(subject, word)),
		Node::List(items, Bracket::Round | Bracket::None, _)
			if items.len() == 2 && items[0].symbol_name() == Some(DIR_WORD) =>
		{
			objects.dir(&items[1])
		}
		Node::List(items, Bracket::Round | Bracket::None, _)
			if items.len() == 2 && items[0].symbol_name() == Some(HELP_WORD) =>
		{
			objects.help(&items[1])
		}
		// `methods(P)` is `P.methods`, `fields(p)` is `p.fields`
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && is_listing_word(&items[0]) => {
			objects.dot_word(&items[1], &items[0].name())
		}
		_ => None,
	};
	reflected.unwrap_or_else(|| node.map_children(|child| with_object_words(child, objects)))
}

/// `o.f = v`, `o.f += v` and `o.f.add(v)` (a list-mutating method) change the field f of o
fn is_field_place(place: &Node, op: &Op, rest: &Node) -> bool {
	let mutates = match op {
		Op::Dot => {
			matches!(rest.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| crate::list_mutations::is_list_mutating_method(&word.drop_meta().name())))
		}
		_ => *op == Op::Assign || op.is_compound_assign(),
	};
	mutates && matches!(place.drop_meta(), Node::Key(_, Op::Dot, _))
}

/// `fields`, `methods` and their other spellings
fn is_listing_word(word: &Node) -> bool {
	word.symbol_name().is_some_and(|word| FIELDS_WORDS.contains(&word) || METHODS_WORDS.contains(&word))
}

/// What a reflection word lists of a class: its methods themselves, `function sq` for each (decision reflection,
/// card module-methods), its fields and dir's names as texts
fn word_list(word: &str, class: &str, names: &[String]) -> Node {
	match word {
		_ if METHODS_WORDS.contains(&word) => Node::List(
			names.iter().map(|name| crate::closures::method_reference(name.clone(), class)).collect(),
			Bracket::Square,
			Separator::Space,
		),
		_ => text_list(names),
	}
}

/// The functions of these names themselves, `[function gcd, function lcm]`
pub(crate) fn function_list(names: &[String]) -> Node {
	Node::List(
		names.iter().cloned().map(crate::closures::function_reference).collect(),
		Bracket::Square,
		Separator::Space,
	)
}

/// The names a reflection word lists for a class of these fields and methods: either, or both for `dir`
fn listed_names(fields: Vec<String>, methods: Vec<String>, word: &str) -> Option<Vec<String>> {
	match word {
		DIR_WORD => Some([fields, methods].concat()),
		_ if METHODS_WORDS.contains(&word) => Some(methods),
		_ if FIELDS_WORDS.contains(&word) => Some(fields),
		_ => None,
	}
}

/// Names as symbols, `[x y]`: a one-letter text would read back as a character
/// The names of a warp.meta list, `["x" "y"]`: a one-letter text reads back as a character
pub(crate) fn entry_names(list: &Node) -> Vec<String> {
	match list.drop_meta() {
		Node::List(items, _, _) => items.iter().map(Node::text_or_name).collect(),
		_ => vec![],
	}
}

pub(crate) fn text_list(names: &[String]) -> Node {
	Node::List(names.iter().map(|name| Node::Text(name.clone())).collect(), Bracket::Square, Separator::None)
}
