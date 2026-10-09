//! Kind inference: the kind of a node, of arithmetic, of list elements and branches

use super::*;

/// A statement changing a variable it reads: `x=x+1`, `x+=1`, `x++` (data like `{x=1}` reads nothing)
pub(super) fn is_update(node: &Node) -> bool {
	match node {
		Node::Key(target, Op::Assign, value) => matches!(target.drop_meta(), Node::Symbol(name) if crate::warp_parser::mentions(value, name)),
		Node::Key(target, op, _) if matches!(op, Op::Inc | Op::Dec) || op.is_compound_assign() => matches!(target.drop_meta(), Node::Symbol(_)),
		_ => false,
	}
}

/// Infer the Kind for an expression
/// Returns Int, Float, Text, etc. based on the expression's result type
/// Result kind of `left op right`: exact (Int, which includes ratios like `1/4`) unless an f64 is involved
/// The runtime function of `a op b` when an operand's kind is known only at run time (a field of a map parameter, an
/// element of a parsed JSON value): Int or Float is decided by the values (wasm_emitter list_ops NODE_ARITHMETIC)
/// A value whose kind is known only at run time, held as a Node: maybe a number, maybe a ± interval
pub fn is_run_time_kind(kind: &Kind) -> bool {
	matches!(kind, Kind::Data | Kind::Empty)
}

pub fn node_arithmetic(left: Kind, op: &Op, right: Kind) -> Option<&'static str> {
	let numeric = |kind: &Kind| matches!(kind, Kind::Int | Kind::Float) || is_run_time_kind(kind);
	// a value of run-time kind added to a list: node_add concatenates when it is a list too
	if *op == Op::Add && [left, right].contains(&Kind::Data) && [left, right].contains(&Kind::List) {
		return Some(crate::wasm_emitter::list_ops::NODE_ADD);
	}
	if ![left, right].iter().any(is_run_time_kind) || ![left, right].iter().all(numeric) {
		return None;
	}
	let index = [Op::Add, Op::Sub, Op::Mul, Op::Div].iter().position(|candidate| candidate == op)?;
	Some(crate::wasm_emitter::list_ops::NODE_ARITHMETIC[index].0)
}

pub fn arithmetic_kind(left: Kind, op: &Op, right: Kind) -> Kind {
	if node_arithmetic(left, op, right).is_some() {
		return Kind::Data; // Int or Float, decided at run time
	}
	if *op == Op::Add && [left, right].iter().all(|kind| matches!(kind, Kind::List | Kind::Empty)) && [left, right].contains(&Kind::List) {
		Kind::List // concatenation
	} else if *op == Op::Add && (crate::wasm_emitter::text_builtins::concatenates(left, right)
		|| [left, right].iter().any(|kind| matches!(kind, Kind::Empty | Kind::Data)) && [left, right].iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint)))
		|| repeats_text(left, op, right)
	{
		// concatenation; a value held as a Node (a map value, an element of one, a number of run-time kind) joining a text; `"ab"*2` repeats, see WasmGcEmitter::emit_text_repeat
		Kind::Text
	} else if [left, right].iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint | Kind::List | Kind::Error | Kind::Function)) {
		Kind::Error // no implicit conversion (DESIGN.md "Dangerous implicitness"); an error operand stays an error, a function is no number
	} else if left == Kind::Float || right == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// text * int or int * text: the text repeated (Python), user decision 2026-10-03
pub fn repeats_text(left: Kind, op: &Op, right: Kind) -> bool {
	let is_text = |kind: Kind| matches!(kind, Kind::Text | Kind::Codepoint);
	*op == Op::Mul && ((is_text(left) && right == Kind::Int) || (left == Kind::Int && is_text(right)))
}

/// `base ^ 0.5`, `base ^ (1/3)`: an exact base with a non-integral constant exponent is no exact number, it is computed as f64
pub fn arithmetic_kind_of_operands(left: Kind, op: &Op, right: Kind, right_operand: &Node) -> Kind {
	let fractional_exponent = *op == Op::Pow && constant_value(right_operand).is_some_and(|exponent| exponent.fract() != 0.0);
	match arithmetic_kind(left, op, right) {
		Kind::Int if fractional_exponent => Kind::Float,
		kind => kind,
	}
}

/// The value of an expression of number literals alone: `(1/3)`, `1.0 / 3`
fn constant_value(node: &Node) -> Option<f64> {
	match node.drop_meta() {
		Node::Number(number) => Some(f64::from(*number)),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => constant_value(&items[0]),
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

/// A math word's result: a Node when its argument is one, which may be a ± interval (crate::uncertain::INTERVAL_WORDS)
fn interval_or(kind: Kind, argument: &Node, scope: &Scope) -> Kind {
	if is_run_time_kind(&infer_type(argument, scope)) { Kind::Data } else { kind }
}

pub fn infer_type(node: &Node, scope: &Scope) -> Kind {
	let node = node.drop_meta();
	match node {
		// Decimal literals are exact numbers, see wasm_emitter/exact.rs
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => Kind::Int,
		Node::Number(Number::Float(_)) => Kind::Float,
		Node::Number(Number::Complex(_, _) | Number::Real(_) | Number::Nan | Number::Inf | Number::NegInf) => Kind::Float,
		// Integer and rational literals
		Node::Number(_) => Kind::Int,
		// Text and char
		Node::Text(_) => Kind::Text,
		Node::Char(_) => Kind::Codepoint,
		// Symbol (identifier)
		Node::Symbol(name) => {
			if let Some(local) = scope.binding(name) {
				local.kind
			} else {
				Kind::Symbol  // Unknown symbol defaults to Symbol
			}
		}
		// List handling: distinguish data lists from statement sequences and function calls
		Node::List(items, bracket, separator) if !items.is_empty() => infer_list_type(node, items, bracket, separator, scope),
		// a range as a value is the list of its numbers (wasm_emitter emit_range), as `x = 1..5` is
		Node::Key(_, Op::Range | Op::To, _) => Kind::List,
		// `5 ± 1` is uncertain, a kind the node arithmetic meets at run time (wasm_emitter/uncertain.rs)
		Node::Key(_, Op::PlusMinus, _) => Kind::Data,
		// Arithmetic: upgrade to Float if either operand is Float
		Node::Key(left, op, right) if op.is_arithmetic() => {
			arithmetic_kind_of_operands(infer_type(left, scope), op, infer_type(right, scope), right)
		}
		// Assignment/definition: type comes from value, ø as the Node a variable holding it is (`if c { f = [] }`)
		Node::Key(_left, Op::Define | Op::Assign, right) => held_kind(right, || infer_type(right, scope)),
		// Compound assignment: upgrade if either side is Float
		Node::Key(left, op, right) if op.is_compound_assign() => {
			let left_kind = infer_type(left, scope);
			let right_kind = infer_type(right, scope);
			// a text joined with a text, a character or a value held as a Node: as `s + x` (arithmetic_kind)
			let joins_text = matches!(left_kind, Kind::Text | Kind::Codepoint) && arithmetic_kind(left_kind, &Op::Add, right_kind) == Kind::Text;
			if op.base_op() == Op::Add && (joins_text || crate::wasm_emitter::text_builtins::concatenates(left_kind, right_kind)) {
				Kind::Text
			} else if op.base_op() == Op::Add && (left_kind == Kind::List || right_kind == Kind::List) {
				Kind::List // `xs += [v]`, what `xs.add(v)` lowers to
			} else if left_kind == Kind::Float || right_kind == Kind::Float {
				Kind::Float
			} else {
				Kind::Int
			}
		}
		// global:value -> type comes from value
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					return infer_type(right, scope);
				}
			}
			// Tag structures like html:body are Key
			Kind::Key
		}
		Node::Key(_, Op::As, target) if matches!(target.name().to_lowercase().as_str(), "char" | "character") => Kind::Codepoint,
		Node::Key(_, Op::As, target) if target.name().to_lowercase() == "list" => Kind::List,
		Node::Key(_, Op::As, target) if matches!(target.name().to_lowercase().as_str(), "string" | "str" | "text") => Kind::Text,
		// `"1/3" as number` is the number the text spells, of its kind (an exact ratio is an Int)
		Node::Key(..) if spelled_number(node).is_some() => infer_type(&spelled_number(node).expect("spelled"), scope),
		// `v as float` is an f64; `as int`, `as exact` stay exact Ints
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => Kind::Float,
		// `a or b`, `a and b` of a text or another Node give one of their operands (`"" or "d"` is "d"); of numbers an
		// Int, a float operand refused in that exact context
		Node::Key(left, Op::And | Op::Or, right) => match (infer_type(left, scope), infer_type(right, scope)) {
			(left, right) if [left, right].iter().any(|kind| kind.is_ref() || *kind == Kind::Codepoint) => branches_kind(left, right),
			_ => Kind::Int,
		},
		// Comparison operators return Int (boolean as 0/1)
		Node::Key(_, op, _) if op.is_comparison() => Kind::Int,
		// √x is irrational in general: an f64; of a value held as a Node maybe an interval, which it maps (a Node)
		Node::Key(left, Op::Sqrt | Op::Cbrt, right) if matches!(left.drop_meta(), Node::Empty) => interval_or(Kind::Float, right, scope),
		// Prefix operators (neg, abs): inherit type from operand
		Node::Key(left, op, right) if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) => {
			infer_type(right, scope)
		}
		// Ternary operator: condition ? then : else
		Node::Key(_cond, Op::Question, then_else) => match then_else.drop_meta() {
			Node::Key(then_expr, Op::Colon, else_expr) => branches_kind(infer_type(then_expr, scope), infer_type(else_expr, scope)),
			_ => Kind::Int,
		},
		// if c {a} else {b}; an `error(…)` branch raises its error, so the other branch decides the kind (bottom kind)
		Node::Key(if_then, Op::Else, else_expr) if matches!(if_then.drop_meta(), Node::Key(_, Op::Then, _)) => {
			let Node::Key(_, _, then_expr) = if_then.drop_meta() else { unreachable!("guarded") };
			match (raises_error(then_expr), raises_error(else_expr)) {
				(true, false) => branch_kind(else_expr, scope),
				(false, true) => branch_kind(then_expr, scope),
				_ => branches_kind(infer_type(if_then, scope), branch_kind(else_expr, scope)),
			}
		}
		Node::Key(if_condition, Op::Then, then_expr) if matches!(if_condition.drop_meta(), Node::Key(_, Op::If, _)) => {
			branches_kind(branch_kind(then_expr, scope), Kind::Int)
		}
		// An element: `xs#i`, `xs[i]`, `text#i`
		// a value looked up by a name in a map of unknown values (`graph[node]` of a parameter) is held as a Node
		// a field read by name with a declared kind: `v.x` of `type V {x: float}`
		Node::Key(indexed, Op::Hash, index) if crate::warp_parser::subscript_key(index)
			.and_then(|key| match key.drop_meta() { Node::Text(name) => scope.function_kind(&field_kind_key(name)), _ => None })
			.is_some() && !matches!(indexed.drop_meta(), Node::Empty) => {
			let Some(Node::Text(name)) = crate::warp_parser::subscript_key(index).map(Node::drop_meta) else { unreachable!("guarded") };
			scope.function_kind(&field_kind_key(name)).expect("guarded")
		}
		Node::Key(indexed, Op::Hash, index) if !matches!(indexed.drop_meta(), Node::Empty) => element_kind(indexed, scope).unwrap_or_else(|| {
			let by_name = crate::warp_parser::subscript_key(index).is_some_and(|key| matches!(key.drop_meta(), Node::Text(_) | Node::Char(_))
				|| matches!(infer_type(key, scope), Kind::Text | Kind::Codepoint));
			if by_name { Kind::Empty } else { Kind::Int }
		}),
		Node::Key(condition, Op::Do, body) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)) => loop_kind(body, scope),
		// Default to Int for other cases
		_ => Kind::Int,
	}
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
		if matches!(word.drop_meta(), Node::Symbol(name) if crate::wasm_emitter::OUTPUT_CALLS.contains(&name.as_str()))))
}

/// The kind of a non-empty list: a call's result, a statement sequence's last value, or a data list
/// The type word of a cast `int 4`, `int("5")`, `double 2`: a type word applied to one value. A list `[int, 4]`, a comma
/// group `(int, 4)` or a variable named like a type word (`int = 3; int 4`) casts nothing (card variable-named)
pub fn type_cast_of<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator, is_variable: &dyn Fn(&str) -> bool) -> Option<&'a str> {
	let [word, _] = items else { return None };
	let Node::Symbol(name) = word.drop_meta() else { return None };
	let listed = *bracket == Bracket::Square || *separator == Separator::Colon; // Colon is the comma
	(!listed && !is_variable(name) && type_word_kind(&name.to_lowercase()).is_some()).then_some(name.as_str())
}

pub(super) fn infer_list_type(node: &Node, items: &[Node], bracket: &Bracket, separator: &Separator, scope: &Scope) -> Kind {
	if crate::host::fetch_call(node).is_some() {
		return Kind::Text; // or an Error value, see check_unchecked_use
	}
	if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if [ZERO_FILL_CALL, INSERT_AT_CALL, INSERT_EITHER_CALL].contains(&name.as_str())) {
		return Kind::List;
	}
	if let Node::Symbol(name) = items[0].drop_meta() {
		if name == crate::library_words::LIST_SUM && items.len() == 3 {
			return infer_type(&items[2], scope); // the loop it dispatches around
		}
		// `data a and b` never runs: it is the literal written, here a Key
		if name == crate::blocks::DATA_WORD && items.len() == 2 && *bracket == Bracket::None {
			return match items[1].kind() { Kind::Block => Kind::List, kind => kind };
		}
		if RETURNING_KEYWORDS.contains(&name.as_str()) && items.len() == 2 {
			return infer_type(&items[1], scope); // `return x` is worth x
		}
		if let Some(arity) = crate::closures::closure_call_arity(name) {
			let callee = items.get(1).and_then(|argument| match argument.drop_meta() {
				Node::Symbol(variable) => Some(variable.as_str()),
				_ => None,
			});
			if let Some(variable) = callee {
				if let Some(targets) = scope.closure_targets_of(variable) {
					if !targets.is_empty() {
						let mut kinds = targets.iter().map(|target| scope.function_kind(target).unwrap_or(Kind::Data));
						let first = kinds.next().unwrap_or(Kind::Data);
						return if kinds.all(|kind| kind == first) { first } else { Kind::Data };
					}
				}
			}
			if let Some(kind) = scope.function_kind(name) {
				return kind;
			}
			let _ = arity;
		} else if let Some(kind) = scope.function_kind(name) {
			return kind;
		}
		if name == crate::closures::CLOSURE_NEW {
			return Kind::Function;
		}
		// another runtime's value or a std adapter's: any Node, held like a map value (its kind decided at run time)
		if crate::host::ANY_VALUE_WORDS.contains(&name.as_str()) {
			return Kind::Empty;
		}
		// a cell's value is held as a Node, like a map value (Empty), and joins a text or adds at run time
		if crate::wasm_emitter::cells::CELL_WORDS.contains(&name.as_str()) {
			return if crate::wasm_emitter::cells::MAKING_WORDS.contains(&name.as_str()) { Kind::Data } else { Kind::Empty };
		}
		if let Some(kind) = crate::wasm_emitter::text_builtins::text_builtin_kind(name, items.len() - 1) {
			return kind;
		}
		if name == crate::warp_parser::TEXT_TIMES {
			// `it times it` of numbers multiplies (list_emitter.rs emit_text_times)
			return match items.get(2).map(|repeated| infer_type(repeated, scope)) {
				Some(kind) if kind.is_int() || kind.is_float() => arithmetic_kind(infer_type(&items[1], scope), &Op::Mul, kind),
				_ => Kind::Text,
			};
		}
		if name == crate::type_tests::TYPE_WORD && items.len() == 2 {
			return Kind::Symbol; // the type's name
		}
		// `puti x`, `puts t`: the output words give an Int (the number written, or the write's status), with or without
		// parentheses
		if crate::wasm_emitter::OUTPUT_WORDS.contains(&name.as_str()) && items.len() == 2 {
			return Kind::Int;
		}
		// `count ys`, `size t`: a number, the user's own function of that name already answered above
		if (super::counting::is_counting_word(name) || name == BYTE_SIZE) && items.len() == 2 {
			return Kind::Int;
		}
		if name == PRINT_CALL && (items.len() >= 2 || *bracket == Bracket::Round) {
			return Kind::Empty; // `print x` writes x and gives nothing (user, issue #18)
		}
	}
	// a call of a function the component being compiled imports (`host.time()`)
	if let Some(Node::Symbol(callee)) = items.first().map(Node::drop_meta) {
		if let Some(kind) = crate::wasm_emitter::component_adapters::imported_kind(callee) {
			return kind;
		}
	}
	// Check for function calls: (funcname args...) where first item is a symbol
	if items.len() >= 2 {
		if let Node::Symbol(s) = items[0].drop_meta() {
			if s == "fetch" { return Kind::Text; }
			// FFI/builtin function calls return Int by default
			// This handles strcmp, strlen, abs, etc.
			if crate::ffi::is_ffi_function(s) {
				return match items {
					[_, argument] if crate::uncertain::maps_intervals(s) => interval_or(ffi_call_kind(s), argument, scope),
					_ => ffi_call_kind(s),
				};
			}
		}
	}
	// Type constructor: int("5"), float("1.5"), str(3), double 2
	if let Some(kind) = type_cast_of(items, bracket, separator, &|name| scope.lookup(name).is_some()).and_then(type_word_kind) {
		return kind;
	}
	// Function call with parentheses: a library word has its own result, any other call is assumed Int. A comma list
	// `(y, 4)` is a tuple, never the call y(4) (`f(a, b)` parses as `(f a b)`)
	if *bracket == Bracket::Round && items.len() >= 2 && *separator != Separator::Colon {
		if let Node::Symbol(name) = items[0].drop_meta() {
			if name == crate::library_words::SLICE || name == "reverse" || name == "sort" {
				// a slice, reversal or sort of a text is a text, of a Node (known at runtime only) a Node, of anything else a list
				return match infer_type(&items[1], scope) {
					Kind::Text | Kind::Codepoint => Kind::Text,
					Kind::Empty => Kind::Empty,
					_ => Kind::List,
				};
			}
			if name == crate::library_words::FIELD_WITH || name == crate::library_words::INSTANCE_COPY {
				// the object with one field set, or its copy: a Node, whatever the object's kind is known as
				return match infer_type(&items[1], scope) { kind if kind.is_ref() => kind, _ => Kind::Empty };
			}
			if name == LIST_DROP_LAST {
				return Kind::List;
			}
			if name == REMOVED_VALUE_CALL {
				// a map's value or a list: decided at runtime (removed_value), a Node
				return Kind::Empty;
			}
			if name == crate::library_words::MAP_GET_OR {
				// `m.get(k)` is worth what `m[k]` is: a value of the map
				return element_kind(&items[1], scope).unwrap_or(Kind::Empty);
			}
			return crate::library_words::result_kind(name).unwrap_or(Kind::Int);
		}
	}
	// Zero-arg function call: (funcname) with no args
	if *bracket == Bracket::Round && items.len() == 1 {
		if let Node::Symbol(s) = items[0].drop_meta() {
			if crate::ffi::is_ffi_function(s) {
				return ffi_call_kind(s);
			}
			// `abs(c)`: a variable in parentheses is the variable
			if let Some(local) = scope.binding(s) {
				return local.kind;
			}
			// Assume zero-arg user function returns Int
			return Kind::Int;
		}
	}
	// Grouping: (x) has the type of x
	if *bracket == Bracket::Round && items.len() == 1 {
		return infer_type(&items[0], scope);
	}
	// Data list: all items are pure data, or `[…]` computing its elements → Kind::List
	let computed_elements = *bracket == Bracket::Square && !items.iter().any(|item| is_statement(item, bracket));
	if computed_elements || (!is_unbracketed_block(items, bracket, separator) && items.iter().all(is_data_node)) {
		return Kind::List;
	}
	// Statement sequence: return type of last item
	if let Some(last) = items.last() {
		infer_type(last, scope)
	} else {
		Kind::Empty
	}
}

/// Kind of an element of `indexed`: a list's common element type (`list of text` → Text), the character of a text;
/// `None` when unknown (a mixed list, a map)
pub(super) fn element_kind(indexed: &Node, scope: &Scope) -> Option<Kind> {
	if matches!(infer_type(indexed, scope), Kind::Text | Kind::Symbol) {
		return Some(Kind::Text); // a character, held as a node like any one-character text (see binding_kind)
	}
	let list_type = list_type_name(indexed, scope);
	if list_type == MAP_TYPE || list_type == NODE_LIST_TYPE || infer_type(indexed, scope) == Kind::Empty {
		return Some(Kind::Empty); // values of different types, or of a value held as a Node: held as a Node, like an optional
	}
	match list_type.strip_prefix("list of ").or_else(|| list_type.strip_prefix(MAP_TYPE_PREFIX))? { // `m[k]` of a map is a value
		word if word.starts_with("list") => Some(Kind::List),
		RATIONAL_WORD => Some(Kind::Int),
		REAL_WORD => Some(Kind::Float),
		word => match type_word_kind(word) {
			Some(Kind::Codepoint) => Some(Kind::Text),
			Some(kind) => Some(kind),
			None => Some(Kind::Empty), // instances, keys, symbols: held as Nodes
		},
	}
}

/// A branch `{a; b}` is worth its last statement
/// A branch that is `error(…)` (or a block ending in it): the bottom kind, it never gives a value to its if
pub(crate) fn raises_error(branch: &Node) -> bool {
	match branch.drop_meta() {
		Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => raises_error(&statements[statements.len() - 1]),
		other => crate::pipeline::returned_error_message(other).is_some(),
	}
}

/// The expression a block `{a; b}` yields: its last statement; `{print b}` is one expression, the call print(b), not
/// the statements `print` and `b`
pub(crate) fn block_result(block: &Node) -> Option<Node> {
	match block.drop_meta() {
		Node::List(words, Bracket::Curly, Separator::Space) if words.len() > 1 => Some(Node::List(words.clone(), Bracket::None, Separator::Space)),
		Node::List(statements, Bracket::Curly, _) => statements.last().cloned(),
		_ => None,
	}
}

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
	if then_kind == Kind::Codepoint && else_kind == Kind::Codepoint {
		Kind::Codepoint
	} else if kinds.contains(&Kind::List) {
		// a list and a list (or ø, the empty list) is a list; a list and anything else a Node of its run-time kind
		if kinds.iter().all(|kind| matches!(kind, Kind::List | Kind::Empty)) { Kind::List } else { Kind::Data }
	} else if kinds.contains(&Kind::Empty) {
		// a Node whose kind is known only at run time (an awaited job's result, ø): the value keeps that kind, so
		// `(if c then 0 else job_result) + 1` adds at run time instead of failing as text
		Kind::Empty
	} else if kinds.contains(&Kind::Data) && kinds.iter().all(|kind| matches!(kind, Kind::Int | Kind::Float | Kind::Data)) {
		// a number and a number of run-time kind (`xs#1 + rest`): a number decided at run time, not a text
		Kind::Data
	} else if [then_kind, else_kind].iter().any(|kind| kind.is_ref() || *kind == Kind::Codepoint) {
		Kind::Text
	} else if then_kind == Kind::Float || else_kind == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// `"1/3" as number`: the number a constant text spells, which the cast is (card fraction-number)
pub fn spelled_number(node: &Node) -> Option<Node> {
	let Node::Key(value, Op::As, target) = node.drop_meta() else { return None };
	let Node::Text(text) = value.drop_meta() else { return None };
	let is_number_word = matches!(target.name().to_lowercase().as_str(), "number" | "num");
	is_number_word.then(|| crate::warp_parser::number_in_text(text)).flatten().map(Node::Number)
}
