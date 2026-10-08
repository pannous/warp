//! The differential check between warp's type checks and W0, the Lean model of warp's type theory whose soundness is
//! proved (notes/type_theory.md, lean/WarpTypes). A warp program in W0's subset is exported as a list of Lean `Item`s
//! (Checker.lean: names, charged names, functions, statements in source order); the model elaborates and checks it
//! and answers `ok <type>` or `rejected`. Warp's own verdict is whether `pipeline::compile` accepts the program.
//! Contract: W0 rejects ⇒ warp rejects; every exception is a known hole with a card.
use super::lean::{lean_executable, run_with_timeout};
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::TRY_MARKER;
use crate::type_kinds::Kind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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
/// `break v` in a block handler ends its block with v (aborting handlers)
const BREAK_KEYWORD: &str = "break";
/// the handler's local holding the payload, `event.level`
const EVENT_LOCAL: &str = "event";
/// an event's payloads are instances of the class `ev·event`, whose fields are every key the program emits it with
const EVENT_CLASS_SUFFIX: &str = "·event";

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
		Some(match crate::type_kinds::canonical_type_name(word) {
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

fn event_class(event: &str) -> String {
	format!("{event}{EVENT_CLASS_SUFFIX}")
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(found) if found == word)
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

/// The parameter's type word: `int` of `y: int`, none for an unannotated `y`
fn parameter_type_word(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, annotation) => Some(annotation.name()),
		_ => None,
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
	/// the event whose block a `break` ends: the innermost block handler's; none in a loop (the loop's break) or a
	/// program-wide handler
	breaking: Vec<Option<String>>,
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
			other => Err(unsupported(other).unwrap_err()),
		}
	}

	/// `class C extends P { f: T … }` and the variants of `type Color = … | rgb(r: int …)` (a variant extends its sum)
	fn collect_classes(&mut self, program: &Node) {
		let own_fields = crate::lowering::class_methods::class_fields(program);
		let mut parents = HashMap::new();
		program.visit(&mut |part| if let Node::Type { name, .. } = part {
			parents.insert(name.drop_meta().name(), name.attribute(crate::wasp_parser::EXTENDS_KEYWORD).map(Node::name));
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
		let fields = self.constructor_fields(class);
		if fields.len() != arguments.len() {
			return Err(format!("{class} takes {} fields, got {}", fields.len(), arguments.len()));
		}
		self.instance(class, fields.into_iter().zip(arguments).collect())
	}

	/// a new instance of class with the given fields written, `let o = new C; o.f1 = a; …; o`
	fn instance(&mut self, class: &str, fields: Vec<(String, &Node)>) -> Lean {
		let path = lean_strings(&self.classes[class].0);
		let local = format!("new·{class}");
		let writes: Result<Vec<String>, String> = fields.iter().map(|(field, value)| Ok(format!(".set (.loc {}) {} ({})", quoted(&local), quoted(field), self.expression(value)?))).collect();
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
		self.collect_classes(program);
		let event_classes = self.collect_event_classes(program);
		let program: Vec<&Node> = statements(program).into_iter().flat_map(type_definitions).collect();
		let mut argument_classes = Vec::new();
		for statement in &program {
			let Some((name, parameters, _)) = function_definition(statement) else { continue };
			if self.functions.contains_key(name) {
				return Err(format!("not in W0: overloads of {name}"));
			}
			let parameter_type = match parameters.as_slice() {
				[] => UNIT_TYPE.to_string(),
				[parameter] => self.parameter_type(parameter)?.1,
				parameters => {
					let class = arguments_class(name);
					let fields = parameters.iter().map(|parameter| Ok((self.parameter_type(parameter)?.0, parameter_type_word(parameter)))).collect::<Result<Vec<_>, String>>()?;
					self.classes.insert(class.clone(), (vec![class.clone()], fields));
					argument_classes.push(class.clone());
					format!(".cls {}", lean_strings(&[class]))
				}
			};
			self.functions.insert(name.to_string(), parameter_type);
		}
		argument_classes.extend(event_classes);
		let mut items = argument_classes.iter().map(|class| self.class_definition(class)).collect::<Result<Vec<_>, String>>()?;
		for statement in program {
			items.push(self.item(statement)?);
		}
		Ok(items)
	}

	fn item(&mut self, statement: &Node) -> Lean {
		if let Some(Effect::On { event, handler, body: None }) = effect(statement) {
			return Ok(format!(".on {} ({})", quoted(&event), self.handler(handler, None)?));
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
			Node::Key(target, Op::Assign, value) if !self.is_bound(target) && !matches!(target.drop_meta(), Node::Key(_, Op::Dot, _)) => self.binding(target, ".var", value),
			other => Ok(format!(".statement ({})", self.expression(other)?)),
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
		if is_list_literal(value) || matches!(value.drop_meta(), Node::Empty) || annotation.as_deref().is_some_and(|lean| lean.starts_with(".list")) {
			self.list_names.push(name.clone());
		}
		let value = self.stored_value(annotation.as_deref(), value)?;
		self.names.insert(name.clone(), annotation.clone());
		let annotation = annotation.map_or("none".to_string(), |lean| format!("(some ({lean}))"));
		Ok(format!(".bind {} {mode} {annotation} ({value}) false", quoted(&name)))
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
			parameters => (arguments_class(name), parameters.iter().map(|parameter| parameter.drop_meta().name()).collect(), declared),
		};
		self.locals.push(parameter.clone());
		self.argument_fields = fields;
		let body = self.block(body);
		self.locals.pop();
		self.argument_fields.clear();
		Ok(format!(".function {} {} {parameter_type} ({})", quoted(name), quoted(&parameter), body?))
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

	fn binary(&mut self, constructor: &str, left: &Node, right: &Node) -> Lean {
		Ok(format!("{constructor} ({}) ({})", self.expression(left)?, self.expression(right)?))
	}

	fn expression(&mut self, node: &Node) -> Lean {
		match node.drop_meta() {
			Node::Number(Number::Int(n)) => Ok(format!(".int ({n})")),
			Node::Number(Number::Float(_) | Number::Quotient(_, _)) => Ok(".num 0".to_string()),
			Node::True => Ok(".bool true".to_string()),
			Node::False => Ok(".bool false".to_string()),
			Node::Text(text) => Ok(format!(".text {}", quoted(text))),
			Node::Char(c) => Ok(format!(".text {}", quoted(&c.to_string()))), // `"a"` parses as a codepoint
			Node::Empty => Ok(".nil".to_string()), // ø is the empty list (`xs = []` parses as ø)
			Node::Symbol(name) if self.argument_fields.contains(name) => {
				Ok(format!(".get (.loc {}) {}", quoted(self.locals.last().expect("the arguments object")), quoted(name)))
			}
			Node::Symbol(name) if self.locals.contains(name) => Ok(format!(".loc {}", quoted(name))),
			// `two()` of a function of no parameters parses as `(two)`
			Node::Symbol(name) if self.functions.get(name).is_some_and(|parameter| parameter == UNIT_TYPE) => Ok(format!(".call {} .unit", quoted(name))),
			Node::Symbol(name) if self.names.contains_key(name) => Ok(format!(".glob {}", quoted(name))),
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
			Node::List(items, _, _) if items.len() == 2 && is_word(&items[0], BREAK_KEYWORD) => self.abort(Some(&items[1])),
			Node::List(items, _, _) if items.first().is_some_and(|class| self.classes.contains_key(&class.name())) => self.construction(&items[0].name(), &items[1..]),
			Node::List(items, _, _) => match items.as_slice() {
				[marker, body, handler] if is_word(marker, TRY_MARKER) => self.binary(".tryCatch", body, handler),
				[call, message] if is_word(call, ERROR_CALL) => match message.drop_meta() {
					Node::Text(message) => Ok(format!(".error {}", quoted(message))),
					_ => unsupported(node),
				},
				[call] if self.functions.get(&call.name()).is_some_and(|parameter| parameter == UNIT_TYPE) => Ok(format!(".call {} .unit", quoted(&call.name()))),
				[_, arguments @ ..] if arguments.iter().any(|argument| matches!(argument.drop_meta(), Node::Key(_, Op::Assign, _))) => Err(format!("not in W0: named arguments in {}", node.serialize().trim())),
				[call, arguments @ ..] if arguments.len() > 1 && self.classes.contains_key(&arguments_class(&call.name())) => {
					let arguments = self.construction(&arguments_class(&call.name()), arguments)?;
					Ok(format!(".call {} ({arguments})", quoted(&call.name())))
				}
				[call, argument] if self.functions.contains_key(&call.name()) => {
					let declared = self.functions[&call.name()].clone();
					let argument = match bool_literal(Some(&declared), argument) {
						Some(yes_or_no) => yes_or_no,
						None => admitted(&declared, self.expression(argument)?),
					};
					Ok(format!(".call {} ({argument})", quoted(&call.name())))
				}
				_ => unsupported(node),
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
				Node::Key(empty, Op::While, condition) if empty.is_nothing() => self.breaking_in(None, |exporter| exporter.binary(".loop", condition, body)),
				_ => unsupported(node),
			},
			Node::Key(left, Op::Add, right) if is_list_literal(left) || is_list_literal(right) => self.binary(".append", left, right),
			Node::Key(left, Op::Add, right) if !left.is_nothing() => self.binary(".add", left, right),
			Node::Key(left, Op::Sub, right) if !left.is_nothing() => self.binary(".arith .sub", left, right),
			Node::Key(left, Op::Mul, right) => self.binary(".arith .mul", left, right),
			Node::Key(left, Op::Lt | Op::Le, right) => self.binary(".lt", left, right),
			Node::Key(left, Op::Gt | Op::Ge, right) => self.binary(".lt", right, left),
			// `c is Color` parses as `c == Color`: a type test
			Node::Key(left, Op::Eq, right) if self.classes.contains_key(&right.name()) => Ok(format!(".isA ({}) {}", self.expression(left)?, quoted(&right.name()))),
			Node::Key(left, Op::Eq | Op::Ne, right) => self.binary(".eq", left, right),
			Node::Key(list, Op::Hash, index) if !list.is_nothing() => self.binary(".index", list, index),
			_ => unsupported(node),
		}
	}
}

/// The program as the Lean list of W0 items, or why it is outside W0
pub fn export(code: &str) -> Result<String, String> {
	let program = crate::wasp_parser::parse(code);
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
