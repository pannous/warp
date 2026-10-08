//! The differential check between warp's type checks and W0, the Lean model of warp's type theory whose soundness is
//! proved (notes/type_theory.md, lean/WarpTypes). A warp program in W0's subset is exported as a list of Lean `Item`s
//! (Checker.lean: names, charged names, functions, statements in source order); the model elaborates and checks it
//! and answers `ok <type>` or `rejected`. Warp's own verdict is whether `pipeline::compile` accepts the program.
//! Contract: W0 rejects ⇒ warp rejects; every exception is a known hole with a card.
use super::lean::{lean_executable, run_with_timeout};
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::TRY_MARKER;
use crate::type_kinds::Kind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

mod nested_functions;
mod used_modules;

/// The lake project of the model, in the source tree
pub const MODEL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lean/WarpTypes");
/// `lake build` of one project must not run twice at once
static MODEL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
const ACCEPTED_PREFIX: &str = "ok ";
const REJECTED: &str = "rejected";
const CONSTANT_KEYWORD: &str = "const";
const ERROR_CALL: &str = "error";
const BOOL_TYPE: &str = ".bool";
const ANY_TYPE: &str = ".any";
/// The builtin scalar type words warp checks a value against (analyzer admits) that W0 has a type for
const BUILTIN_TYPE_WORDS: [&str; 11] = ["int", "integer", "long", "exact", "float", "number", "text", "string", "str", "bool", "boolean"];
/// A value of each W0 scalar type and the run-time kind warp sees it as (a bool is an Int)
const VALUE_KINDS: [(&str, Kind); 4] = [(BOOL_TYPE, Kind::Int), (".int", Kind::Int), (".number", Kind::Float), (".text", Kind::Text)];
const DEF_KEYWORD: &str = "def";
const UNIT_TYPE: &str = ".unit";
/// the parameter of a function that takes none
const UNIT_PARAMETER: &str = "·";
const ARGUMENTS_SUFFIX: &str = "·args";
const APPEND_METHODS: [&str; 2] = ["add", "push"];
const FOR_KEYWORD: &str = "for";
const VARIABLE_KEYWORDS: [&str; 2] = ["let", "shared"];
/// the one field of a function local's cell (`f·n`): `·` keeps it apart from the program's own fields
const CELL_FIELD: &str = "·value";
/// An inline union `int | text` or an optional `int?` is the join of its alternatives, every value given to it a cast
const UNION_TYPE: &str = "(Ty.joinAll ";
const UNION_JOINER: &str = " or ";
const OPTIONAL_MARK: char = '?';
/// the type of ø, the empty part of an optional (ø is the empty list)
const EMPTY_TYPE: &str = ".list .never";
/// Effect handlers (notes/effect_handlers.md): `on ev {h}`, `on ev {h} in {body}`, `emit ev{payload}`
const ON_KEYWORD: &str = "on";
const IN_KEYWORD: &str = "in";
const EMIT_KEYWORD: &str = "emit";
/// `global n = 0` at main level, `global n` in a function: a main-level name functions may assign
const GLOBAL_KEYWORD: &str = "global";
/// `break v` in a block handler ends its block with v (aborting handlers)
const BREAK_KEYWORD: &str = "break";
/// `return v` before a function's end is the event `f·return` whose block handler, around f's body, breaks with v
const RETURN_KEYWORD: &str = "return";
const RETURN_EVENT_SUFFIX: &str = "·return";
/// the handler's local holding the payload, `event.level`
const EVENT_LOCAL: &str = "event";
/// an event's payloads are instances of the class `ev·event`, whose fields are every key the program emits it with
const EVENT_CLASS_SUFFIX: &str = "·event";
/// Maps `{a:1}` (P200b: they share, like instances) are instances of one class `map` whose fields are every key the
/// program writes, in a literal or with `m.key = v`; reading a key never written is W0's run-time "unset field"
const MAP_CLASS: &str = "map";
/// `count xs`, `count x in xs` (how often x is in xs) and `x in xs` (x's first position from 1, else 0) walk the list
/// with a `·tally` instance: its int fields hold the count or position and the current index
const COUNT_WORD: &str = "count";
/// `min(a, b)` and `max(a, b)` of two values (lowering/min_max.rs): a comparison, each argument computed once
const EXTREMA: [&str; 2] = ["min", "max"];
const TALLY_CLASS: &str = "·tally";
const TALLY_INDEX: &str = "·index";

/// W0's answer for one program
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelVerdict {
	Accepted(String),
	Rejected,
}

type Lean = Result<String, String>;

fn unsupported(node: &Node) -> Lean {
	Err(format!("not in W0: {}", node.serialize().trim()))
}

fn quoted(text: &str) -> String {
	format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `int`, `texts`, `list`: the W0 type a warp type word names
fn type_of_word(word: &str) -> Option<String> {
	let scalar = |word: &str| -> Option<&str> {
		Some(match crate::type_kinds::canonical_type_name(&word.to_lowercase()) {
			"int" | "integer" | "long" => ".int",
			"float" | "number" | "exact" => ".number",
			"text" | "string" | "str" => ".text",
			"bool" | "boolean" => BOOL_TYPE,
			"any" => ANY_TYPE,
			_ => return None,
		})
	};
	if word == "list" {
		return Some(".list .any".to_string());
	}
	match scalar(word) {
		Some(lean) => Some(lean.to_string()),
		None => word.strip_suffix('s').and_then(scalar).map(|element| format!(".list {element}")),
	}
}

/// `[.int, .text]` of the union type `(Ty.joinAll [.int, .text])`
fn union_alternatives(declared: &str) -> Option<&str> {
	declared.strip_prefix(UNION_TYPE)?.strip_suffix(')')
}

/// a value given to a declared place: a union checks it against its alternatives when it runs
fn admitted(declared: &str, value: String) -> String {
	match union_alternatives(declared) {
		Some(alternatives) => format!(".cast ({value}) {alternatives}"),
		None => value,
	}
}

/// `on`/`emit` forms; an event is named by one or more words (`on stop the machine {…}`)
enum Effect<'a> {
	On { event: String, handler: &'a Node, body: Option<&'a Node> },
	Emit { event: String, payload: Vec<(String, &'a Node)> },
}

fn event_name(words: &[Node]) -> Option<String> {
	let words: Option<Vec<String>> = words.iter().map(|word| match word.drop_meta() {
		Node::Symbol(word) => Some(word.clone()),
		_ => None,
	}).collect();
	words.filter(|words| !words.is_empty()).map(|words| words.join(" "))
}

fn effect(node: &Node) -> Option<Effect<'_>> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	// `on ev {h} in {body}` as a value parses as `(on ev {h}) in {body}`
	if let [handled, keyword, body] = items.as_slice() {
		if is_word(keyword, IN_KEYWORD) {
			if let Some(Effect::On { event, handler, body: None }) = effect(handled) {
				return Some(Effect::On { event, handler, body: Some(body) });
			}
		}
	}
	let (first, rest) = items.split_first()?;
	if is_word(first, ON_KEYWORD) {
		// `a = on ask {2} in {…}` groups the words after `on`: `on (ask {2})`
		let rest = match rest {
			[grouped] => match grouped.drop_meta() {
				Node::List(words, Bracket::None, Separator::Space) => words.as_slice(),
				_ => rest,
			},
			_ => rest,
		};
		let (last, words) = rest.split_last()?;
		let event = event_name(words)?;
		return match last.drop_meta() {
			Node::List(_, Bracket::Curly, _) => Some(Effect::On { event, handler: last, body: None }),
			Node::List(parts, _, _) if parts.len() == 3 && is_word(&parts[1], IN_KEYWORD) => Some(Effect::On { event, handler: &parts[0], body: Some(&parts[2]) }),
			_ => None,
		};
	}
	if !is_word(first, EMIT_KEYWORD) {
		return None;
	}
	// `emit alarm{level: 3}` parses as `alarm: {level: 3}` after the event's other words
	if let Some((last, words)) = rest.split_last() {
		if let Node::Key(word, Op::Colon, fields) = last.drop_meta() {
			let Node::List(fields, Bracket::Curly, _) = fields.drop_meta() else { return None };
			let payload: Option<Vec<(String, &Node)>> = fields.iter().map(|field| match field.drop_meta() {
				Node::Key(key, Op::Colon, value) => Some((key.name(), &**value)),
				_ => None,
			}).collect();
			let mut words = words.to_vec();
			words.push((**word).clone());
			return Some(Effect::Emit { event: event_name(&words)?, payload: payload? });
		}
	}
	Some(Effect::Emit { event: event_name(rest)?, payload: vec![] })
}

/// `global n` or `global n = 0` (parsed `global: n`, `global: (n = 0)`): the name and the value
fn global_declaration(node: &Node) -> Option<(&str, Option<&Node>)> {
	let Node::Key(keyword, Op::Colon, declared) = node.drop_meta() else { return None };
	if !is_word(keyword, GLOBAL_KEYWORD) {
		return None;
	}
	match declared.drop_meta() {
		Node::Symbol(name) => Some((name, None)),
		Node::Key(name, Op::Assign, value) => match name.drop_meta() {
			Node::Symbol(name) => Some((name, Some(&**value))),
			_ => None,
		},
		_ => None,
	}
}

fn event_class(event: &str) -> String {
	format!("{event}{EVENT_CLASS_SUFFIX}")
}

/// `{a:1 b:2}`: its keys and values
fn map_entries(node: &Node) -> Option<Vec<(String, &Node)>> {
	let Node::List(items, Bracket::Curly, _) = node.drop_meta() else { return None };
	items.iter().map(|item| match item.drop_meta() {
		Node::Key(key, Op::Colon, value) if matches!(key.drop_meta(), Node::Symbol(_)) => Some((key.name(), value.as_ref())),
		_ => None,
	}).collect()
}

/// every key a map literal or a field write `x.key = v` names, when the program has a map literal
fn map_keys(program: &Node) -> Option<Vec<String>> {
	let mut has_map = false;
	let mut keys: Vec<String> = Vec::new();
	program.visit(&mut |part| {
		let written: Vec<String> = match (map_entries(part), part) {
			(Some(entries), _) => {
				has_map = true;
				entries.into_iter().map(|(key, _)| key).collect()
			}
			(None, Node::Key(target, Op::Assign, _)) => match target.drop_meta() {
				Node::Key(_, Op::Dot, field) => vec![field.name()],
				_ => vec![],
			},
			_ => vec![],
		};
		for key in written {
			if !keys.contains(&key) {
				keys.push(key);
			}
		}
	});
	has_map.then_some(keys)
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(found) if found == word)
}

/// the value of `return v`, ø of a bare `return`
fn returned(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.first().is_some_and(|first| is_word(first, RETURN_KEYWORD)) => Some(match &items[1..] {
			[] => Node::Empty,
			[value] => value.clone(),
			words => Node::List(words.to_vec(), Bracket::None, Separator::Space),
		}),
		_ => None,
	}
}

fn returns(body: &Node) -> bool {
	let mut found = false;
	body.visit(&mut |part| found |= returned(part).is_some());
	found
}

/// A function body ending in `return v` ends in v
fn without_tail_return(body: &Node) -> Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, separator) if returned(body).is_none() => {
			let mut items = items.clone();
			if let Some(value) = items.last().and_then(returned) {
				*items.last_mut().expect("a last statement") = value;
			}
			Node::List(items, Bracket::Curly, separator.clone())
		}
		_ => returned(body).unwrap_or_else(|| body.clone()),
	}
}

fn is_list_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Square, _))
}

/// The statements of a program or block: an unbracketed `;`/newline list, else the one node
fn statements(node: &Node) -> Vec<&Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.iter().collect(),
		other => vec![other],
	}
}

/// A sum type `type Color = red | rgb(…)` parses as the list of its type definitions (the sum, then each variant)
fn type_definitions(statement: &Node) -> Vec<&Node> {
	match statement.drop_meta() {
		Node::List(items, Bracket::None, _) if items.iter().all(|item| matches!(item.drop_meta(), Node::Type { .. })) => items.iter().collect(),
		other => vec![other],
	}
}

/// `f(y)`, `f(y: int)`, `f(a, b)`, `two()`: a definition head, the name and the parameters
fn function_head(head: &Node) -> Option<(&str, Vec<&Node>)> {
	let Node::List(items, _, _) = head.drop_meta() else { return None };
	let (Node::Symbol(name), parameters) = (items.first()?.drop_meta(), &items[1..]) else { return None };
	let parameters = match parameters {
		[only] if only.is_nothing() => vec![],
		[Node::List(grouped, Bracket::Round, _)] => grouped.iter().collect(),
		parameters => parameters.iter().collect(),
	};
	Some((name.as_str(), parameters))
}

/// `f(a, b) := body` and `def f(a, b) { body }`: the name, the parameters and the body
fn function_definition(statement: &Node) -> Option<(&str, Vec<&Node>, &Node)> {
	match statement.drop_meta() {
		Node::Key(head, Op::Define, body) => function_head(head).map(|(name, parameters)| (name, parameters, &**body)),
		Node::List(items, _, _) if items.len() == 2 && is_word(&items[0], DEF_KEYWORD) => match items[1].drop_meta() {
			Node::List(parts, _, _) if parts.len() == 2 => function_head(&parts[0]).map(|(name, parameters)| (name, parameters, &parts[1])),
			_ => None,
		},
		_ => None,
	}
}

/// `a, b = xs` and `a, b = 1, 2`: the names and the values after `=` (one: a list to take apart)
fn destructuring(items: &[Node]) -> Option<(Vec<String>, Vec<&Node>)> {
	let assignment = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _)))?;
	let Node::Key(target, Op::Assign, value) = items[assignment].drop_meta() else { return None };
	let names: Option<Vec<String>> = items[..assignment].iter().chain(std::iter::once(&**target)).map(|name| match name.drop_meta() {
		// `*rest` takes the items between: outside W0
		Node::Symbol(name) if !name.starts_with('*') => Some(name.clone()),
		_ => None,
	}).collect();
	Some((names?, std::iter::once(&**value).chain(&items[assignment + 1..]).collect()))
}

/// A function or lambda bound at main level: its name, parameters and body
fn callable_definition(statement: &Node) -> Option<(&str, Vec<String>, &Node)> {
	if let Some((name, parameters, body)) = function_definition(statement) {
		return Some((name, parameters.iter().map(|parameter| parameter_name(parameter)).collect(), body));
	}
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { return None };
	let (Node::Symbol(name), Node::Key(parameter, Op::FatArrow, body)) = (target.drop_meta(), unwrapped(value).drop_meta()) else { return None };
	Some((name, vec![parameter.name()], &**body))
}

/// `(e)` is e
fn unwrapped(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => unwrapped(&items[0]),
		other => other,
	}
}

fn mentions(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(symbol) if symbol == name));
	found
}

/// Warp's late binding (src/lowering/late_binding.rs): a function or lambda reading a main-level name the program
/// changes after its definition, with a call after that change, needs `global x`. W0 reads at call time, which gives
/// what warp gives for every program warp accepts, so such programs are outside W0 rather than rejected by it
fn late_binding(statements: &[&Node], globals: &[String]) -> Option<String> {
	for (position, statement) in statements.iter().enumerate() {
		let Some((name, parameters, body)) = callable_definition(statement) else { continue };
		for (offset, later) in statements[position + 1..].iter().enumerate() {
			let Node::Key(target, op, _) = later.drop_meta() else { continue };
			let Node::Symbol(changed) = target.drop_meta() else { continue };
			let read = mentions(body, changed) && !parameters.contains(changed) && !globals.contains(changed);
			if (*op == Op::Assign || op.is_compound_assign()) && read && statements[position + offset + 2..].iter().any(|call| mentions(call, name)) {
				return Some(format!("not in W0: {name} reads {changed}, which changes after {name} is defined and before a call (warp asks for `global {changed}`, late binding)"));
			}
		}
	}
	None
}

/// `not e`: yes when e is falsy
fn negation(lean: String) -> String {
	format!(".ite ({lean}) (.bool false) (.bool true)")
}

/// The class of the cell a local of function holds its value in: `f·n` for n in f
fn cell_class(function: &str, local: &str) -> String {
	format!("{function}·{local}")
}

/// The names a function body assigns (`n = 0`, `n += 1`, a parameter too), in order of first assignment, but for the
/// given globals: its locals (functions2)
fn assigned_locals(body: &Node, globals: &[String]) -> Vec<String> {
	let mut locals: Vec<String> = vec![];
	body.visit(&mut |part| {
		if let Node::Key(target, op, _) = part {
			match target.drop_meta() {
				Node::Symbol(name) if (*op == Op::Assign || op.is_compound_assign()) && !globals.contains(name) && !locals.contains(name) => locals.push(name.clone()),
				_ => {}
			}
		}
	});
	locals
}

/// Whether the first value a function body gives local is a list (`out = []`, `out = [1]`)
fn first_value_is_list(body: &Node, local: &str) -> bool {
	let mut first = None;
	body.visit(&mut |part| match part {
		Node::Key(target, Op::Assign, value) if first.is_none() && matches!(target.drop_meta(), Node::Symbol(name) if name == local) => {
			first = Some(is_list_literal(value) || matches!(value.drop_meta(), Node::Empty));
		}
		_ => {}
	});
	first.unwrap_or(false)
}

/// The parameter's type word: `int` of `y: int`, none for an unannotated `y`
fn parameter_type_word(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, annotation) => Some(annotation.name()),
		Node::Key(parameter, Op::Assign, _) => parameter_type_word(parameter),
		_ => None,
	}
}

/// `b`, `b: int`, `b=3` and `b: int = 3` all name the parameter b
fn parameter_name(parameter: &Node) -> String {
	match parameter.drop_meta() {
		Node::Key(parameter, Op::Colon | Op::Assign, _) => parameter_name(parameter),
		parameter => parameter.name(),
	}
}

/// The class a function of several parameters takes them as: `f(a, b)` takes one `f·args` with fields a and b
fn arguments_class(function: &str) -> String {
	format!("{function}{ARGUMENTS_SUFFIX}")
}

/// A bool place (a declared variable or parameter) takes the literals 1 and 0 as yes and no (P199)
fn bool_literal(declared: Option<&str>, value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::Number(Number::Int(n @ (0 | 1))) if declared == Some(BOOL_TYPE) => Some(format!(".bool {}", *n == 1)),
		_ => None,
	}
}

/// A value whose type is evidently bool: `true`, `1 < 2`
fn is_evident_bool(value: &Node) -> bool {
	matches!(value.drop_meta(), Node::True | Node::False | Node::Key(_, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq | Op::Ne, _))
}

fn lean_strings(words: &[String]) -> String {
	format!("[{}]", words.iter().map(|word| quoted(word)).collect::<Vec<_>>().join(", "))
}

#[derive(Default)]
struct Exporter {
	/// the functions of the program with their parameter's W0 type
	functions: HashMap<String, String>,
	/// the parameter name of each function of one parameter: `f(n=3)` names it
	single_parameters: HashMap<String, String>,
	/// main-level names bound so far, with their declared type when annotated
	names: HashMap<String, Option<String>>,
	/// main-level names bound to an evident bool (`b = true`): a bool place for the literals 1 and 0 (P199)
	bool_names: Vec<String>,
	/// main-level names bound to a list (`xs = [1]`, `xs = []`, `xs: ints = …`): `xs.add(v)` appends to them, while
	/// `.add` of a text concatenates, outside W0
	list_names: Vec<String>,
	locals: Vec<String>,
	/// the parameters of the function being exported when it takes several: each is a field of its arguments object
	argument_fields: Vec<String>,
	/// each class's ancestor chain, root first, and its own fields with their type words
	classes: HashMap<String, ClassShape>,
	/// the default values of a function's parameters (`f(a, b=3)`), by its arguments class: a call missing b passes 3
	defaults: HashMap<String, HashMap<String, Node>>,
	/// the event whose block a `break` ends: the innermost block handler's; none in a loop (the loop's break) or a
	/// program-wide handler
	breaking: Vec<Option<String>>,
	/// the names declared `global` anywhere in the program
	globals: Vec<String>,
	/// the locals of the function being exported: each lives in a cell, a fresh instance of its class `f·n` per call
	/// (W0's locals are bound by substitution, warp's change), whose one field takes the join of the values written
	cells: Vec<String>,
	/// the event a `return` of the function being exported emits, when it returns before its end
	returning: Option<String>,
	/// the classes of the function locals' cells: a parameter of one holds the cell itself (a lifted nested function's
	/// nonlocal, nested_functions.rs)
	cell_classes: Vec<String>,
	/// the cell locals whose first value is a list: `xs.add(v)` appends to them
	cell_lists: Vec<String>,
	/// main-level names bound to a lambda: warp makes them functions, so a bare `f` is a call missing its argument
	lambda_names: Vec<String>,
	/// how many hidden main-level names (`·tuple1`, `·count1`) destructuring took so far
	hidden_names: usize,
}

/// A class's ancestor chain and its own fields as (name, type word)
/// a class's ancestor chain, root first, and its own fields with their type words (none: unannotated)
type ClassShape = (Vec<String>, Vec<(String, Option<String>)>);

impl Exporter {
	/// `int`, `texts`, `Point`: the W0 type a type word names
	fn type_of(&self, word: &str) -> Lean {
		match self.classes.get(word) {
			Some((path, _)) => Ok(format!(".cls {}", lean_strings(path))),
			None => type_of_word(word).ok_or_else(|| format!("not in W0: type {word}")),
		}
	}

	/// `(some T)` of a type word, `none` of an unannotated place: the model infers it from the values given to it
	fn optional_type(&self, word: Option<&str>) -> Lean {
		Ok(match word {
			Some(word) => format!("(some ({}))", self.type_of(word)?),
			None => "none".to_string(),
		})
	}

	/// `int or text`, `int?`: the join of the alternatives; a union warp narrows to one part (`int | float` is float)
	/// is that part
	fn union_type(&self, union: &str) -> Lean {
		let parts = union.strip_suffix(OPTIONAL_MARK);
		let mut alternatives = parts.unwrap_or(union).split(UNION_JOINER).map(|part| self.type_of(part)).collect::<Result<Vec<_>, String>>()?;
		if parts.is_some() {
			alternatives.push(EMPTY_TYPE.to_string());
		}
		Ok(match alternatives.as_slice() {
			[single] => single.clone(),
			_ => format!("{UNION_TYPE}[{}])", alternatives.join(", ")),
		})
	}

	fn annotation_type(&self, annotation: &Node) -> Lean {
		if let Some(union) = crate::analyzer::union_type_name(annotation) {
			return self.union_type(&union);
		}
		match annotation.drop_meta() {
			Node::Symbol(word) if word.ends_with(OPTIONAL_MARK) => self.union_type(word),
			Node::Symbol(word) => self.type_of(word),
			other => unsupported(other),
		}
	}

	/// `y`, `y: int`: the parameter's name and W0 type
	fn parameter_type(&self, parameter: &Node) -> Result<(String, String), String> {
		match parameter.drop_meta() {
			Node::Symbol(parameter) => Ok((parameter.clone(), ANY_TYPE.to_string())),
			Node::Key(parameter, Op::Colon, annotation) => Ok((parameter.name(), self.annotation_type(annotation)?)),
			Node::Key(parameter, Op::Assign, _) => self.parameter_type(parameter),
			other => Err(unsupported(other).unwrap_err()),
		}
	}

	/// `class C extends P { f: T … }` and the variants of `type Color = … | rgb(r: int …)` (a variant extends its sum)
	fn collect_classes(&mut self, program: &Node) {
		let own_fields = crate::lowering::class_methods::class_fields(program);
		let mut parents = HashMap::new();
		program.visit(&mut |part| if let Node::Type { name, .. } = part {
			parents.insert(name.drop_meta().name(), name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(Node::name));
		});
		fn path(class: &str, parents: &HashMap<String, Option<String>>, depth: usize) -> Vec<String> {
			let mut chain = match parents.get(class) {
				Some(Some(parent)) if depth < parents.len() => path(parent, parents, depth + 1),
				_ => vec![],
			};
			chain.push(class.to_string());
			chain
		}
		for class in parents.keys() {
			let fields = own_fields.get(class).cloned().unwrap_or_default().into_iter().map(|(field, word)| (field, (!word.is_empty()).then_some(word))).collect();
			self.classes.insert(class.clone(), (path(class, &parents, 0), fields));
		}
	}

	/// all fields of a class, its ancestors' first, the order its constructor takes them in
	fn constructor_fields(&self, class: &str) -> Vec<String> {
		let (path, _) = &self.classes[class];
		path.iter().flat_map(|ancestor| self.classes[ancestor].1.iter().map(|(field, _)| field.clone())).collect()
	}

	/// `C(a, b)`: a new instance with its fields written in order, `let o = new C; o.f1 = a; …; o`
	fn construction(&mut self, class: &str, arguments: &[Node]) -> Lean {
		let written = self.field_values(class, arguments)?;
		self.instance(class, written.iter().map(|(field, value)| (field.clone(), value)).collect())
	}

	/// The fields an argument list writes: a named argument `f(b=1, a=5)` goes to its field, the positional ones fill
	/// the others in order, a missing one takes its default; they run as written (P216)
	fn field_values(&self, class: &str, arguments: &[Node]) -> Result<Vec<(String, Node)>, String> {
		let fields = self.constructor_fields(class);
		let named: Vec<Option<(String, Node)>> = arguments.iter().map(|argument| match argument.drop_meta() {
			Node::Key(name, Op::Assign, value) => Some((name.name(), value.as_ref().clone())),
			_ => None,
		}).collect();
		if let Some((unknown, _)) = named.iter().flatten().find(|(name, _)| !fields.contains(name)) {
			return Err(format!("{class} has no field {unknown}"));
		}
		let named_fields: Vec<String> = named.iter().flatten().map(|(name, _)| name.clone()).collect();
		let mut unnamed = fields.iter().filter(|field| !named_fields.contains(field));
		let written: Option<Vec<(String, Node)>> = arguments.iter().zip(named).map(|(argument, named)| named.or_else(|| unnamed.next().map(|field| (field.clone(), argument.clone())))).collect();
		let mut written = written.ok_or_else(|| format!("{class} takes {} fields, got {}", fields.len(), arguments.len()))?;
		let defaults = self.defaults.get(class);
		for field in unnamed {
			let default = defaults.and_then(|defaults| defaults.get(field)).ok_or_else(|| format!("{class} needs a value for {field}"))?;
			written.push((field.clone(), default.clone()));
		}
		Ok(written)
	}

	/// a new instance of class with the given fields written, `let o = new C; o.f1 = a; …; o`
	fn instance(&mut self, class: &str, fields: Vec<(String, &Node)>) -> Lean {
		let path = lean_strings(&self.classes[class].0);
		let local = format!("new·{class}");
		let writes: Result<Vec<String>, String> = fields.iter().map(|(field, value)| {
			let field_type = self.classes[class].1.iter().find(|(own, _)| own == field).and_then(|(_, word)| word.clone());
			Ok(format!(".set (.loc {}) {} ({})", quoted(&local), quoted(field), self.argument(field_type.as_deref(), value)?))
		}).collect();
		let body = writes?.iter().rev().fold(format!(".loc {}", quoted(&local)), |rest, write| format!(".seq ({write}) ({rest})"));
		Ok(format!(".letIn {} (.cls {path}) (.new {path}) ({body})", quoted(&local)))
	}

	/// `ev·event` with every key the program emits ev with
	fn collect_event_classes(&mut self, program: &Node) -> Vec<String> {
		let mut keys: Vec<(String, Vec<String>)> = vec![];
		program.visit(&mut |part| if let Some(Effect::Emit { event, payload }) = effect(part) {
			let class = event_class(&event);
			let position = keys.iter().position(|(known, _)| *known == class).unwrap_or_else(|| {
				keys.push((class, vec![]));
				keys.len() - 1
			});
			for (key, _) in payload {
				if !keys[position].1.contains(&key) {
					keys[position].1.push(key);
				}
			}
		});
		keys.retain(|(_, fields)| !fields.is_empty());
		for (class, fields) in &keys {
			self.classes.insert(class.clone(), (vec![class.clone()], fields.iter().map(|field| (field.clone(), None)).collect()));
		}
		keys.into_iter().map(|(class, _)| class).collect()
	}

	/// a handler body: `event` is its payload; a block handler (of `on ev {…} in {…}`) may `break`
	fn handler(&mut self, handler: &Node, block_event: Option<&str>) -> Lean {
		self.locals.push(EVENT_LOCAL.to_string());
		let handler = self.breaking_in(block_event.map(str::to_string), |exporter| exporter.block(handler));
		self.locals.pop();
		handler
	}

	fn breaking_in(&mut self, event: Option<String>, export: impl FnOnce(&mut Self) -> Lean) -> Lean {
		self.breaking.push(event);
		let exported = export(self);
		self.breaking.pop();
		exported
	}

	/// `break v` or a bare `break` (ø) in a block handler
	fn abort(&mut self, value: Option<&Node>) -> Lean {
		let Some(event) = self.breaking.last().cloned().flatten() else {
			return Err("not in W0: a break outside a block handler".to_string());
		};
		let value = match value {
			Some(value) => self.expression(value)?,
			None => UNIT_TYPE.to_string(),
		};
		Ok(format!(".abort {} none ({value})", quoted(&event)))
	}

	fn effect(&mut self, effect: Effect) -> Lean {
		match effect {
			Effect::On { event, handler, body: Some(body) } => Ok(format!(".handle {} ({}) ({})", quoted(&event), self.handler(handler, Some(&event))?, self.block(body)?)),
			Effect::On { event, .. } => Err(format!("not in W0: a program-wide handler of {event} inside an expression")),
			Effect::Emit { event, payload } if payload.is_empty() => Ok(format!(".emit {} .unit", quoted(&event))),
			Effect::Emit { event, payload } => Ok(format!(".emit {} ({})", quoted(&event), self.instance(&event_class(&event), payload)?)),
		}
	}

	fn class_definition(&self, class: &str) -> Lean {
		let fields: Result<Vec<String>, String> = self.classes[class].1.iter().map(|(field, type_word)| Ok(format!("({}, {})", quoted(field), self.optional_type(type_word.as_deref())?))).collect();
		Ok(format!(".classDef {} [{}]", quoted(class), fields?.join(", ")))
	}

	fn items(&mut self, program: &Node) -> Result<Vec<String>, String> {
		let with_modules = used_modules::inline(program)?;
		let program = &with_modules;
		self.collect_classes(program);
		program.visit(&mut |part| if let Some((name, _)) = global_declaration(part) {
			self.globals.push(name.to_string());
		});
		let lifted = nested_functions::lift(program, &self.globals)?;
		let program = &lifted;
		let event_classes = self.collect_event_classes(program);
		let program_node = program;
		let program: Vec<&Node> = statements(program).into_iter().flat_map(type_definitions).collect();
		if let Some(why) = late_binding(&program, &self.globals) {
			return Err(why);
		}
		let mut argument_classes = Vec::new();
		let mut cell_items = Vec::new();
		// first: a lifted nested function's parameter `y: outer·y` holds outer's cell
		for statement in &program {
			let Some((name, _, body)) = function_definition(statement) else { continue };
			for local in assigned_locals(body, &self.globals) {
				let class = cell_class(name, &local);
				self.classes.insert(class.clone(), (vec![class.clone()], vec![(CELL_FIELD.to_string(), None)]));
				cell_items.push(format!(".cell {}", quoted(&class)));
				self.cell_classes.push(class);
			}
		}
		for statement in &program {
			let Some((name, parameters, _)) = function_definition(statement) else { continue };
			if self.functions.contains_key(name) {
				return Err(format!("not in W0: overloads of {name}"));
			}
			let parameter_type = match parameters.as_slice() {
				[] => UNIT_TYPE.to_string(),
				[parameter] => {
					self.single_parameters.insert(name.to_string(), parameter_name(parameter));
					self.parameter_type(parameter)?.1
				}
				parameters => {
					let class = arguments_class(name);
					let fields = parameters.iter().map(|parameter| Ok((self.parameter_type(parameter)?.0, parameter_type_word(parameter)))).collect::<Result<Vec<_>, String>>()?;
					let defaults = parameters.iter().filter_map(|parameter| match parameter.drop_meta() {
						Node::Key(_, Op::Assign, default) => Some((parameter_name(parameter), default.as_ref().clone())),
						_ => None,
					});
					self.defaults.insert(class.clone(), defaults.collect());
					self.classes.insert(class.clone(), (vec![class.clone()], fields));
					argument_classes.push(class.clone());
					format!(".cls {}", lean_strings(&[class]))
				}
			};
			self.functions.insert(name.to_string(), parameter_type);
		}
		argument_classes.extend(event_classes);
		let mut words_used = false;
		program_node.visit(&mut |part| words_used |= is_word(part, COUNT_WORD) || is_word(part, IN_KEYWORD));
		if words_used {
			let fields = [CELL_FIELD, TALLY_INDEX].map(|field| (field.to_string(), Some("int".to_string())));
			self.classes.insert(TALLY_CLASS.to_string(), (vec![TALLY_CLASS.to_string()], fields.to_vec()));
			argument_classes.push(TALLY_CLASS.to_string());
		}
		if let Some(keys) = map_keys(program_node) {
			self.classes.insert(MAP_CLASS.to_string(), (vec![MAP_CLASS.to_string()], keys.into_iter().map(|key| (key, None)).collect()));
			argument_classes.push(MAP_CLASS.to_string());
		}
		let mut items = argument_classes.iter().map(|class| self.class_definition(class)).collect::<Result<Vec<_>, String>>()?;
		items.extend(cell_items);
		for statement in program {
			items.push(self.item(statement)?);
		}
		Ok(items)
	}

	fn item(&mut self, statement: &Node) -> Lean {
		if let Some(Effect::On { event, handler, body: None }) = effect(statement) {
			return Ok(format!(".on {} ({})", quoted(&event), self.handler(handler, None)?));
		}
		match global_declaration(statement) {
			Some((name, Some(value))) => return self.binding(&Node::Symbol(name.to_string()), ".var", value),
			// `global x; x = 7`: the later first binding is the global one
			Some((_, None)) => return Ok(format!(".statement {UNIT_TYPE}")),
			None => {}
		}
		match statement.drop_meta() {
			Node::Type { name, .. } => self.class_definition(&name.drop_meta().name()),
			_ if function_definition(statement).is_some() => {
				let (name, parameters, body) = function_definition(statement).expect("a function definition");
				self.function(name, &parameters, body)
			}
			Node::Key(head, Op::Define, body) => match head.drop_meta() {
				Node::Symbol(name) => {
					self.names.insert(name.clone(), None);
					Ok(format!(".charged {} ({})", quoted(name), self.expression(body)?))
				}
				_ => unsupported(statement),
			},
			Node::List(items, _, _) if items.len() == 2 && is_word(&items[0], CONSTANT_KEYWORD) => match items[1].drop_meta() {
				Node::Key(target, Op::Assign, value) => self.binding(target, ".const", value),
				other => unsupported(other),
			},
			Node::List(items, Bracket::None, Separator::Colon) if destructuring(items).is_some() => {
				let (names, values) = destructuring(items).expect("a destructuring");
				match values[..] {
					[list] => self.destructure(&names, list),
					_ => self.assign_tuple(&names, &values),
				}
			}
			// `let x = 1`, `shared n = 5` (one thread in W0): a variable; `int i = 2`: `i: int = 2`
			Node::List(items, _, _) if items.len() == 2 && matches!(items[1].drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(_))) => {
				let Node::Key(target, _, value) = items[1].drop_meta() else { unreachable!("an assignment") };
				match items[0].drop_meta() {
					Node::Symbol(word) if VARIABLE_KEYWORDS.contains(&word.as_str()) => self.binding(target, ".var", value),
					Node::Symbol(word) if self.type_of(word).is_ok() => self.binding(&Node::Key(target.clone(), Op::Colon, Box::new(items[0].clone())), ".var", value),
					_ => Ok(format!(".statement ({})", self.expression(statement)?)),
				}
			}
			// `a = b = 3`: `b = 3`, then `a = b`
			Node::Key(target, Op::Assign, value) if matches!(value.drop_meta(), Node::Key(_, Op::Assign, _)) => {
				let Node::Key(inner_target, _, _) = value.drop_meta() else { unreachable!("an assignment") };
				Ok(format!("{},\n  {}", self.item(value)?, self.item(&Node::Key(target.clone(), Op::Assign, inner_target.clone()))?))
			}
			Node::Key(target, Op::Assign, value) if !self.is_bound(target) && !matches!(target.drop_meta(), Node::Key(_, Op::Dot, _)) => self.binding(target, ".var", value),
			other => Ok(format!(".statement ({})", self.expression(other)?)),
		}
	}

	/// `a, b = xs`: the hidden `·tupleN = xs`, its items counted (warp fails on a wrong number of values), then
	/// `a = ·tupleN#1`, `b = ·tupleN#2`; several items in one
	fn destructure(&mut self, names: &[String], list: &Node) -> Lean {
		if let Some(bound) = names.iter().find(|name| self.names.contains_key(*name)) {
			return Err(format!("not in W0: destructuring into {bound}, which holds a value (card destructure-existing)"));
		}
		self.hidden_names += 1;
		let (tuple, count) = (format!("·tuple{}", self.hidden_names), format!("·count{}", self.hidden_names));
		let mut items = vec![
			format!(".bind {} .var none ({}) false", quoted(&tuple), self.expression(list)?),
			format!(".bind {} .var none (.int 0) false", quoted(&count)),
			format!(".statement (.forIn \"·item\" (.glob {}) (.assign {} (.add (.glob {}) (.int 1))) {UNIT_TYPE})", quoted(&tuple), quoted(&count), quoted(&count)),
			format!(".statement (.ite (.eq false (.glob {}) (.int {})) .unit (.error \"wrong number of values\"))", quoted(&count), names.len()),
		];
		for (position, name) in names.iter().enumerate() {
			items.push(self.store(name, format!(".index (.glob {}) (.int {})", quoted(&tuple), position + 1)));
		}
		Ok(items.join(",\n  "))
	}

	/// `a, b = 1, 2`: each value in a hidden `·valueN_i` first, so `a, b = b, a` swaps, then into the names, which may
	/// hold values already (checked against their declared types as any assignment)
	fn assign_tuple(&mut self, names: &[String], values: &[&Node]) -> Lean {
		if names.len() != values.len() {
			return Err(format!("not in W0: {} values for {} names (warp counts them when it compiles)", values.len(), names.len()));
		}
		self.hidden_names += 1;
		let hidden: Vec<String> = (1..=values.len()).map(|position| format!("·value{}_{position}", self.hidden_names)).collect();
		let mut items = Vec::new();
		for (name, value) in hidden.iter().zip(values) {
			items.push(format!(".bind {} .var none ({}) false", quoted(name), self.expression(value)?));
		}
		for (name, value) in names.iter().zip(&hidden) {
			items.push(self.store(name, format!(".glob {}", quoted(value))));
		}
		Ok(items.join(",\n  "))
	}

	/// a main-level name takes a value: its first binding, or an assignment checked against its declared type
	fn store(&mut self, name: &str, value: String) -> String {
		match self.names.get(name).cloned() {
			Some(declared) => format!(".statement (.assign {} ({}))", quoted(name), declared.map_or(value.clone(), |declared| admitted(&declared, value))),
			None => {
				self.names.insert(name.to_string(), None);
				format!(".bind {} .var none ({value}) {}", quoted(name), self.globals.contains(&name.to_string()))
			}
		}
	}

	fn is_bound(&self, target: &Node) -> bool {
		matches!(target.drop_meta(), Node::Symbol(name) if self.names.contains_key(name))
	}

	/// the first binding of a main-level name: `x = 1`, `x: int = 1`, `const x = 1`
	fn binding(&mut self, target: &Node, mode: &str, value: &Node) -> Lean {
		let (name, annotation) = match target.drop_meta() {
			Node::Symbol(name) => (name.clone(), None),
			Node::Key(name, Op::Colon, annotation) => (name.name(), Some(self.annotation_type(annotation)?)),
			other => return unsupported(other),
		};
		if annotation.is_none() && is_evident_bool(value) {
			self.bool_names.push(name.clone());
		}
		if matches!(unwrapped(value), Node::Key(_, Op::FatArrow, _)) {
			self.lambda_names.push(name.clone());
		}
		if is_list_literal(value) || matches!(value.drop_meta(), Node::Empty) || annotation.as_deref().is_some_and(|lean| lean.starts_with(".list")) {
			self.list_names.push(name.clone());
		}
		let value = self.stored_value(annotation.as_deref(), value)?;
		self.names.insert(name.clone(), annotation.clone());
		let annotation = annotation.map_or("none".to_string(), |lean| format!("(some ({lean}))"));
		Ok(format!(".bind {} {mode} {annotation} ({value}) {}", quoted(&name), self.globals.contains(&name)))
	}

	/// A call's result stored in a declared list is checked item by item at run time (list-element-types): a cast
	fn stored_value(&mut self, declared: Option<&str>, value: &Node) -> Lean {
		if let Some(yes_or_no) = bool_literal(declared, value) {
			return Ok(yes_or_no);
		}
		let lean = self.expression(value)?;
		Ok(match declared {
			Some(declared) if declared.starts_with(".list") && self.is_call(value) => format!(".cast ({lean}) [{declared}]"),
			Some(declared) => admitted(declared, lean),
			None => lean,
		})
	}

	/// a field some class declares: other words after a dot are methods (`x.size`, `4.square`), outside W0
	fn is_field(&self, word: &str) -> bool {
		self.classes.values().any(|(_, fields)| fields.iter().any(|(field, _)| field == word))
	}

	fn is_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.functions.contains_key(name)))
	}

	/// A function of one parameter as it is; of none, a function of `unit`; of several, a function of its
	/// arguments object, whose fields the parameters read
	fn function(&mut self, name: &str, parameters: &[&Node], body: &Node) -> Lean {
		let declared = format!("(some ({}))", self.functions[name]);
		let (parameter, fields, parameter_type) = match parameters {
			[parameter] => {
				let inferred = parameter_type_word(parameter).is_none();
				(self.parameter_type(parameter)?.0, vec![], if inferred { "none".to_string() } else { declared })
			}
			[] => (UNIT_PARAMETER.to_string(), vec![], declared),
			parameters => (arguments_class(name), parameters.iter().map(|parameter| parameter_name(parameter)).collect(), declared),
		};
		let references: Vec<String> = parameters.iter().filter(|parameter| parameter_type_word(parameter).is_some_and(|word| self.cell_classes.contains(&word))).map(|parameter| parameter_name(parameter)).collect();
		let cells: Vec<String> = assigned_locals(body, &self.globals).into_iter().filter(|cell| !references.contains(cell)).collect();
		// an assigned parameter's cell starts with the argument (read before the cell's name hides it)
		let first_values: Vec<String> = cells.iter().map(|cell| match () {
			_ if *cell == parameter => format!(".loc {}", quoted(cell)),
			_ if fields.contains(cell) => format!(".get (.loc {}) {}", quoted(&parameter), quoted(cell)),
			_ => UNIT_TYPE.to_string(),
		}).collect();
		self.cell_lists = cells.iter().filter(|cell| first_value_is_list(body, cell)).cloned().collect();
		self.cells = cells.iter().chain(&references).cloned().collect();
		self.locals.push(parameter.clone());
		self.argument_fields = fields.clone();
		let body = &without_tail_return(body);
		let return_event = format!("{name}{RETURN_EVENT_SUFFIX}");
		self.returning = returns(body).then(|| return_event.clone());
		let body = self.block(body).map(|body| match self.returning.take() {
			Some(event) => format!(".handle {event} (.abort {event} none (.loc {})) ({body})", quoted(EVENT_LOCAL), event = quoted(&event)),
			None => body,
		});
		self.locals.pop();
		self.argument_fields.clear();
		self.cells.clear();
		let body = cells.iter().zip(&first_values).rev().fold(body?, |body, (cell, first_value)| {
			let path = lean_strings(&[cell_class(name, cell)]);
			let new_cell = match first_value.as_str() {
				UNIT_TYPE => format!(".new {path}"),
				_ => format!(".letIn \"·new\" (.cls {path}) (.new {path}) (.seq (.set (.loc \"·new\") {} ({first_value})) (.loc \"·new\"))", quoted(CELL_FIELD)),
			};
			format!(".letIn {} (.cls {path}) ({new_cell}) ({body})", quoted(cell))
		});
		// a cell parameter among several is read from the arguments object once
		let body = references.iter().filter(|reference| fields.contains(reference)).fold(body, |body, reference| {
			let cell = parameters.iter().find(|parameter| parameter_name(parameter) == **reference).and_then(|parameter| parameter_type_word(parameter)).expect("a cell parameter");
			let path = lean_strings(&[cell]);
			format!(".letIn {} (.cls {path}) (.get (.loc {}) {}) ({body})", quoted(reference), quoted(&parameter), quoted(reference))
		});
		Ok(format!(".function {} {} {parameter_type} ({body})", quoted(name), quoted(&parameter)))
	}

	/// a function body: `{ a; b }` is the sequence
	fn block(&mut self, body: &Node) -> Lean {
		match body.drop_meta() {
			// `{emit too big{value: x}}`: the words of one statement
			Node::List(items, Bracket::Curly, Separator::Space) if items.len() > 1 => self.expression(&Node::List(items.clone(), Bracket::None, Separator::Space)),
			Node::List(items, Bracket::Curly, _) if !items.is_empty() => {
				let statements: Result<Vec<String>, String> = items.iter().map(|item| self.expression(item)).collect();
				let mut statements = statements?;
				let last = statements.pop().expect("a statement");
				Ok(statements.iter().rev().fold(last, |rest, statement| format!(".seq ({statement}) ({rest})")))
			}
			_ => self.expression(body),
		}
	}

	fn assignment(&mut self, name: &str, value: &Node) -> Lean {
		if self.cells.iter().any(|cell| cell == name) {
			return Ok(format!(".set (.loc {}) {} ({})", quoted(name), quoted(CELL_FIELD), self.expression(value)?));
		}
		let declared = self.names.get(name).cloned().flatten();
		let value = match bool_literal(self.bool_names.contains(&name.to_string()).then_some(BOOL_TYPE), value) {
			Some(yes_or_no) => yes_or_no,
			None => self.stored_value(declared.as_deref(), value)?,
		};
		if !self.names.contains_key(name) {
			return Err(format!("not in W0: a first binding of {name} inside an expression"));
		}
		Ok(format!(".assign {} ({value})", quoted(name)))
	}

	/// `for x in xs { body }`: x is a local of the body; a `break` in it ends the loop. A type word or `it` as the
	/// variable (type filters, unit walks) is not in W0, nor a main-level name (warp's loop assigns it, W0's shadows it)
	fn for_loop(&mut self, variable: &Node, list: &Node, body: &Node) -> Lean {
		let variable = match variable.drop_meta() {
			Node::Symbol(name) if name != crate::lambdas::IMPLICIT_PARAMETER && type_of_word(name).is_none() && !self.names.contains_key(name) && !self.cells.contains(name) => name.clone(),
			_ => return Err(format!("not in W0: the loop variable {}", variable.serialize().trim())),
		};
		let list = self.expression(list)?;
		self.locals.push(variable.clone());
		let body = self.breaking_in(None, |exporter| exporter.block(body));
		self.locals.pop();
		Ok(format!(".forIn {} ({list}) ({}) {UNIT_TYPE}", quoted(&variable), body?))
	}

	/// `x => body`: x is a local of the body; a lambda of no or several parameters is not in W0
	fn lambda(&mut self, parameter: &Node, body: &Node) -> Lean {
		let parameter = match parameter.drop_meta() {
			Node::Symbol(name) => name.clone(),
			other => return Err(format!("not in W0: the lambda parameters {}", other.serialize().trim())),
		};
		self.locals.push(parameter.clone());
		let body = self.breaking_in(None, |exporter| exporter.block(body));
		self.locals.pop();
		Ok(format!(".lam {} ({})", quoted(&parameter), body?))
	}

	/// a variable, parameter or local (not a function or class): what a call of it calls is its value
	fn holds_value(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(name) => !self.functions.contains_key(name) && !self.classes.contains_key(name)
				&& (self.names.contains_key(name) || self.locals.contains(name) || self.argument_fields.contains(name) || self.cells.contains(name)),
			_ => false,
		}
	}

	/// `min(a, b)` is `b < a ? b : a`, `max(a, b)` is `a < b ? b : a`, a and b bound once
	fn extremum(&mut self, word: &str, a: &Node, b: &Node) -> Lean {
		self.hidden_names += 1;
		let (first, second) = (quoted(&format!("·a{}", self.hidden_names)), quoted(&format!("·b{}", self.hidden_names)));
		let (left, right) = if word == EXTREMA[0] { (&second, &first) } else { (&first, &second) };
		let comparison = format!(".ite (.lt (.loc {left}) (.loc {right})) (.loc {second}) (.loc {first})");
		Ok(format!(".letIn {first} {ANY_TYPE} ({}) (.letIn {second} {ANY_TYPE} ({}) ({comparison}))", self.expression(a)?, self.expression(b)?))
	}

	/// a literal, a name (not a function's), or operators on such: evaluating it twice changes nothing
	fn is_pure(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
			Node::Symbol(name) => !self.functions.contains_key(name),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_pure(&items[0]),
			Node::Key(left, op, right) => !matches!(op, Op::Assign | Op::Define | Op::Dot) && !op.is_compound_assign() && self.is_pure(left) && self.is_pure(right),
			_ => false,
		}
	}

	/// the value given to a place of the given type word: to a cell parameter, the caller's cell itself
	fn argument(&mut self, type_word: Option<&str>, value: &Node) -> Lean {
		match value.drop_meta() {
			Node::Symbol(name) if type_word.is_some_and(|word| self.cell_classes.iter().any(|cell| cell == word)) && self.cells.contains(name) => Ok(format!(".loc {}", quoted(name))),
			_ => self.expression(value),
		}
	}

	fn cell_value(&self, cell: &str) -> String {
		format!(".get (.loc {}) {}", quoted(cell), quoted(CELL_FIELD))
	}

	/// `count list`, `count item in list`, and with `position` `item in list`: a walk of the list with a fresh `·tally`
	fn tally(&mut self, list: &Node, item: Option<&Node>, position: bool) -> Lean {
		let (tally, path) = (".loc \"·t\"", lean_strings(&[TALLY_CLASS.to_string()]));
		let field = |name: &str| format!(".get ({tally}) {}", quoted(name));
		let write = |name: &str, value: String| format!(".set ({tally}) {} ({value})", quoted(name));
		let increment = |name: &str| write(name, format!(".add ({}) (.int 1)", field(name)));
		let matches = |then: String| format!(".ite (.eq false (.loc \"·item\") (.loc \"·x\")) ({then}) .unit");
		let step = match (item, position) {
			(None, _) => increment(CELL_FIELD),
			(Some(_), false) => matches(increment(CELL_FIELD)),
			(Some(_), true) => {
				let first = format!(".ite (.eq false ({}) (.int 0)) ({}) .unit", field(CELL_FIELD), write(CELL_FIELD, field(TALLY_INDEX)));
				format!(".seq ({}) ({})", increment(TALLY_INDEX), matches(first))
			}
		};
		let walk = format!(".forIn \"·item\" ({}) ({step}) {UNIT_TYPE}", self.expression(list)?);
		let start = format!(".seq ({}) ({})", write(CELL_FIELD, ".int 0".to_string()), write(TALLY_INDEX, ".int 0".to_string()));
		let counted = format!(".letIn \"·t\" (.cls {path}) (.new {path}) (.seq ({start}) (.seq ({walk}) ({})))", field(CELL_FIELD));
		match item {
			Some(item) => Ok(format!(".letIn \"·x\" .any ({}) ({counted})", self.expression(item)?)),
			None => Ok(counted),
		}
	}

	fn binary(&mut self, constructor: &str, left: &Node, right: &Node) -> Lean {
		Ok(format!("{constructor} ({}) ({})", self.expression(left)?, self.expression(right)?))
	}

	fn expression(&mut self, node: &Node) -> Lean {
		if let Some(value) = returned(node) {
			let event = self.returning.clone().ok_or("not in W0: a return outside a function body")?;
			return Ok(format!(".emit {} ({})", quoted(&event), self.expression(&value)?));
		}
		match node.drop_meta() {
			Node::Number(Number::Int(n)) => Ok(format!(".int ({n})")),
			Node::Number(Number::Float(_) | Number::Quotient(_, _)) => Ok(".num 0".to_string()),
			Node::True => Ok(".bool true".to_string()),
			Node::False => Ok(".bool false".to_string()),
			Node::Text(text) => Ok(format!(".text {}", quoted(text))),
			Node::Char(c) => Ok(format!(".text {}", quoted(&c.to_string()))), // `"a"` parses as a codepoint
			Node::Empty => Ok(".nil".to_string()), // ø is the empty list (`xs = []` parses as ø)
			Node::Symbol(name) if self.cells.contains(name) => Ok(self.cell_value(name)),
			// the function's first local is its arguments object; loops and lambdas push theirs after it
			Node::Symbol(name) if self.argument_fields.contains(name) => {
				Ok(format!(".get (.loc {}) {}", quoted(self.locals.first().expect("the arguments object")), quoted(name)))
			}
			Node::Symbol(name) if self.locals.contains(name) => Ok(format!(".loc {}", quoted(name))),
			// `two()` of a function of no parameters parses as `(two)`
			Node::Symbol(name) if self.functions.get(name).is_some_and(|parameter| parameter == UNIT_TYPE) => Ok(format!(".call {} .unit", quoted(name))),
			Node::Symbol(name) if self.lambda_names.contains(name) => Err(format!("not in W0: {name} as a value (warp calls it; `function {name}` is the function)")),
			Node::Symbol(name) if self.names.contains_key(name) => Ok(format!(".glob {}", quoted(name))),
			_ if map_entries(node).is_some() && self.classes.contains_key(MAP_CLASS) => self.instance(MAP_CLASS, map_entries(node).expect("a map")),
			Node::List(items, Bracket::Square, _) => {
				let elements: Result<Vec<String>, String> = items.iter().map(|item| self.expression(item)).collect();
				Ok(elements?.iter().rev().fold(".nil".to_string(), |tail, head| format!(".cons ({head}) ({tail})")))
			}
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.expression(&items[0]),
			Node::List(items, Bracket::Curly, _) if !items.is_empty() => self.block(node),
			Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) if items.len() > 1 => {
				let statements: Result<Vec<String>, String> = items.iter().map(|item| self.expression(item)).collect();
				let mut statements = statements?;
				let last = statements.pop().expect("more than one statement");
				Ok(statements.iter().rev().fold(last, |rest, statement| format!(".seq ({statement}) ({rest})")))
			}
			_ if effect(node).is_some() => self.effect(effect(node).expect("an effect")),
			Node::Symbol(word) if word == BREAK_KEYWORD => self.abort(None),
			// `global y` in a function only declares: the name is main-level and assignable there
			_ if global_declaration(node).is_some_and(|(name, value)| value.is_none() && self.names.contains_key(name)) => Ok(UNIT_TYPE.to_string()),
			Node::List(items, _, _) if items.len() == 2 && is_word(&items[0], BREAK_KEYWORD) => self.abort(Some(&items[1])),
			Node::List(items, _, _) if items.first().is_some_and(|class| self.classes.contains_key(&class.name())) => self.construction(&items[0].name(), &items[1..]),
			Node::List(items, _, _) => match items.as_slice() {
				[marker, body, handler] if is_word(marker, TRY_MARKER) => self.binary(".tryCatch", body, handler),
				[for_word, variable, in_word, list, body] if is_word(for_word, FOR_KEYWORD) && is_word(in_word, IN_KEYWORD) => self.for_loop(variable, list, body),
				// `count 2 in xs` parses as `count (2 in xs)`
				[count, list] if is_word(count, COUNT_WORD) && !self.functions.contains_key(COUNT_WORD) => match list.drop_meta() {
					Node::List(words, Bracket::None, _) if words.len() == 3 && is_word(&words[1], IN_KEYWORD) => self.tally(&words[2], Some(&words[0]), false),
					_ => self.tally(list, None, false),
				},
				[item, in_word, list] if is_word(in_word, IN_KEYWORD) => self.tally(list, Some(item), true),
				[call, a, b] if EXTREMA.iter().any(|word| is_word(call, word)) && !self.functions.contains_key(&call.name()) => self.extremum(&call.name(), a, b),
				[call, message] if is_word(call, ERROR_CALL) => match message.drop_meta() {
					Node::Text(message) => Ok(format!(".error {}", quoted(message))),
					_ => unsupported(node),
				},
				[call] if self.functions.get(&call.name()).is_some_and(|parameter| parameter == UNIT_TYPE) => Ok(format!(".call {} .unit", quoted(&call.name()))),
				// `add 1 to 2` of `to add number a to number b: …` parses as `add (1 to 2)`: two arguments
				[call, argument] if self.classes.contains_key(&arguments_class(&call.name())) && matches!(argument.drop_meta(), Node::Key(_, Op::To, _)) => {
					let Node::Key(first, _, second) = argument.drop_meta() else { unreachable!("a `to` pair") };
					let arguments = self.construction(&arguments_class(&call.name()), &[first.as_ref().clone(), second.as_ref().clone()])?;
					Ok(format!(".call {} ({arguments})", quoted(&call.name())))
				}
				[call, arguments @ ..] if self.classes.contains_key(&arguments_class(&call.name())) => {
					// an argument list that does not fit is a compile error in warp: W0 rejects a call given no arguments object
					let class = arguments_class(&call.name());
					let arguments = match self.field_values(&class, arguments) {
						Ok(written) => self.instance(&class, written.iter().map(|(field, value)| (field.clone(), value)).collect())?,
						Err(_) => UNIT_TYPE.to_string(),
					};
					Ok(format!(".call {} ({arguments})", quoted(&call.name())))
				}
				// `f(3)` of a name holding a lambda
				[value, argument] if self.holds_value(value) => {
					let function = match value.drop_meta() {
						Node::Symbol(name) if self.lambda_names.contains(name) => format!(".glob {}", quoted(name)),
						_ => self.expression(value)?,
					};
					Ok(format!(".app ({function}) ({})", self.expression(argument)?))
				}
				[call, argument] if self.functions.contains_key(&call.name()) => {
					let argument = match argument.drop_meta() {
						Node::Key(name, Op::Assign, value) if self.single_parameters.get(&call.name()) == Some(&name.name()) => value.as_ref(),
						Node::Key(name, Op::Assign, _) => return Err(format!("{} has no parameter {}", call.name(), name.name())),
						_ => argument,
					};
					let declared = self.functions[&call.name()].clone();
					let cell = self.cell_classes.iter().find(|cell| declared == format!(".cls {}", lean_strings(&[cell.to_string()]))).cloned();
					let argument = match bool_literal(Some(&declared), argument) {
						Some(yes_or_no) => yes_or_no,
						None if cell.is_some() => self.argument(cell.as_deref(), argument)?,
						None => admitted(&declared, self.expression(argument)?),
					};
					Ok(format!(".call {} ({argument})", quoted(&call.name())))
				}
				_ => unsupported(node),
			},
			// `i++` and `++i` are `i = i + 1`, both giving the new value
			Node::Key(left, op @ (Op::Inc | Op::Dec), right) => match [left, right].into_iter().find(|side| matches!(side.drop_meta(), Node::Symbol(_))) {
				Some(target) => {
					let step = if *op == Op::Inc { Op::Add } else { Op::Sub };
					self.assignment(&target.name(), &Node::Key(target.clone(), step, Box::new(Node::Number(Number::Int(1)))))
				}
				None => unsupported(node),
			},
			// `s += 2` is `s = s + 2`
			Node::Key(target, op, value) if op.is_compound_assign() => match target.drop_meta() {
				Node::Symbol(name) => self.assignment(name, &Node::Key(target.clone(), op.base_op(), value.clone())),
				_ => unsupported(node),
			},
			Node::Key(target, Op::Assign, value) => match target.drop_meta() {
				Node::Symbol(name) => self.assignment(name, value),
				Node::Key(object, Op::Dot, field) => Ok(format!(".set ({}) {} ({})", self.expression(object)?, quoted(&field.name()), self.expression(value)?)),
				_ => unsupported(node),
			},
			// `xs.add(v)` is `xs = xs ++ [v]`: lists are values
			Node::Key(list, Op::Dot, call) => match (list.drop_meta(), call.drop_meta()) {
				(Node::Symbol(name), Node::List(items, _, _)) if items.len() == 2 && self.cell_lists.contains(name) && APPEND_METHODS.iter().any(|method| is_word(&items[0], method)) => {
					let item = self.expression(&items[1])?;
					Ok(format!(".set (.loc {}) {} (.append ({}) (.cons ({item}) .nil))", quoted(name), quoted(CELL_FIELD), self.cell_value(name)))
				}
				(Node::Symbol(name), Node::List(items, _, _)) if items.len() == 2 && APPEND_METHODS.iter().any(|method| is_word(&items[0], method)) => {
					let item = self.expression(&items[1])?;
					if !self.list_names.contains(name) {
						return unsupported(node);
					}
					Ok(format!(".assign {} (.append (.glob {}) (.cons ({item}) .nil))", quoted(name), quoted(name)))
				}
				(_, Node::Symbol(field)) if self.is_field(field) => Ok(format!(".get ({}) {}", self.expression(list)?, quoted(field))),
				_ => unsupported(node),
			},
			Node::Key(if_then, Op::Else, otherwise) => match if_then.drop_meta() {
				Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
					Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
						Ok(format!(".ite ({}) ({}) ({})", self.expression(condition)?, self.expression(then)?, self.expression(otherwise)?))
					}
					_ => unsupported(node),
				},
				_ => unsupported(node),
			},
			Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
				Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
					Ok(format!(".ite ({}) ({}) .unit", self.expression(condition)?, self.expression(then)?))
				}
				_ => unsupported(node),
			},
			Node::Key(condition, Op::Do, body) => match condition.drop_meta() {
				Node::Key(empty, Op::While, condition) if empty.is_nothing() => self.breaking_in(None, |exporter| exporter.binary(".loop", condition, body).map(|lean| format!("{lean} {UNIT_TYPE}"))),
				_ => unsupported(node),
			},
			Node::Key(left, Op::Add, right) if is_list_literal(left) || is_list_literal(right) => self.binary(".append", left, right),
			Node::Key(left, Op::Add, right) if !left.is_nothing() => self.binary(".add", left, right),
			Node::Key(left, Op::Sub, right) if !left.is_nothing() => self.binary(".arith .sub", left, right),
			Node::Key(left, Op::Mul, right) => self.binary(".arith .mul", left, right),
			Node::Key(left, Op::Mod, right) => self.binary(".arith .mod", left, right),
			Node::Key(left, Op::Lt | Op::Le, right) => self.binary(".lt", left, right),
			Node::Key(left, Op::Gt | Op::Ge, right) => self.binary(".lt", right, left),
			// `c is Color` parses as `c == Color`: a type test
			Node::Key(left, Op::Eq, right) if matches!(right.drop_meta(), Node::Symbol(class) if self.classes.contains_key(class)) => Ok(format!(".isA ({}) {}", self.expression(left)?, quoted(&right.name()))),
			Node::Key(left, op @ (Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical), right) => {
				let compared = self.binary(if matches!(op, Op::Identical | Op::NotIdentical) { ".eq true" } else { ".eq false" }, left, right)?;
				Ok(if matches!(op, Op::Ne | Op::NotIdentical) { negation(compared) } else { compared })
			}
			Node::Key(list, Op::Hash, index) if !list.is_nothing() => self.binary(".index", list, index),
			Node::Key(empty, Op::Not, operand) if empty.is_nothing() => Ok(negation(self.expression(operand)?)),
			Node::Key(parameter, Op::FatArrow, body) => self.lambda(parameter, body),
			// `a and b` is b when a is truthy, else a; `a or b` the other way round: a, free of effects, evaluated twice
			Node::Key(left, op @ (Op::And | Op::Or), right) if self.is_pure(left) => {
				let (left, right) = (self.expression(left)?, self.expression(right)?);
				Ok(if *op == Op::And { format!(".ite ({left}) ({right}) ({left})") } else { format!(".ite ({left}) ({left}) ({right})") })
			}
			// `'a'..'e'`: letters, outside W0
			Node::Key(from, Op::Range | Op::To, to) if [from, to].iter().any(|bound| matches!(bound.drop_meta(), Node::Text(_) | Node::Char(_))) => unsupported(node),
			Node::Key(from, Op::Range, to) => self.binary(".range", from, to),
			// `1 to 3` includes 3
			Node::Key(from, Op::To, to) => Ok(format!(".range ({}) (.add ({}) (.int 1))", self.expression(from)?, self.expression(to)?)),
			_ => unsupported(node),
		}
	}
}

/// The program as the Lean list of W0 items, or why it is outside W0
pub fn export(code: &str) -> Result<String, String> {
	let program = crate::warp_parser::parse(code);
	let items = Exporter::default().items(&program)?;
	Ok(format!("[{}]", items.join(",\n  ")))
}

/// Runs Lean commands against the built model, from `.lake/<name>.lean` (one file per kind of request: tests run in
/// parallel)
fn ask_model(name: &str, requests: &str) -> Result<String, String> {
	let _one_lake_at_a_time = MODEL_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	build_model()?;
	let file = Path::new(MODEL_DIR).join(".lake").join(format!("{name}.lean"));
	std::fs::write(&file, format!("import WarpTypes\nopen Warp\n\n{requests}")).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("env").arg("lean").arg(&file);
	run_with_timeout(command)
}

/// Builds the model (`lake build` fails on any proof error) and returns its verdict for each exported program
pub fn verdicts(exported: &[String]) -> Result<Vec<ModelVerdict>, String> {
	let requests: String = exported.iter().map(|items| format!("#eval IO.println (verdict {items})\n")).collect();
	let output = ask_model("verdicts", &requests)?;
	let lines: Vec<&str> = output.lines().filter(|line| line.starts_with(ACCEPTED_PREFIX) || *line == REJECTED).collect();
	if lines.len() != exported.len() {
		return Err(format!("the model answered {} of {} programs:\n{output}", lines.len(), exported.len()));
	}
	Ok(lines.iter().map(|line| match line.strip_prefix(ACCEPTED_PREFIX) {
		Some(type_name) => ModelVerdict::Accepted(type_name.to_string()),
		None => ModelVerdict::Rejected,
	}).collect())
}

/// Where warp's run-time admission of a value to a builtin type (analyzer admits) differs from W0's `Ty.sub`, for
/// every type word and W0 scalar value type: "word ← value type: warp admits / W0 sub"
pub fn admits_disagreements() -> Result<Vec<String>, String> {
	let pairs: Vec<(&str, &str, Kind)> = BUILTIN_TYPE_WORDS.iter().flat_map(|word| VALUE_KINDS.iter().map(move |(value, kind)| (*word, *value, *kind))).collect();
	let requests: String = pairs.iter().map(|(word, value, _)| format!("#eval IO.println (Ty.sub ({value}) ({}))\n", type_of_word(word).expect("a W0 type word"))).collect();
	let output = ask_model("admits", &requests)?;
	let answers: Vec<bool> = output.lines().filter_map(|line| line.parse().ok()).collect();
	if answers.len() != pairs.len() {
		return Err(format!("the model answered {} of {} pairs:\n{output}", answers.len(), pairs.len()));
	}
	Ok(pairs.iter().zip(answers).filter_map(|((word, value, kind), sub)| {
		let admits = crate::analyzer::admits(word, *kind);
		(admits != sub).then(|| format!("{word} ← {value}: warp admits {admits} / W0 sub {sub}"))
	}).collect())
}

/// What each exported program gives when the model runs it: `rejected`, `error`, a value as warp prints it, or `?`
/// where the model does not keep the value (numbers other than ints, instances)
pub fn outcomes(exported: &[String]) -> Result<Vec<String>, String> {
	let requests: String = exported.iter().map(|items| format!("#eval IO.println (outcome {items})\n")).collect();
	let output = ask_model("outcomes", &requests)?;
	let lines: Vec<String> = output.lines().map(str::to_string).collect();
	if lines.len() != exported.len() {
		return Err(format!("the model answered {} of {} programs:\n{output}", lines.len(), exported.len()));
	}
	Ok(lines)
}

/// Warp's value of a program, printed as the model's `outcome` prints it (`?` for what the model does not keep)
pub fn warp_value(code: &str) -> String {
	fn shown(value: &Node) -> String {
		match value.drop_meta() {
			Node::Number(Number::Int(n)) => n.to_string(),
			Node::True => "yes".to_string(),
			Node::False => "no".to_string(),
			Node::Text(text) => format!("\"{text}\""),
			Node::Char(c) => format!("\"{c}\""),
			Node::List(items, _, _) => format!("[{}]", items.iter().map(shown).collect::<Vec<_>>().join(" ")),
			Node::Error(_) => "error".to_string(),
			_ => "?".to_string(),
		}
	}
	shown(&crate::pipeline::eval(code))
}

/// The axioms the named theorems rest on, as `#print axioms` lists them (a `sorry` shows as sorryAx)
pub fn axioms(theorems: &[&str]) -> Result<String, String> {
	ask_model("axioms", &theorems.iter().map(|theorem| format!("#print axioms {theorem}\n")).collect::<String>())
}

/// `lake build` of the model: every theorem is checked again
pub fn build_model() -> Result<String, String> {
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("build");
	run_with_timeout(command)
}

/// The proof files of the model
pub fn model_sources() -> Vec<PathBuf> {
	let sources = Path::new(MODEL_DIR).join("WarpTypes");
	let mut files: Vec<PathBuf> = std::fs::read_dir(&sources).map(|entries| entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect()).unwrap_or_default();
	files.retain(|path| path.extension().is_some_and(|extension| extension == "lean"));
	files.sort();
	files
}

/// Warp's own verdict: does it compile the program? An `Err` that is no Error is the constant answer of a program
/// that needs no module, an acceptance
pub fn warp_verdict(code: &str) -> Result<(), String> {
	match crate::pipeline::compile(code) {
		Err(answer) => match answer.drop_meta() {
			Node::Error(why) => Err(why.serialize()),
			_ => Ok(()),
		},
		Ok(_) => Ok(()),
	}
}

/// `warp types <code>`: the program in W0, the model's verdict and warp's
pub fn report(code: &str) -> String {
	let exported = match export(code) {
		Ok(items) => items,
		Err(why) => return why,
	};
	let model = match verdicts(std::slice::from_ref(&exported)) {
		Ok(verdicts) => format!("{:?}", verdicts[0]),
		Err(why) => format!("model failed: {why}"),
	};
	let warp = match warp_verdict(code) {
		Ok(()) => "Accepted".to_string(),
		Err(why) => format!("Rejected: {why}"),
	};
	let value = match outcomes(std::slice::from_ref(&exported)) {
		Ok(values) => values[0].clone(),
		Err(why) => format!("model failed: {why}"),
	};
	format!("W0: {exported}\nmodel: {model}\nmodel value: {value}\nwarp: {warp}\nwarp value: {}", warp_value(code))
}
