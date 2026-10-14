//! The differential check between warp's type checks and W0, the Lean model of warp's type theory whose soundness is
//! proved (notes/type_theory.md, lean/WarpTypes). A warp program in W0's subset is exported as a list of Lean `Item`s
//! (Checker.lean: names, charged names, functions, statements in source order); the model elaborates and checks it
//! and answers `ok <type>` or `rejected`. Warp's own verdict is whether `pipeline::compile` accepts the program.
//! Contract: W0 rejects ⇒ warp rejects; every exception is a known hole with a card.
use super::lean::{lean_executable, run_with_timeout, LEAN_TIMEOUT};
use crate::analyzer::UNION_JOINER;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::warp_parser::TRY_MARKER;
use crate::words::EXTREMUM_WORDS;
use crate::words::{COUNT_WORD, IN_WORD, LET_WORD, MAP_WORD};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

mod forms;
use forms::*;
mod expressions;
mod model_runs;
pub use model_runs::*;

mod nested_functions;
mod used_modules;

/// The lake project of the model, in the source tree
pub const MODEL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lean/WarpTypes");
/// `lake build` of one project must not run twice at once
static MODEL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
/// The first `lake build` of the model takes minutes (card types-first-run); later ones check it again in seconds
const MODEL_BUILD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(900);
/// What `lake build` leaves once the model is built
const MODEL_BUILD_OUTPUT: &str = ".lake/build";
const ACCEPTED_PREFIX: &str = "ok ";
const REJECTED: &str = "rejected";
const CONSTANT_KEYWORD: &str = "const";
const BOOL_TYPE: &str = ".bool";
const EXACT_TYPE: &str = ".exact";
const ANY_TYPE: &str = ".any";
const LIST_TYPE_PREFIX: &str = ".list ";
/// A value of each W0 scalar type and the kind warp's analyzer sees it as (a bool is an Int; a Float is a decimal
/// written in the program, an exact number: admits cannot tell it from a float, which only a run makes)
const VALUE_KINDS: [(&str, Kind); 5] = [
	(BOOL_TYPE, Kind::Int),
	(".int", Kind::Int),
	(EXACT_TYPE, Kind::Float),
	(".text", Kind::Text),
	(".codepoint", Kind::Codepoint),
];
const UNIT_TYPE: &str = ".unit";
/// the parameter of a function that takes none
const UNIT_PARAMETER: &str = "·";
const ARGUMENTS_SUFFIX: &str = "·args";
const SHARED_KEYWORD: &str = "shared";
/// the one field of a function local's cell (`f·n`): `·` keeps it apart from the program's own fields
const CELL_FIELD: &str = "·value";
/// An inline union `int | text` is the join of its alternatives, every value given to it a cast
const UNION_TYPE: &str = "(Ty.joinAll ";
const RANGED_TYPE_PREFIX: &str = ".ranged ";
const OPTIONAL_MARK: char = '?';
/// An optional `int?`: its values and ø
const OPTIONAL_TYPE_PREFIX: &str = ".opt ";
/// ø is the empty list (`xs = []` parses as ø), but the empty part where an optional takes it
const EMPTY_LIST: &str = ".nil";
const EMPTY_VALUE: &str = ".unit";
const EMIT_KEYWORD: &str = "emit";
/// `global n = 0` at main level, `global n` in a function: a main-level name functions may assign
/// What a global declared without a value holds until it is assigned: `global x; x + 1` is 1
const UNSET_GLOBAL: i64 = 0;
/// `break` is not in W0: block handlers, whose block it ended, were dropped (card cleanup-drop-scoped-handlers)
const BREAK_KEYWORD: &str = "break";
const RETURN_EVENT_SUFFIX: &str = "·return";
/// the handler's local holding the payload, `event.level`
const EVENT_LOCAL: &str = "event";
/// an event's payloads are instances of the class `ev·event`, whose fields are every key the program emits it with
const EVENT_CLASS_SUFFIX: &str = "·event";
const TALLY_CLASS: &str = "·tally";
const TALLY_INDEX: &str = "·index";

/// W0's answer for one program
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelVerdict {
	Accepted(String),
	Rejected,
}

type Lean = Result<String, String>;

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
	/// main-level names bound to a map literal (`m = {a:1}`): `m[k]` reads and writes its key k
	map_names: Vec<String>,
	locals: Vec<String>,
	/// the parameters of the function being exported when it takes several: each is a field of its arguments object
	argument_fields: Vec<String>,
	/// each class's ancestor chain, root first, and its own fields with their type words
	classes: HashMap<String, ClassShape>,
	/// the default values of a function's parameters (`f(a, b=3)`), by its arguments class: a call missing b passes 3
	defaults: HashMap<String, HashMap<String, Node>>,
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
			Some(word) => format!("(some ({}))", self.union_type(word)?),
			None => "none".to_string(),
		})
	}

	/// `int or text`: the join of the alternatives; a union warp narrows to one part (`int | float` is float) is that
	/// part. `int?`: the optional of it
	fn union_type(&self, union: &str) -> Lean {
		if let Some(inner) = union.strip_suffix(OPTIONAL_MARK) {
			return Ok(format!("{OPTIONAL_TYPE_PREFIX}({})", self.union_type(inner)?));
		}
		let alternatives =
			union.split(UNION_JOINER).map(|part| self.type_of(part)).collect::<Result<Vec<_>, String>>()?;
		Ok(match alternatives.as_slice() {
			[single] => single.clone(),
			_ => format!("{UNION_TYPE}[{}])", alternatives.join(", ")),
		})
	}

	fn annotation_type(&self, annotation: &Node) -> Lean {
		if let Some(union) = crate::analyzer::union_of_types(annotation, &|word| self.classes.contains_key(word)) {
			return self.union_type(&union);
		}
		match annotation.drop_meta() {
			Node::Symbol(word) if word.ends_with(OPTIONAL_MARK) => self.union_type(word),
			Node::Symbol(word) => self.type_of(word),
			// `[int]`: a list of int
			Node::List(items, Bracket::Square, _) if items.len() == 1 => {
				Ok(format!("{LIST_TYPE_PREFIX}({})", self.annotation_type(&items[0])?))
			}
			other => outside_w0(other),
		}
	}

	/// `y`, `y: int`: the parameter's name and W0 type
	fn parameter_type(&self, parameter: &Node) -> Result<(String, String), String> {
		match parameter.drop_meta() {
			Node::Symbol(parameter) => Ok((parameter.clone(), ANY_TYPE.to_string())),
			Node::Key(parameter, Op::Colon, annotation) => Ok((parameter.name(), self.annotation_type(annotation)?)),
			Node::Key(parameter, Op::Assign, _) => self.parameter_type(parameter),
			other => Err(outside_w0(other).unwrap_err()),
		}
	}

	/// `class C extends P { f: T … }` and the variants of `type Color = … | rgb(r: int …)` (a variant extends its sum)
	fn collect_classes(&mut self, program: &Node) {
		let own_fields = crate::lowering::class_methods::class_fields(program);
		let mut parents = HashMap::new();
		program.visit(&mut |part| {
			if let Node::Type { name, .. } = part {
				parents.insert(
					name.drop_meta().name(),
					name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(Node::name),
				);
			}
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
			let fields = own_fields
				.get(class)
				.cloned()
				.unwrap_or_default()
				.into_iter()
				.map(|(field, word)| (field, (!word.is_empty()).then_some(word)))
				.collect();
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
		let named: Vec<Option<(String, Node)>> = arguments
			.iter()
			.map(|argument| match argument.drop_meta() {
				Node::Key(name, Op::Assign, value) => Some((name.name(), value.as_ref().clone())),
				_ => None,
			})
			.collect();
		if let Some((unknown, _)) = named.iter().flatten().find(|(name, _)| !fields.contains(name)) {
			return Err(format!("{class} has no field {unknown}"));
		}
		let named_fields: Vec<String> = named.iter().flatten().map(|(name, _)| name.clone()).collect();
		let mut unnamed = fields.iter().filter(|field| !named_fields.contains(field));
		let written: Option<Vec<(String, Node)>> = arguments
			.iter()
			.zip(named)
			.map(|(argument, named)| named.or_else(|| unnamed.next().map(|field| (field.clone(), argument.clone()))))
			.collect();
		let mut written =
			written.ok_or_else(|| format!("{class} takes {} fields, got {}", fields.len(), arguments.len()))?;
		let defaults = self.defaults.get(class);
		for field in unnamed {
			let default = defaults
				.and_then(|defaults| defaults.get(field))
				.ok_or_else(|| format!("{class} needs a value for {field}"))?;
			written.push((field.clone(), default.clone()));
		}
		Ok(written)
	}

	/// a new instance of class with the given fields written, `let o = new C; o.f1 = a; …; o`
	fn instance(&mut self, class: &str, fields: Vec<(String, &Node)>) -> Lean {
		let path = lean_strings(&self.classes[class].0);
		let local = format!("new·{class}");
		let writes: Result<Vec<String>, String> = fields
			.iter()
			.map(|(field, value)| {
				let field_type =
					self.classes[class].1.iter().find(|(own, _)| own == field).and_then(|(_, word)| word.clone());
				let value = self.argument(field_type.as_deref(), value)?;
				Ok(format!(
					".set (.loc {}) {} ({})",
					quoted(&local),
					quoted(field),
					self.admitted_in_field(field, value)?
				))
			})
			.collect();
		let body = writes?
			.iter()
			.rev()
			.fold(format!(".loc {}", quoted(&local)), |rest, write| format!(".seq ({write}) ({rest})"));
		Ok(format!(".letIn {} (.cls {path}) (.new {path}) ({body})", quoted(&local)))
	}

	/// a value written to a field: a union field (`size: int | text`) checks it when it runs. W0 knows no object's
	/// class here, so the cast needs every class with a field of that name to declare it alike
	fn admitted_in_field(&self, field: &str, value: String) -> Lean {
		let declared: Vec<&Option<String>> = self
			.classes
			.values()
			.flat_map(|(_, fields)| fields.iter().filter(|(own, _)| own == field).map(|(_, word)| word))
			.collect();
		match declared.split_first() {
			Some((Some(word), others)) if others.iter().all(|other| other.as_ref() == Some(word)) => {
				Ok(admitted(&self.union_type(word)?, value))
			}
			_ => Ok(value),
		}
	}

	/// `ev·event` with every key the program emits ev with
	fn collect_event_classes(&mut self, program: &Node) -> Vec<String> {
		let mut keys: Vec<(String, Vec<String>)> = vec![];
		program.visit(&mut |part| {
			if let Some(Effect::Emit { event, payload }) = effect(part) {
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
			}
		});
		keys.retain(|(_, fields)| !fields.is_empty());
		for (class, fields) in &keys {
			self.classes.insert(
				class.clone(),
				(vec![class.clone()], fields.iter().map(|field| (field.clone(), None)).collect()),
			);
		}
		keys.into_iter().map(|(class, _)| class).collect()
	}

	/// a handler body: `event` is its payload
	fn handler(&mut self, handler: &Node) -> Lean {
		self.locals.push(EVENT_LOCAL.to_string());
		let handler = self.block(handler);
		self.locals.pop();
		handler
	}

	fn effect(&mut self, effect: Effect) -> Lean {
		match effect {
			Effect::On { event, .. } => {
				Err(format!("not in W0: a program-wide handler of {event} inside an expression"))
			}
			Effect::Emit { event, payload } if payload.is_empty() => Ok(format!(".emit {} .unit", quoted(&event))),
			Effect::Emit { event, payload } => {
				Ok(format!(".emit {} ({})", quoted(&event), self.instance(&event_class(&event), payload)?))
			}
		}
	}

	fn class_definition(&self, class: &str) -> Lean {
		let fields: Result<Vec<String>, String> = self.classes[class]
			.1
			.iter()
			.map(|(field, type_word)| Ok(format!("({}, {})", quoted(field), self.optional_type(type_word.as_deref())?)))
			.collect();
		Ok(format!(".classDef {} [{}]", quoted(class), fields?.join(", ")))
	}

	fn items(&mut self, program: &Node) -> Result<Vec<String>, String> {
		let with_modules = used_modules::inline(program)?;
		let program = &with_modules;
		self.collect_classes(program);
		program.visit(&mut |part| {
			if let Some((name, _)) = global_declaration(part) {
				self.globals.push(name.to_string());
			}
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
		let mut declared_cells = vec![];
		for statement in &program {
			let Some(definition) = function_definition(statement) else {
				continue;
			};
			for local in assigned_locals(definition.body, &self.globals) {
				let class = cell_class(definition.name, &local);
				self.classes.insert(class.clone(), (vec![class.clone()], vec![(CELL_FIELD.to_string(), None)]));
				declared_cells.push((
					class.clone(),
					definition
						.parameters()
						.iter()
						.find(|parameter| parameter_word(parameter) == local)
						.and_then(|parameter| parameter_type_word(parameter)),
				));
				self.cell_classes.push(class);
			}
		}
		// an annotated parameter's cell keeps its type (P203); a cell parameter has none of its own
		for (class, declared) in declared_cells {
			let declared = declared.filter(|word| !self.cell_classes.contains(word));
			cell_items.push(format!(".cell {} {}", quoted(&class), self.optional_type(declared.as_deref())?));
		}
		for statement in &program {
			let Some(definition) = function_definition(statement) else {
				continue;
			};
			let (name, parameters) = (definition.name, definition.parameters());
			if self.functions.contains_key(name) {
				return Err(format!("not in W0: overloads of {name}"));
			}
			let parameter_type = match parameters.as_slice() {
				[] => UNIT_TYPE.to_string(),
				[parameter] => {
					self.single_parameters.insert(name.to_string(), parameter_word(parameter));
					self.parameter_type(parameter)?.1
				}
				parameters => {
					let class = arguments_class(name);
					let fields = parameters
						.iter()
						.map(|parameter| Ok((self.parameter_type(parameter)?.0, parameter_type_word(parameter))))
						.collect::<Result<Vec<_>, String>>()?;
					let defaults = parameters.iter().filter_map(|parameter| match parameter.drop_meta() {
						Node::Key(_, Op::Assign, default) => {
							Some((parameter_word(parameter), default.as_ref().clone()))
						}
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
		program_node.visit(&mut |part| {
			words_used |=
				[COUNT_WORD, IN_WORD].iter().chain(&crate::analyzer::COUNTING_WORDS).any(|word| part.is_symbol(word))
		});
		if words_used {
			let fields = [CELL_FIELD, TALLY_INDEX].map(|field| (field.to_string(), Some("int".to_string())));
			self.classes.insert(TALLY_CLASS.to_string(), (vec![TALLY_CLASS.to_string()], fields.to_vec()));
			argument_classes.push(TALLY_CLASS.to_string());
		}
		if let Some(keys) = map_keys(program_node) {
			self.classes.insert(
				MAP_WORD.to_string(),
				(vec![MAP_WORD.to_string()], keys.into_iter().map(|key| (key, None)).collect()),
			);
			argument_classes.push(MAP_WORD.to_string());
		}
		let mut items =
			argument_classes.iter().map(|class| self.class_definition(class)).collect::<Result<Vec<_>, String>>()?;
		items.extend(cell_items);
		for (position, statement) in program.iter().enumerate() {
			items.push(match global_declaration(statement) {
				// `global x; x + 1`: read before any value, x is warp's unset global 0
				Some((name, None)) if !assigned_before_read(&program[position + 1..], name) => {
					self.binding(&Node::Symbol(name.to_string()), ".var", &Node::int(UNSET_GLOBAL))?
				}
				_ => self.item(statement)?,
			});
		}
		Ok(items)
	}

	fn item(&mut self, statement: &Node) -> Lean {
		if let Some(Effect::On { event, handler }) = effect(statement) {
			return Ok(format!(".on {} ({})", quoted(&event), self.handler(handler)?));
		}
		match global_declaration(statement) {
			Some((_, Some((target, value)))) => return self.binding(target, ".var", value),
			// `global x; x = 7`: the later first binding is the global one
			Some((_, None)) => return Ok(format!(".statement {UNIT_TYPE}")),
			None => {}
		}
		match statement.drop_meta() {
			Node::Type { name, .. } => self.class_definition(&name.drop_meta().name()),
			_ if let Some(definition) = function_definition(statement) => self.function(&definition),
			Node::Key(head, Op::Define, body) => match head.drop_meta() {
				Node::Symbol(name) => {
					self.names.insert(name.clone(), None);
					Ok(format!(".charged {} ({})", quoted(name), self.expression(body)?))
				}
				_ => outside_w0(statement),
			},
			Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol(CONSTANT_KEYWORD) => {
				match items[1].drop_meta() {
					Node::Key(target, Op::Assign, value) => self.binding(target, ".const", value),
					other => outside_w0(other),
				}
			}
			Node::List(items, Bracket::None, Separator::Colon) if destructuring_assignment(items).is_some() => {
				let (names, values) = destructuring_assignment(items).expect("a destructuring");
				match values[..] {
					[list] => self.destructure(&names, list),
					_ => self.assign_tuple(&names, &values),
				}
			}
			// `let x = 1`: a constant; `shared n = 5` (one thread in W0): a variable; `int i = 2`: `i: int = 2`
			Node::List(items, _, _)
				if items.len() == 2
					&& matches!(items[1].drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(_))) =>
			{
				let Node::Key(target, _, value) = items[1].drop_meta() else { unreachable!("an assignment") };
				match items[0].drop_meta() {
					Node::Symbol(word) if word == LET_WORD => self.binding(target, ".const", value),
					Node::Symbol(word) if word == SHARED_KEYWORD => self.binding(target, ".var", value),
					Node::Symbol(word) if self.type_of(word).is_ok() => {
						self.binding(&Node::Key(target.clone(), Op::Colon, Box::new(items[0].clone())), ".var", value)
					}
					_ => Ok(format!(".statement ({})", self.expression(statement)?)),
				}
			}
			// `a = b = 3`: `b = 3`, then `a = b`
			Node::Key(target, Op::Assign, value) if matches!(value.drop_meta(), Node::Key(_, Op::Assign, _)) => {
				let Node::Key(inner_target, _, _) = value.drop_meta() else { unreachable!("an assignment") };
				Ok(format!(
					"{},\n  {}",
					self.item(value)?,
					self.item(&Node::Key(target.clone(), Op::Assign, inner_target.clone()))?
				))
			}
			Node::Key(target, Op::Assign, value)
				if !self.is_bound(target) && !matches!(target.drop_meta(), Node::Key(_, Op::Dot | Op::Hash, _)) =>
			{
				self.binding(target, ".var", value)
			}
			other => Ok(format!(".statement ({})", self.expression(other)?)),
		}
	}

	/// `a, b = xs`: the hidden `·tupleN = xs`, its items counted (warp fails on a wrong number of values), then
	/// `a = ·tupleN#1`, `b = ·tupleN#2`; several items in one
	fn destructure(&mut self, names: &[String], list: &Node) -> Lean {
		if let Some(bound) = names.iter().find(|name| self.names.contains_key(*name)) {
			return Err(format!(
				"not in W0: destructuring into {bound}, which holds a value (card destructure-existing)"
			));
		}
		self.hidden_names += 1;
		let (tuple, count) = (format!("·tuple{}", self.hidden_names), format!("·count{}", self.hidden_names));
		let mut items = vec![
			format!(".bind {} .var none ({}) false", quoted(&tuple), self.expression(list)?),
			format!(".bind {} .var none (.int 0) false", quoted(&count)),
			format!(
				".statement (.forIn \"·item\" (.glob {}) (.assign {} (.add (.glob {}) (.int 1))) {UNIT_TYPE})",
				quoted(&tuple),
				quoted(&count),
				quoted(&count)
			),
			format!(
				".statement (.ite (.eq false (.glob {}) (.int {})) .unit (.error \"wrong number of values\"))",
				quoted(&count),
				names.len()
			),
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
			return Err(format!(
				"not in W0: {} values for {} names (warp counts them when it compiles)",
				values.len(),
				names.len()
			));
		}
		self.hidden_names += 1;
		let hidden: Vec<String> =
			(1..=values.len()).map(|position| format!("·value{}_{position}", self.hidden_names)).collect();
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
			Some(declared) => format!(
				".statement (.assign {} ({}))",
				quoted(name),
				declared.map_or(value.clone(), |declared| admitted(&declared, value))
			),
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
			other => return outside_w0(other),
		};
		if annotation.is_none() && is_evident_bool(value) {
			self.bool_names.push(name.clone());
		}
		if matches!(unwrapped(value), Node::Key(_, Op::Arrow, _)) {
			self.lambda_names.push(name.clone());
		}
		if map_entries(value).is_some() {
			self.map_names.push(name.clone());
		}
		let aliased_list = matches!(value.drop_meta(), Node::Symbol(other) if self.list_names.contains(other));
		let optional = annotation.as_deref().is_some_and(|lean| lean.starts_with(OPTIONAL_TYPE_PREFIX));
		if aliased_list
			|| is_list_literal(value)
			|| (value.is_empty_node() && !optional)
			|| annotation.as_deref().is_some_and(|lean| lean.starts_with(LIST_TYPE_PREFIX))
		{
			self.list_names.push(name.clone());
		}
		let value = self.shared_list(&name, annotation.as_deref(), value)?;
		self.names.insert(name.clone(), annotation.clone());
		let annotation = annotation.map_or("none".to_string(), |lean| format!("(some ({lean}))"));
		Ok(format!(".bind {} {mode} {annotation} ({value}) {}", quoted(&name), self.globals.contains(&name)))
	}

	/// A list stored in a list name is shared (P215): a new list, tagged with the declared element type (anything when
	/// undeclared), which each `add` checks when it runs; a name read (`ys = xs`) stores the same list
	fn shared_list(&mut self, name: &str, declared: Option<&str>, value: &Node) -> Lean {
		let lean = self.stored_value(declared, value)?;
		if !self.list_names.iter().any(|list| list == name) || matches!(value.drop_meta(), Node::Symbol(_)) {
			return Ok(lean);
		}
		let tag = declared.and_then(|declared| declared.strip_prefix(LIST_TYPE_PREFIX)).unwrap_or(ANY_TYPE);
		Ok(format!(".share ({lean}) {tag}"))
	}

	/// A call's result stored in a declared list is checked item by item at run time (list-element-types): a cast
	fn stored_value(&mut self, declared: Option<&str>, value: &Node) -> Lean {
		if let Some(literal) = place_literal(declared, value) {
			return Ok(literal);
		}
		let lean = self.expression(value)?;
		Ok(match declared {
			Some(declared) if declared.starts_with(".list") && self.is_call(value) => {
				format!(".cast ({lean}) [{declared}]")
			}
			Some(declared) => admitted(declared, lean),
			None => lean,
		})
	}

	/// a field some class declares: other words after a dot are methods (`x.size`, `4.square`), outside W0
	fn is_field(&self, word: &str) -> bool {
		self.classes.values().any(|(_, fields)| fields.iter().any(|(field, _)| field == word))
	}

	fn is_call(&self, node: &Node) -> bool {
		node.call_parts().is_some_and(|(name, _)| self.functions.contains_key(name))
	}
}
