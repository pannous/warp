//! Reading warp forms for the export: literals and type words, `on`/`emit` effects, definitions and their heads, locals and late binding
use super::*;
pub(super) use crate::filled_lists::is_list_literal;
pub(super) use crate::lowering::nodes::parameter_word;
use crate::lowering::{function_head, DefinitionForm, FunctionHead};
pub(super) use crate::text::injection::literal_text;
use crate::words::{ON_WORD, RETURN_WORD};

pub(super) fn outside_w0(node: &Node) -> Lean {
	Err(format!("not in W0: {}", node.serialize().trim()))
}

/// `0.5`, `7/2`: an exact number, an int when it is whole (Semantics.lean exactValue)
pub(super) fn exact_number(numerator: impl std::fmt::Display, denominator: impl std::fmt::Display) -> String {
	format!("(Warp.exactValue (({numerator} : Rat) / {denominator}))")
}

pub(super) fn quoted(text: &str) -> String {
	format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `int`, `texts`, `list`, `int16` (the range of ints it holds): the W0 type a warp type word names
pub(super) fn type_of_word(word: &str) -> Option<String> {
	let scalar = |word: &str| -> Option<String> {
		if let Some(width) = crate::numbers::fixed_width::fixed_width(word) {
			return Some(format!("{RANGED_TYPE_PREFIX}({}) ({})", width.low, width.high));
		}
		let word = word.to_lowercase();
		Some(
			match crate::type_tests::canonical_spec_word(&word) {
				"int" => ".int",
				"text" => ".text",
				"codepoint" => ".codepoint",
				crate::analyzer::BOOL_TYPE => BOOL_TYPE,
				"any" => ANY_TYPE,
				exact if crate::type_tests::is_exact_fraction_type(exact) => EXACT_TYPE,
				float if crate::type_tests::is_float_type_word(float) => ".float",
				number if crate::type_tests::type_matches(number, "number") => ".number",
				_ => return None,
			}
			.to_string(),
		)
	};
	if word == "list" {
		return Some(".list .any".to_string());
	}
	match scalar(word) {
		Some(lean) => Some(lean),
		None => word.strip_suffix('s').and_then(scalar).map(|element| format!(".list ({element})")),
	}
}

/// `[.int, .text]` of the union type `(Ty.joinAll [.int, .text])`
pub(super) fn union_alternatives(declared: &str) -> Option<&str> {
	declared.strip_prefix(UNION_TYPE)?.strip_suffix(')')
}

/// a value given to a declared place: a union checks it against its alternatives when it runs, a fixed width
/// against its range (an int literal is an `int`, not below the range: `x: int16 = 5`)
pub(super) fn admitted(declared: &str, value: String) -> String {
	match union_alternatives(declared) {
		Some(alternatives) => format!(".cast ({value}) {alternatives}"),
		None if declared.starts_with(RANGED_TYPE_PREFIX) => format!(".cast ({value}) [{declared}]"),
		None if declared.starts_with(OPTIONAL_TYPE_PREFIX) && value == EMPTY_LIST => EMPTY_VALUE.to_string(),
		None => value,
	}
}

/// `on`/`emit` forms; an event is named by one or more words (`on stop the machine {…}`)
pub(super) enum Effect<'a> {
	On { event: String, handler: &'a Node },
	Emit { event: String, payload: Vec<(String, &'a Node)> },
}

pub(super) fn effect(node: &Node) -> Option<Effect<'_>> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let (first, rest) = items.split_first()?;
	if first.is_symbol(ON_WORD) {
		let (last, words) = rest.split_last()?;
		let event = crate::event_signals::event_name(words.iter())?;
		return last.is_block().then_some(Effect::On { event, handler: last });
	}
	if !first.is_symbol(EMIT_KEYWORD) {
		return None;
	}
	// `emit alarm{level: 3}` parses as `alarm: {level: 3}` after the event's other words
	if let Some((last, words)) = rest.split_last() {
		if let Node::Key(word, Op::Colon, fields) = last.drop_meta() {
			let Node::List(fields, Bracket::Curly, _) = fields.drop_meta() else {
				return None;
			};
			let payload: Option<Vec<(String, &Node)>> = fields
				.iter()
				.map(|field| match field.drop_meta() {
					Node::Key(key, Op::Colon, value) => Some((key.name(), &**value)),
					_ => None,
				})
				.collect();
			let mut words = words.to_vec();
			words.push((**word).clone());
			return Some(Effect::Emit { event: crate::event_signals::event_name(words.iter())?, payload: payload? });
		}
	}
	Some(Effect::Emit { event: crate::event_signals::event_name(rest.iter())?, payload: vec![] })
}

/// `global n`, `global n = 0` or `global float y = 2` (parsed `global: n`, `global: (n = 0)`, `global: (y: float = 2)`):
/// the name, and the assignment's target and value
pub(super) fn global_declaration(node: &Node) -> Option<(&str, Option<(&Node, &Node)>)> {
	let Node::Key(keyword, Op::Colon, declared) = node.drop_meta() else {
		return None;
	};
	if !keyword.is_symbol(crate::words::GLOBAL_WORD) {
		return None;
	}
	match declared.drop_meta() {
		Node::Symbol(name) => Some((name, None)),
		Node::Key(target, Op::Assign, value) => match target.drop_meta() {
			Node::Symbol(name) => Some((name, Some((&**target, &**value)))),
			Node::Key(name, Op::Colon, _) => match name.drop_meta() {
				Node::Symbol(name) => Some((name, Some((&**target, &**value)))),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

pub(super) fn event_class(event: &str) -> String {
	format!("{event}{EVENT_CLASS_SUFFIX}")
}

/// `{a:1 b:2}`: its keys and values
pub(super) fn map_entries(node: &Node) -> Option<Vec<(String, &Node)>> {
	let Node::List(items, Bracket::Curly, _) = node.drop_meta() else {
		return None;
	};
	items
		.iter()
		.map(|item| match item.drop_meta() {
			Node::Key(key, Op::Colon, value) if matches!(key.drop_meta(), Node::Symbol(_)) => {
				Some((key.name(), value.as_ref()))
			}
			_ => None,
		})
		.collect()
}

/// `k` of `k+1`: the parser writes `m[k]` as `m#(k+1)`, a list's 1-based index
pub(super) fn bracket_index(index: &Node) -> Option<&Node> {
	match index.drop_meta() {
		Node::Key(key, Op::Add, one) if matches!(one.drop_meta(), Node::Number(Number::Int(1))) => Some(key),
		_ => None,
	}
}

/// every key a map literal, a field write `x.key = v` or a write `x["key"] = v` names, when the program has a map
/// literal
pub(super) fn map_keys(program: &Node) -> Option<Vec<String>> {
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
				Node::Key(_, Op::Hash, index) => bracket_index(index).and_then(literal_text).into_iter().collect(),
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

/// the value of `return v`, ø of a bare `return`
pub(super) fn returned(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.first().is_some_and(|first| first.is_symbol(RETURN_WORD)) => {
			Some(match &items[1..] {
				[] => Node::Empty,
				[value] => value.clone(),
				words => Node::List(words.to_vec(), Bracket::None, Separator::Space),
			})
		}
		_ => None,
	}
}

pub(super) fn returns(body: &Node) -> bool {
	let mut found = false;
	body.visit(&mut |part| found |= returned(part).is_some());
	found
}

/// A function body ending in `return v` ends in v
pub(super) fn without_tail_return(body: &Node) -> Node {
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

/// The statements of a program or block: an unbracketed `;`/newline list, else the one node
pub(super) fn statements(node: &Node) -> Vec<&Node> {
	node.drop_meta().sequence_statements().iter().collect()
}

/// A sum type `type Color = red | rgb(…)` parses as the list of its type definitions (the sum, then each variant)
pub(super) fn type_definitions(statement: &Node) -> Vec<&Node> {
	match statement.drop_meta() {
		Node::List(items, Bracket::None, _)
			if items.iter().all(|item| matches!(item.drop_meta(), Node::Type { .. })) =>
		{
			items.iter().collect()
		}
		other => vec![other],
	}
}

/// The definitions the export reads, with their declared result type (warp converts the result to it,
/// lowering/definitions/declarations.rs): `f(x) := body`, `f(x): int := body`, `f(x) as int := body`, `int f(x) := body`,
/// `def f(x) { body }`, `def f(x) -> int { body }`, `fun f(x): Int { body }`, `def f(x) -> int: body`,
/// `def f(x) as int = body`
pub(super) fn function_definition(statement: &Node) -> Option<FunctionHead<'_>> {
	// C style: the result type before the name
	if let Node::List(items, _, _) = statement.drop_meta() {
		if let [result, definition] = items.as_slice() {
			if type_of_word(&result.name()).is_some() {
				let definition = function_head(definition).filter(|definition| {
					definition.result.is_none()
						&& (definition.form == DefinitionForm::Operator(Op::Assign) || is_exported(definition))
				})?;
				return Some(FunctionHead { result: Some(result), ..definition });
			}
		}
	}
	function_head(statement).filter(is_exported)
}

/// `f(x) = body` is an assignment here; `def f(x) := body`, `def f(x): body` without a result type and
/// `def (f x) {…}` are not read yet
fn is_exported(definition: &FunctionHead) -> bool {
	match definition.form {
		DefinitionForm::Operator(op) => op == Op::Define,
		DefinitionForm::KeywordBlock => true,
		DefinitionForm::KeywordSpaced => false,
		DefinitionForm::Keyword(op) => op != Op::Define && definition.result.is_some(),
	}
}

/// `a, b = xs` and `a, b = 1, 2`: the names and the values after `=` (one: a list to take apart)
pub(super) fn destructuring_assignment(items: &[Node]) -> Option<(Vec<String>, Vec<&Node>)> {
	let assignment = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _)))?;
	let Node::Key(target, Op::Assign, value) = items[assignment].drop_meta() else {
		return None;
	};
	let names: Option<Vec<String>> = items[..assignment]
		.iter()
		.chain(std::iter::once(&**target))
		.map(|name| match name.drop_meta() {
			// `*rest` takes the items between: outside W0
			Node::Symbol(name) if !name.starts_with('*') => Some(name.clone()),
			_ => None,
		})
		.collect();
	Some((names?, std::iter::once(&**value).chain(&items[assignment + 1..]).collect()))
}

/// A function or lambda bound at main level: its name, parameters and body
pub(super) fn callable_definition(statement: &Node) -> Option<(&str, Vec<String>, &Node)> {
	if let Some(definition) = function_definition(statement) {
		let parameters = definition.parameters().iter().map(|parameter| parameter_word(parameter)).collect();
		return Some((definition.name, parameters, definition.body));
	}
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else {
		return None;
	};
	let (Node::Symbol(name), Node::Key(parameter, Op::Arrow, body)) = (target.drop_meta(), value.ungrouped()) else {
		return None;
	};
	Some((name, vec![parameter.name()], &**body))
}

/// `global x; x = 7`: the first statement naming x assigns it (or none names it), so that is the global's binding
pub(super) fn assigned_before_read(statements: &[&Node], name: &str) -> bool {
	match statements.iter().find(|statement| statement.mentions(name)).map(|statement| statement.drop_meta()) {
		Some(Node::Key(target, Op::Assign, value)) => target.is_symbol(name) && !value.mentions(name),
		Some(_) => false,
		None => true,
	}
}

/// Warp's late binding (src/lowering/definitions/late_binding.rs): a function or lambda reading a main-level name the program
/// changes after its definition, with a call after that change, needs `global x`. W0 reads at call time, which gives
/// what warp gives for every program warp accepts, so such programs are outside W0 rather than rejected by it
pub(super) fn late_binding(statements: &[&Node], globals: &[String]) -> Option<String> {
	for (position, statement) in statements.iter().enumerate() {
		let Some((name, parameters, body)) = callable_definition(statement) else {
			continue;
		};
		for (offset, later) in statements[position + 1..].iter().enumerate() {
			let Node::Key(target, op, _) = later.drop_meta() else {
				continue;
			};
			let Node::Symbol(changed) = target.drop_meta() else {
				continue;
			};
			let read = body.mentions(changed) && !parameters.contains(changed) && !globals.contains(changed);
			if (*op == Op::Assign || op.is_compound_assign())
				&& read && statements[position + offset + 2..].iter().any(|call| call.mentions(name))
			{
				return Some(format!(
					"not in W0: {name} reads {changed}, which changes after {name} is defined and before a call (warp asks for `global {changed}`, late binding)"
				));
			}
		}
	}
	None
}

/// yes when e is truthy
pub(super) fn truthiness(lean: String) -> String {
	format!(".ite ({lean}) (.bool true) (.bool false)")
}

/// `not e`: yes when e is falsy
pub(super) fn negation(lean: String) -> String {
	format!(".ite ({lean}) (.bool false) (.bool true)")
}

/// The class of the cell a local of function holds its value in: `f·n` for n in f
pub(super) fn cell_class(function: &str, local: &str) -> String {
	format!("{function}·{local}")
}

/// The names a function body assigns (`n = 0`, `n += 1`, a parameter too), in order of first assignment, but for the
/// given globals: its locals (functions2)
pub(super) fn assigned_locals(body: &Node, globals: &[String]) -> Vec<String> {
	let mut locals: Vec<String> = vec![];
	body.visit(&mut |part| {
		if let Node::Key(target, op, _) = part {
			match target.drop_meta() {
				Node::Symbol(name)
					if (*op == Op::Assign || op.is_compound_assign())
						&& !globals.contains(name)
						&& !locals.contains(name) =>
				{
					locals.push(name.clone())
				}
				_ => {}
			}
		}
	});
	locals
}

/// Whether the first value a function body gives local is a list (`out = []`, `out = [1]`)
pub(super) fn first_value_is_list(body: &Node, local: &str) -> bool {
	let mut first = None;
	body.visit(&mut |part| match part {
		Node::Key(target, Op::Assign, value)
			if first.is_none() && matches!(target.drop_meta(), Node::Symbol(name) if name == local) =>
		{
			first = Some(is_list_literal(value) || value.is_empty_node());
		}
		_ => {}
	});
	first.unwrap_or(false)
}

/// The parameter's type word: `int` of `y: int`, none for an unannotated `y`
pub(super) fn parameter_type_word(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, annotation) => Some(crate::lowering::class_methods::type_word(annotation)),
		Node::Key(parameter, Op::Assign, _) => parameter_type_word(parameter),
		_ => None,
	}
}

/// The class a function of several parameters takes them as: `f(a, b)` takes one `f·args` with fields a and b
pub(super) fn arguments_class(function: &str) -> String {
	format!("{function}{ARGUMENTS_SUFFIX}")
}

/// A bool place (a declared variable or parameter) takes the literals 1 and 0 as yes and no (P199); an exact place
/// takes a decimal as the fraction it spells (`x: rational = 0.5`, as warp's emit_declared_value)
pub(super) fn place_literal(declared: Option<&str>, value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::Number(Number::Int(n @ (0 | 1))) if declared == Some(BOOL_TYPE) => Some(format!(".bool {}", *n == 1)),
		Node::Number(Number::Float(decimal)) if declared == Some(EXACT_TYPE) && decimal.is_finite() => {
			let (numerator, denominator) = crate::wasm_emitter::numbers::exact::decimal_fraction(*decimal);
			Some(exact_number(numerator, denominator))
		}
		_ => None,
	}
}

/// One `unit` as a W0 quantity: its number of smallest steps of its base dimension
pub(super) fn quantity_literal(unit: &str) -> String {
	let (dimension, factor) = crate::numbers::units::dimension_and_factor(unit).expect("a unit");
	format!(".qty {factor} [({}, 1)]", quoted(&dimension))
}

pub(super) fn is_evident_bool(value: &Node) -> bool {
	matches!(
		value.drop_meta(),
		Node::True | Node::False | Node::Key(_, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq | Op::Ne, _)
	)
}

pub(super) fn lean_strings(words: &[String]) -> String {
	format!("[{}]", words.iter().map(|word| quoted(word)).collect::<Vec<_>>().join(", "))
}
