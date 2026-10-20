//! Kind inference: the kind of a node, of arithmetic, of list elements and branches

use super::*;
use crate::words::{ANY_TYPE, MAP_WORD, NUMBER_WORD, REAL_WORD};

/// A statement changing a variable it reads: `x=x+1`, `x+=1`, `x++` (data like `{x=1}` reads nothing)
pub(super) fn is_update(node: &Node) -> bool {
	match node {
		Node::Key(target, Op::Assign, value) => {
			target.symbol_name().is_some_and(|name| crate::warp_parser::mentions(value, name))
		}
		Node::Key(target, op, _) if matches!(op, Op::Inc | Op::Dec) || op.is_compound_assign() => {
			matches!(target.drop_meta(), Node::Symbol(_))
		}
		_ => false,
	}
}

/// A value whose kind is known only at run time, held as a Node: maybe a number, maybe a ± interval
pub fn is_run_time_kind(kind: &Kind) -> bool {
	matches!(kind, Kind::Data | Kind::Empty)
}

/// The runtime function of `a op b` when an operand's kind is known only at run time (a field of a map parameter, an
/// element of a parsed JSON value): Int or Float is decided by the values (wasm_emitter list_ops NODE_ARITHMETIC)
pub fn node_arithmetic(left: Kind, op: &Op, right: Kind) -> Option<&'static str> {
	let numeric = |kind: &Kind| matches!(kind, Kind::Int | Kind::Float) || is_run_time_kind(kind);
	// a value of run-time kind added to a list: node_add concatenates when it is a list too
	if *op == Op::Add && [left, right].contains(&Kind::Data) && [left, right].contains(&Kind::List) {
		return Some(crate::wasm_emitter::lists::list_ops::NODE_ADD);
	}
	if ![left, right].iter().any(is_run_time_kind) || ![left, right].iter().all(numeric) {
		return None;
	}
	crate::wasm_emitter::lists::list_ops::NODE_ARITHMETIC
		.iter()
		.find(|(_, known, _)| known == op)
		.map(|(name, _, _)| *name)
}

/// Result kind of `left op right`: exact (Int, which includes ratios like `1/4`) unless an f64 is involved
pub fn arithmetic_kind(left: Kind, op: &Op, right: Kind) -> Kind {
	if node_arithmetic(left, op, right).is_some() {
		return Kind::Data; // Int or Float, decided at run time
	}
	match concatenation_kind(left, right) {
		Some(kind) if *op == Op::Add => return kind,
		_ => {}
	}
	if repeats_text(left, op, right) {
		Kind::Text // `"ab"*2`, see WasmGcEmitter::emit_text_repeat
	} else if [left, right]
		.iter()
		.any(|kind| matches!(kind, Kind::Text | Kind::Codepoint | Kind::List | Kind::Error | Kind::Function))
	{
		Kind::Error // no implicit conversion (DESIGN.md "Dangerous implicitness"); an error operand stays an error, a function is no number
	} else if left == Kind::Float || right == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// `a + b` joining lists or texts: lists (an empty one too) are a list; a text with anything but a function, a value held
/// as a Node too (a map value, a number of run-time kind), is a text
fn concatenation_kind(left: Kind, right: Kind) -> Option<Kind> {
	if [left, right].iter().all(|kind| matches!(kind, Kind::List | Kind::Empty)) && [left, right].contains(&Kind::List)
	{
		Some(Kind::List)
	} else {
		crate::wasm_emitter::text::text_builtins::concatenates(left, right).then_some(Kind::Text)
	}
}

/// text * int or int * text: the text repeated (Python), user decision 2026-10-03
pub fn repeats_text(left: Kind, op: &Op, right: Kind) -> bool {
	let is_text = |kind: Kind| matches!(kind, Kind::Text | Kind::Codepoint);
	*op == Op::Mul && ((is_text(left) && right == Kind::Int) || (left == Kind::Int && is_text(right)))
}

/// `base ^ 0.5`, `base ^ (1/3)`: an exact base with a non-integral constant exponent is no exact number, it is computed as
/// f64; `2^(n/12)`, an exponent that may be a ratio at run time, is decided then (node_pow): exact when it is whole
pub fn arithmetic_kind_of_operands(left: Kind, op: &Op, right: Kind, right_operand: &Node, scope: &Scope) -> Kind {
	let constant_exponent = if *op == Op::Pow { constant_value(right_operand) } else { None };
	let fractional_exponent = constant_exponent.is_some_and(|exponent| exponent.fract() != 0.0);
	let ratio_exponent = *op == Op::Pow && constant_exponent.is_none() && may_be_ratio(right_operand, scope);
	match arithmetic_kind(left, op, right) {
		Kind::Int if fractional_exponent => Kind::Float,
		Kind::Int if ratio_exponent => Kind::Data,
		kind => kind,
	}
}

/// An expression whose exact value may be a ratio: a division `/` in it, a fraction literal (`-0.45`), a variable given one
pub(crate) fn may_be_ratio(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::Key(left, op, right) => *op == Op::Div || may_be_ratio(left, scope) || may_be_ratio(right, scope),
		Node::List(items, Bracket::Round, _) => items.iter().any(|item| may_be_ratio(item, scope)),
		Node::Symbol(name) => scope.lookup(name).is_some_and(|local| local.may_be_ratio),
		number @ Node::Number(_) => {
			super::checks::literal_number_type_word(number) == Some(super::checks::RATIONAL_WORD)
		}
		_ => false,
	}
}

/// The value of an expression of number literals alone: `(1/3)`, `1.0 / 3`
fn constant_value(node: &Node) -> Option<f64> {
	match node.ungrouped() {
		Node::Number(number) => Some(f64::from(*number)),
		Node::Key(left, op, right) => {
			let (a, b) = (constant_value(left)?, constant_value(right)?);
			match op {
				Op::Add => Some(a + b),
				Op::Sub => Some(a - b),
				Op::Mul => Some(a * b),
				Op::Div => Some(a / b),
				_ => None,
			}
		}
		_ => None,
	}
}

/// Kind as written: a literal keeps its data kind (`3.14` is a float literal even though its value
/// computes exactly), anything else is inferred
pub fn written_kind(node: &Node, scope: &Scope) -> Kind {
	match node.drop_meta() {
		literal @ Node::Number(_) => literal.kind(),
		other => infer_type(other, scope),
	}
}

/// The number a text literal spells: `"4"` is 4
pub fn literal_number(value: &Node) -> Option<Node> {
	match value.drop_meta() {
		Node::Text(text) => crate::warp_parser::number_in_text(text).map(Node::Number),
		Node::Char(digit) => digit.to_digit(10).map(|digit| Node::Number(Number::Int(digit as i64))),
		_ => None,
	}
}

/// A math word's result: a Node when its argument is one, which may be a ± interval (crate::numbers::uncertain::INTERVAL_WORDS)
fn interval_or(kind: Kind, argument: &Node, scope: &Scope) -> Kind {
	if is_run_time_kind(&infer_type(argument, scope)) {
		Kind::Data
	} else {
		kind
	}
}

pub fn infer_type(node: &Node, scope: &Scope) -> Kind {
	let node = node.drop_meta();
	match node {
		Node::Number(number) => number_kind(number),
		Node::Text(_) => Kind::Text,
		Node::Char(_) => Kind::Codepoint,
		// ø, the empty value (card infer-type: it was the catch-all's Int)
		Node::Empty => Kind::Empty,
		Node::Symbol(name) => scope.binding(name).map_or(Kind::Symbol, |local| local.kind),
		// List handling: distinguish data lists from statement sequences and function calls
		Node::List(items, bracket, separator) if !items.is_empty() => {
			infer_list_type(node, items, bracket, separator, scope)
		}
		// a range as a value is the list of its numbers (wasm_emitter emit_range), as `x = 1..5` is
		Node::Key(_, Op::Range | Op::To, _) => Kind::List,
		// `5 ± 1` is uncertain, a kind the node arithmetic meets at run time (wasm_emitter/uncertain.rs)
		Node::Key(_, Op::PlusMinus, _) => Kind::Data,
		// Arithmetic: upgrade to Float if either operand is Float
		Node::Key(left, op, right) if op.is_arithmetic() => {
			arithmetic_kind_of_operands(infer_type(left, scope), op, infer_type(right, scope), right, scope)
		}
		// Assignment/definition: type comes from value, ø as the Node a variable holding it is (`if c { f = [] }`)
		Node::Key(_left, Op::Define | Op::Assign, right) => held_kind(right, || infer_type(right, scope)),
		Node::Key(left, op, right) if op.is_compound_assign() => compound_assign_kind(left, op, right, scope),
		// `x++` of a float steps the float
		Node::Key(target, Op::Inc | Op::Dec, _) if infer_type(target, scope) == Kind::Float => Kind::Float,
		// `global:value` is its value; a tag structure like `html:body` a Key
		Node::Key(left, Op::Colon, right) if left.is_symbol("global") => infer_type(right, scope),
		Node::Key(_, Op::Colon, _) => Kind::Key,
		Node::Key(value, Op::As, target) if let Some(kind) = converted_kind(node, value, target, scope) => kind,
		Node::Key(left, Op::And | Op::Or | Op::BitAnd | Op::BitOr, right) => logical_kind(left, right, scope),
		// Comparison operators return Int (boolean as 0/1)
		Node::Key(_, op, _) if op.is_comparison() => Kind::Int,
		// √x is irrational in general: an f64; of a value held as a Node maybe an interval, which it maps (a Node)
		Node::Key(left, Op::Sqrt | Op::Cbrt, right) if left.is_empty_node() => interval_or(Kind::Float, right, scope),
		// Prefix operators (neg, abs): inherit type from operand
		Node::Key(left, op, right) if op.is_prefix() && left.is_empty_node() => infer_type(right, scope),
		// Ternary operator: condition ? then : else, an error branch as in `if c {a} else {b}`
		Node::Key(_cond, Op::Question, then_else) => match then_else.drop_meta() {
			Node::Key(then_expr, Op::Colon, else_expr) => choice_kind(then_expr, else_expr, scope),
			_ => Kind::Int,
		},
		Node::Key(if_then, Op::Else, else_expr) if let Node::Key(_, Op::Then, then_expr) = if_then.drop_meta() => {
			choice_kind(then_expr, else_expr, scope)
		}
		Node::Key(if_condition, Op::Then, then_expr) if matches!(if_condition.drop_meta(), Node::Key(_, Op::If, _)) => {
			branches_kind(branch_kind(then_expr, scope), Kind::Int)
		}
		Node::Key(indexed, Op::Hash, index) if !indexed.is_empty_node() => subscript_kind(indexed, index, scope),
		Node::Key(condition, Op::Do, body) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)) => {
			loop_kind(body, scope)
		}
		// Default to Int for other cases
		_ => Kind::Int,
	}
}

fn number_kind(number: &Number) -> Kind {
	match number {
		// Decimal literals are exact numbers, see wasm_emitter/exact.rs
		Number::Float(f) if Number::is_exact_decimal(*f) => Kind::Int,
		Number::Float(_) | Number::Complex(_, _) | Number::Real(_) | Number::Nan | Number::Inf | Number::NegInf => {
			Kind::Float
		}
		// Integer and rational literals
		_ => Kind::Int,
	}
}

/// Compound assignment: upgrade if either side is Float
fn compound_assign_kind(left: &Node, op: &Op, right: &Node, scope: &Scope) -> Kind {
	let left_kind = infer_type(left, scope);
	let right_kind = infer_type(right, scope);
	// as `s + x` (arithmetic_kind); `xs += v` is what `xs.add(v)` lowers to
	let any_list = [left_kind, right_kind].contains(&Kind::List);
	match concatenation_kind(left_kind, right_kind).or(any_list.then_some(Kind::List)) {
		Some(kind) if op.base_op() == Op::Add => kind,
		// a variable held as a Node stays one: `total += event.value` of a global widened by it
		_ if left_kind == Kind::Empty => Kind::Empty,
		_ if left_kind == Kind::Float || right_kind == Kind::Float => Kind::Float,
		_ => Kind::Int,
	}
}

/// `value as target`, of the `node` it is; None for the conversions the other cases decide
fn converted_kind(node: &Node, value: &Node, target: &Node, scope: &Scope) -> Option<Kind> {
	if let Some(kind) = conversion_kind(&target.name().to_lowercase()) {
		return Some(kind);
	}
	// `"1/3" as number` is the number the text spells, of its kind (an exact ratio is an Int)
	if let Some(number) = spelled_number(node) {
		return Some(infer_type(&number, scope));
	}
	// `t as number` of a text known only at run time: an Int, a ratio or a Float, as the text says (card runtime-text-ratio);
	// of a value known only at run time (`number(t)` of a parameter, whatever kind a call guessed for it) likewise: a
	// number stays as it is (card number-text)
	if is_number_word(target) && !infer_type(value, scope).is_float() {
		return Some(Kind::Data);
	}
	// `v as float` is an f64; `as int`, `as exact` stay exact Ints
	builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()).then_some(Kind::Float)
}

/// `a or b`, `a and b` of a text or another Node give one of their operands (`"" or "d"` is "d"); of numbers an Int, a
/// float operand refused in that exact context
fn logical_kind(left: &Node, right: &Node, scope: &Scope) -> Kind {
	let (left, right) = (infer_type(left, scope), infer_type(right, scope));
	if [left, right].iter().any(|kind| kind.is_ref() || *kind == Kind::Codepoint) {
		branches_kind(left, right)
	} else {
		Kind::Int
	}
}

/// An element: `xs#i`, `xs[i]`, `text#i`
fn subscript_kind(indexed: &Node, index: &Node, scope: &Scope) -> Kind {
	let key = crate::warp_parser::subscript_key(index);
	// a field read by name with a declared kind: `v.x` of `type V {x: float}`; a field of a value known only at run time
	// (`request.body.priority` of an any) holds what it holds, whatever a class's field of that name declares
	if let Some(Node::Text(name)) = key.map(Node::drop_meta) {
		if let Some(kind) = scope.function_kind(&field_kind_key(name)).filter(|_| !declared_any(indexed, scope)) {
			return kind;
		}
		// `p.xy` of a list: the list of its components (decision S1)
		let is_swizzle = crate::words::swizzle_components(name).is_some_and(|components| components.len() > 1);
		if is_swizzle && infer_type(indexed, scope) == Kind::List {
			return Kind::List;
		}
	}
	// a value looked up by a name in a map of unknown values (`graph[node]` of a parameter) is held as a Node; so is a
	// member a text cannot have (`line["length"]`, the run-time branch of `line.length` beside a map with that key)
	let by_name =
		key.is_some_and(|key| key.is_text_literal() || matches!(infer_type(key, scope), Kind::Text | Kind::Codepoint));
	if by_name && infer_type(indexed, scope) == Kind::Text {
		return Kind::Empty;
	}
	element_kind(indexed, scope).unwrap_or(if by_name { Kind::Empty } else { Kind::Int })
}

/// `x as char`, `x as list`, `x as text`
fn conversion_kind(target: &str) -> Option<Kind> {
	match crate::type_tests::canonical_spec_word(target) {
		"codepoint" => Some(Kind::Codepoint),
		"list" => Some(Kind::List),
		"text" | crate::wasm_emitter::numbers::exact::DECIMAL_WORD => Some(Kind::Text),
		radix if crate::wasm_emitter::numbers::radix::named_radix(radix).is_some() => Some(Kind::Int),
		_ => None,
	}
}

/// `as number`, `as Number`
fn is_number_word(target: &Node) -> bool {
	crate::type_tests::canonical_spec_word(&target.name().to_lowercase()) == "number"
}

/// A loop is its last body value (P55): a text, character or list one held as such (card loop-value-kind), ø after a
/// print (P213); a number or ø otherwise, as a loop may never run (card loop-empty), so a Node decided at run time
fn loop_kind(body: &Node, scope: &Scope) -> Kind {
	let (statements, _) = crate::wasm_emitter::split_step(body);
	let prints = match statements.drop_meta() {
		Node::List(items, _, _) => items.last().is_some_and(is_output_call),
		other => is_output_call(other),
	};
	let kind = infer_type(&statements, scope);
	match kind {
		_ if prints => Kind::Empty,
		Kind::Text | Kind::Codepoint | Kind::List => kind,
		_ => Kind::Data,
	}
}

/// `print x`, `puts x`: an output word applied to one value
pub(crate) fn is_output_call(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, _]
		if word.symbol_name().is_some_and(|name| crate::wasm_emitter::OUTPUT_CALLS.contains(&name))))
}

/// The type word of a cast `int 4`, `int("5")`, `double 2`: a type word applied to one value. A list `[int, 4]`, a comma
/// group `(int, 4)` or a variable named like a type word (`int = 3; int 4`) casts nothing (card variable-named)
pub fn type_cast_of<'a>(
	items: &'a [Node],
	bracket: &Bracket,
	separator: &Separator,
	is_variable: &dyn Fn(&str) -> bool,
) -> Option<&'a str> {
	let (name, _) = applied_word(items, bracket, separator)?;
	(!is_variable(name) && type_word_kind(&name.to_lowercase()).is_some()).then_some(name)
}

/// A built-in type word that converts the value it is applied to: `int 3.7`, `text 5`, `hex 255`, `byte 65`
pub fn is_type_call_word(name: &str) -> bool {
	type_word_kind(&name.to_lowercase()).is_some() || crate::wasm_emitter::is_called_conversion(name)
}

/// A word applied to one value: `f(x)` and `f x` are the same call, braced or not. A list `[f, x]`, a comma group
/// `(f, x)`, the statements `f; x` or a typed declaration `byte b = 10` apply nothing
pub fn applied_word<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator) -> Option<(&'a str, &'a Node)> {
	let [word, value] = items else { return None };
	let Node::Symbol(name) = word.drop_meta() else {
		return None;
	};
	let listed = *bracket == Bracket::Square || *separator == Separator::Comma || separator.separates_statements();
	let declared = matches!(value.drop_meta(), Node::Key(_, Op::Assign | Op::Define, _));
	(!listed && !declared).then_some((name.as_str(), value))
}

/// The kind of a non-empty list: a call's result, a statement sequence's last value, or a data list
pub(super) fn infer_list_type(
	node: &Node,
	items: &[Node],
	bracket: &Bracket,
	separator: &Separator,
	scope: &Scope,
) -> Kind {
	if crate::host::fetch_call(node).is_some() {
		return Kind::Text; // or an Error value, see check_unchecked_use
	}
	let head = items.first().and_then(Node::symbol_name);
	if let Some(name) = head {
		let named = special_form_kind(name, items, bracket, scope)
			.or_else(|| closure_call_kind(name, items, scope))
			// a parameter takes a function's name, any other variable in a square list (wasm_emitter takes_list_head)
			.or_else(|| {
				let is_variable =
					scope.is_parameter(name) || (*bracket == Bracket::Square && scope.lookup(name).is_some());
				scope.function_kind(name).filter(|_| !is_variable)
			})
			.or_else(|| builtin_word_kind(name, items, bracket, scope));
		if let Some(kind) = named {
			return kind;
		}
	}
	// Type constructor: int("5"), float("1.5"), str(3), double 2
	if let Some(kind) =
		type_cast_of(items, bracket, separator, &|name| scope.lookup(name).is_some()).and_then(type_word_kind)
	{
		return kind;
	}
	// Function call with parentheses: a library word has its own result, any other call is assumed Int. A comma list
	// `(y, 4)` is a tuple, never the call y(4) (`f(a, b)` parses as `(f a b)`)
	if *bracket == Bracket::Round && items.len() >= 2 && *separator != Separator::Comma {
		if let Some(name) = head {
			return library_call_kind(name, items, scope);
		}
	}
	// Zero-arg function call: (funcname) with no args
	if *bracket == Bracket::Round && items.len() == 1 {
		return match head {
			Some(name) if crate::ffi::is_ffi_function(name) => ffi_call_kind(name),
			// `abs(c)`: a variable in parentheses is the variable
			Some(name) => scope
				.binding(name)
				.map(|local| local.kind)
				.or_else(|| crate::library_words::result_kind(name))
				.unwrap_or(Kind::Int), // a user function without arguments: Int
			// Grouping: (x) has the type of x
			None => infer_type(&items[0], scope),
		};
	}
	// Data list: all items are pure data, or `[…]` computing its elements → Kind::List
	let computed_elements = (*bracket == Bracket::Square && !items.iter().any(|item| is_statement(item, bracket)))
		|| (*bracket == Bracket::Round && *separator == Separator::Comma);
	if computed_elements || (!is_unbracketed_block(items, bracket, separator) && items.iter().all(is_data_node)) {
		return Kind::List;
	}
	// Statement sequence: return type of last item
	items.last().map_or(Kind::Empty, |last| infer_type(last, scope))
}

/// The kind of a lowered form headed by `name` that is no function call: an insertion, a time, `return x`, `data …`
fn special_form_kind(name: &str, items: &[Node], bracket: &Bracket, scope: &Scope) -> Option<Kind> {
	if [ZERO_FILL_CALL, INSERT_EITHER_CALL].contains(&name) {
		return Some(Kind::List);
	}
	// `now`, `2024-02-29`: a time, a kind the node operations meet at run time (wasm_emitter/times.rs)
	if crate::time::TIME_CALLS.contains(&name) {
		return Some(Kind::Data);
	}
	if name == crate::library_words::LIST_SUM && items.len() == 3 {
		return Some(infer_type(&items[2], scope)); // the loop it dispatches around
	}
	// `data a and b` never runs: it is the literal written, here a Key
	if name == crate::blocks::DATA_WORD && items.len() == 2 && *bracket == Bracket::None {
		return Some(match items[1].kind() {
			Kind::Block => Kind::List,
			kind => kind,
		});
	}
	// code is the node quoted (decision body-code)
	if name == crate::words::AS_CODE && items.len() == 2 {
		return Some(infer_type(&items[1], scope));
	}
	if RETURNING_KEYWORDS.contains(&name) && items.len() == 2 {
		return Some(infer_type(&items[1], scope)); // `return x` is worth x
	}
	None
}

/// A closure call is worth what all the closures its variable may hold give
fn closure_call_kind(name: &str, items: &[Node], scope: &Scope) -> Option<Kind> {
	crate::closures::closure_call_arity(name)?;
	let targets = items
		.get(1)
		.and_then(Node::symbol_name)
		.and_then(|variable| scope.closure_targets_of(variable))
		.filter(|targets| !targets.is_empty())?;
	let mut kinds = targets.iter().map(|target| scope.function_kind(target).unwrap_or(Kind::Data));
	let first = kinds.next().unwrap_or(Kind::Data);
	Some(if kinds.all(|kind| kind == first) { first } else { Kind::Data })
}

/// The kind of a call of a built-in word, the program's own function of that name already answered: closures, cells,
/// texts, output, counting, imported and FFI functions
fn builtin_word_kind(name: &str, items: &[Node], bracket: &Bracket, scope: &Scope) -> Option<Kind> {
	use crate::wasm_emitter::functions::cells::{CELL_WORDS, MAKING_WORDS};
	if name == crate::closures::CLOSURE_NEW {
		return Some(Kind::Function);
	}
	// another runtime's value or a std adapter's: any Node, held like a map value (its kind decided at run time)
	if crate::host::ANY_VALUE_WORDS.contains(&name) {
		return Some(Kind::Empty);
	}
	// a cell's value is held as a Node, like a map value (Empty), and joins a text or adds at run time
	if CELL_WORDS.contains(&name) {
		return Some(if MAKING_WORDS.contains(&name) { Kind::Data } else { Kind::Empty });
	}
	if let Some(kind) = crate::wasm_emitter::text::text_builtins::text_builtin_kind(name, items.len() - 1) {
		return Some(kind);
	}
	if name == crate::warp_parser::TEXT_TIMES {
		// `it times it` of numbers multiplies (list_emitter.rs emit_text_times)
		return Some(match items.get(2).map(|repeated| infer_type(repeated, scope)) {
			Some(kind) if kind.is_int() || kind.is_float() => {
				arithmetic_kind(infer_type(&items[1], scope), &Op::Mul, kind)
			}
			_ => Kind::Text,
		});
	}
	// `count ys`, `print x`, `type(x)`, `puti n`: a builtin the emitter expands gives what its prelude line declares,
	// called with arguments or parentheses
	if items.len() >= 2 || *bracket == Bracket::Round {
		if let Some(kind) = crate::library_words::emitted_call_kind(name, items.len() - 1) {
			return Some(kind);
		}
	}
	// a call of a function the component being compiled imports (`host.time()`)
	if let Some(kind) = crate::wasm_emitter::host::component_adapters::imported_kind(name) {
		return Some(kind);
	}
	if items.len() < 2 {
		return None;
	}
	// FFI/builtin function calls (strcmp, strlen, abs, …) return Int by default
	crate::ffi::is_ffi_function(name).then(|| match items {
		[_, argument] if crate::numbers::uncertain::maps_intervals(name) => {
			interval_or(ffi_call_kind(name), argument, scope)
		}
		_ => ffi_call_kind(name),
	})
}

/// The kind of `name(arguments…)`: a library word's own result, Int for any other call
fn library_call_kind(name: &str, items: &[Node], scope: &Scope) -> Kind {
	use crate::library_words::ArgumentResult;
	// `field_with(object, …): same as object`, `map_get_or(m, …): value of m`: read off that argument
	if let Some((how, index)) = crate::library_words::argument_result(name) {
		let Some(argument) = items.get(index + 1) else { return Kind::Empty };
		return match how {
			// a value kept as a Node: an object's or a list's kind, a number or character inside it a Node
			ArgumentResult::Same => Some(infer_type(argument, scope)).filter(Kind::is_ref).unwrap_or(Kind::Empty),
			ArgumentResult::Element => element_kind(argument, scope).unwrap_or(Kind::Empty),
		};
	}
	// an overloaded word's by its first argument: a reversal of a text is a text, of a list a list
	crate::library_words::call_result_kind(name, || items.get(1).map(|first| infer_type(first, scope)))
		.unwrap_or(Kind::Int)
}

/// Kind of an element of `indexed`: a list's common element type (`list of text` → Text), the character of a text;
/// `None` when unknown (a mixed list, a map)
pub(super) fn element_kind(indexed: &Node, scope: &Scope) -> Option<Kind> {
	if matches!(infer_type(indexed, scope), Kind::Text | Kind::Symbol) {
		return Some(Kind::Text); // a character, held as a node like any one-character text (see binding_kind)
	}
	let list_type = list_type_name(indexed, scope);
	// a part of code (`parse("a; b")#2`) is any node
	if list_type == MAP_WORD
		|| super::is_code(indexed, scope)
		|| list_type == NODE_LIST_TYPE
		|| matches!(infer_type(indexed, scope), Kind::Empty | Kind::Data)
	{
		return Some(Kind::Empty); // values of different types, or of a value held as a Node: held as a Node, like an optional
	}
	match list_type.strip_prefix("list of ").or_else(|| list_type.strip_prefix(MAP_TYPE_PREFIX))? {
		// `m[k]` of a map is a value
		word if word.starts_with("list") => Some(Kind::List),
		RATIONAL_WORD => Some(Kind::Int),
		REAL_WORD => Some(Kind::Float),
		// a mix of floats and exact numbers (`[sqrt(2.0), 0.2]`, `[π 1]`): each element held as its Node, of its own kind
		NUMBER_WORD => Some(Kind::Empty),
		word => match type_word_kind(word) {
			Some(Kind::Codepoint) => Some(Kind::Text),
			Some(kind) => Some(kind),
			None => Some(Kind::Empty), // instances, keys, symbols: held as Nodes
		},
	}
}

/// A branch that is `error …` (or a block ending in it): the bottom kind, it never gives a value to its if
pub(crate) fn raises_error(branch: &Node) -> bool {
	match branch.drop_meta() {
		Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => {
			raises_error(&statements[statements.len() - 1])
		}
		other => crate::errors::run_time::returned_error_message(other).is_some(),
	}
}

/// The expression a block `{a; b}` yields: its last statement; `{print b}` is one expression, the call print(b), not
/// the statements `print` and `b`
pub(crate) fn block_result(block: &Node) -> Option<Node> {
	match block.drop_meta() {
		Node::List(words, Bracket::Curly, Separator::Space) if words.len() > 1 => {
			Some(Node::List(words.clone(), Bracket::None, Separator::Space))
		}
		Node::List(statements, Bracket::Curly, _) => statements.last().cloned(),
		_ => None,
	}
}

/// `if c then error "…" else x`: τ or error (Decided #1, errors are values), a Node keeping the error until a use
/// wants a number; a number is held as a Node of run-time kind
fn error_or(kind: Kind) -> Kind {
	if matches!(kind, Kind::Int | Kind::Float) {
		Kind::Data
	} else {
		kind
	}
}

/// The kind of a choice between two branches; an `error …` branch raises its error, so the other branch decides the
/// kind (bottom kind)
fn choice_kind(then_expr: &Node, else_expr: &Node, scope: &Scope) -> Kind {
	match (raises_error(then_expr), raises_error(else_expr)) {
		(true, false) => error_or(branch_kind(else_expr, scope)),
		(false, true) => error_or(branch_kind(then_expr, scope)),
		// `if c then 2 else true`: a bool beside a number is a Node of run-time kind, keeping its mark (card rust-lane-if)
		_ if is_boolean(then_expr, scope) != is_boolean(else_expr, scope) => {
			error_or(branches_kind(branch_kind(then_expr, scope), branch_kind(else_expr, scope)))
		}
		_ => branches_kind(branch_kind(then_expr, scope), branch_kind(else_expr, scope)),
	}
}

/// A branch `{a; b}` is worth its last statement
pub(crate) fn branch_kind(branch: &Node, scope: &Scope) -> Kind {
	if let Some(result) = block_result(branch) {
		return branch_kind(&result, scope);
	}
	// a branch yielding ø (`if c then 3 else ø`) is a Node, as a variable holding ø is
	let branch = branch.drop_meta();
	held_kind(branch, || infer_type(branch, scope))
}

/// Either branch a reference type (Text, Symbol, List…) or a character: the value is a Node, else a number
pub(super) fn branches_kind(then_kind: Kind, else_kind: Kind) -> Kind {
	let kinds = [then_kind, else_kind];
	if then_kind == else_kind && (then_kind.is_ref() || then_kind == Kind::Codepoint) {
		// two instances or two maps (`if c then T{…} else T{…}`) stay what they are
		then_kind
	} else if kinds.contains(&Kind::List) {
		// a list and a list (or ø, the empty list) is a list; a list and anything else a Node of its run-time kind
		if kinds.iter().all(|kind| matches!(kind, Kind::List | Kind::Empty)) {
			Kind::List
		} else {
			Kind::Data
		}
	} else if kinds.contains(&Kind::Empty) {
		// a Node whose kind is known only at run time (an awaited job's result, ø): the value keeps that kind, so
		// `(if c then 0 else job_result) + 1` adds at run time instead of failing as text
		Kind::Empty
	} else if kinds.contains(&Kind::Data)
		&& kinds.iter().all(|kind| matches!(kind, Kind::Int | Kind::Float | Kind::Data))
	{
		// a number and a number of run-time kind (`xs#1 + rest`): a number decided at run time, not a text
		Kind::Data
	} else if kinds.iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint)) {
		Kind::Text
	} else if kinds.iter().any(Kind::is_ref) {
		// an instance or a map and a number (`if a is V then a.plus(b) else a + b`): a Node of its run-time kind
		Kind::Data
	} else if then_kind == Kind::Float || else_kind == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// `"1/3" as number`: the number a constant text spells, which the cast is (card fraction-number)
pub fn spelled_number(node: &Node) -> Option<Node> {
	let Node::Key(value, Op::As, target) = node.drop_meta() else {
		return None;
	};
	is_number_word(target).then(|| literal_number(value)).flatten()
}

/// A value declared `any` (`r: any`, a route's `request`) or a field of one (`request.body`): known only at run time
fn declared_any(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => scope
			.lookup(name)
			.and_then(|local| local.type_node.as_deref())
			.is_some_and(|type_node| type_node.is_symbol(ANY_TYPE)),
		Node::Key(indexed, Op::Hash, _) => declared_any(indexed, scope),
		_ => false,
	}
}

/// Kind of a value known before running the program
pub(crate) fn literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Int(_) | Number::BigInt(_)) | Node::True | Node::False => Some(Kind::Int),
		Node::Number(_) => Some(Kind::Float),
		Node::Text(_) => Some(Kind::Text),
		Node::Char(_) => Some(Kind::Codepoint),
		Node::Key(nothing, Op::Neg, operand) if matches!(nothing.drop_meta(), Node::Empty) => literal_kind(operand),
		_ => None,
	}
}

/// The kind of a literal or of arithmetic on literals: `π*1000000` is a float, `2*3` and `2.0*3` (exact decimals) Ints
pub(crate) fn computed_literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() && !matches!(left.drop_meta(), Node::Empty) => {
			// exact: whole or not by its value, `7/2*2` is 7, `33/10*2` is 33/5
			if let Some(exact) = exact_literal_value(value) {
				return Some(if exact.is_integer() { Kind::Int } else { Kind::Float });
			}
			match (operand_kind(left)?, operand_kind(right)?) {
				(Kind::Int, Kind::Int) => Some(Kind::Int),
				(Kind::Float | Kind::Int, Kind::Float | Kind::Int) => Some(Kind::Float),
				// `"5"*3` repeats the text (card repeat-int)
				(left_kind, right_kind) if super::inference::repeats_text(left_kind, op, right_kind) => {
					Some(Kind::Text)
				}
				_ => None,
			}
		}
		_ => literal_kind(value),
	}
}

/// The exact value of arithmetic on exact literals (`3.3*2` is 33/5), None for anything else or a division by zero
pub(super) fn exact_literal_value(node: &Node) -> Option<crate::extensions::reals::Rational> {
	match node.drop_meta() {
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => {
			let (numerator, denominator) = crate::wasm_emitter::numbers::exact::decimal_fraction(*f);
			Some(crate::extensions::reals::Rational::new(numerator, denominator))
		}
		Node::Number(number @ (Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_))) => {
			Some(number.to_rational())
		}
		Node::Key(nothing, Op::Neg | Op::Sub, operand) if matches!(nothing.drop_meta(), Node::Empty) => {
			Some(exact_literal_value(operand)?.neg())
		}
		Node::Key(left, op, right) => {
			let (left, right) = (exact_literal_value(left)?, exact_literal_value(right)?);
			match op {
				Op::Add => Some(left.add(&right)),
				Op::Sub => Some(left.add(&right.neg())),
				Op::Mul => Some(left.mul(&right)),
				Op::Div => Some(left.mul(&right.inverse()?)),
				_ => None,
			}
		}
		_ => None,
	}
}

/// An operand of arithmetic on literals: an exact decimal stays exact (`2.0*3` is 6), a real constant is a float
pub(super) fn operand_kind(operand: &Node) -> Option<Kind> {
	match operand.drop_meta() {
		Node::Symbol(name) if REAL_CONSTANTS.contains(&name.as_str()) => Some(Kind::Float),
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => Some(Kind::Int),
		Node::Key(..) => computed_literal_kind(operand),
		_ => literal_kind(operand),
	}
}

/// The kind a call argument certainly has, judged from the literal alone
pub fn argument_literal_kind(argument: &Node) -> Option<Kind> {
	match argument.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _) => {
			Some(infer_type(argument, &Scope::new()))
		}
		Node::Empty => Some(Kind::List),
		// `"" + n`, `"#" + x + y`: a concatenation with a text is a text
		Node::Key(_, Op::Add, _) if crate::lowering::declaration_forms::is_text(argument) => Some(Kind::Text),
		// `1.5 as float`, `x as float`: the kind the conversion names
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => {
			Some(Kind::Float)
		}
		_ if crate::closures::as_closure_new(argument).is_some() => Some(Kind::Function),
		Node::List(entries, Bracket::Curly, _)
			if entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			Some(Kind::List)
		} // a map
		_ => None,
	}
}

/// A decimal literal, `2.5` or `2.0` (an exact decimal, so its kind may be Int): a float an int parameter refuses (P49, P49b)
pub(super) fn is_decimal_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(crate::extensions::numbers::Number::Float(_)))
}
