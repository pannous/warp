use crate::context::{Context, Param, UserFunctionDef};
use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::function::{Function, FunctionRegistry};
use crate::local::Local;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::type_kinds::{canonical_type_name, Kind};
use std::collections::{HashMap, HashSet};

/// Property words that count the elements of a value: `size of x`, `x size`, `x.size`; `size` is a synonym of `count`
/// Words that declare a name assignable once: `const x=5`, `final x=5`
pub const CONSTANT_KEYWORDS: [&str; 4] = ["const", "constant", "final", "val"];
/// Statement words whose argument is never their property: `return count` is no `return.count`
const PRINT_CALL: &str = "print";
/// The error for a user function named like a type word; `{name}` is the word
const TYPE_WORD_FUNCTION_CLASH: &str = "{name} is a type; rename your function";
const RETURNING_KEYWORDS: [&str; 2] = ["return", "yield"];
const PROPERTYLESS_KEYWORDS: [&str; 6] = ["return", "yield", PRINT_CALL, "println", "puts", "not"];

fn is_constant_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if CONSTANT_KEYWORDS.contains(&word.as_str()))
}

/// `var x = 1` announces a reassignable variable, plain `x = 1`
const VAR_KEYWORD: &str = "var";
/// `let x = 1` binds once, as wiki/variable.md says (unlike JS): check_constants refuses a reassignment
const IMMUTABLE_LET: &str = "let";

fn is_declaration_keyword(node: &Node) -> bool {
	is_constant_keyword(node) || is_word(node, VAR_KEYWORD) || is_word(node, IMMUTABLE_LET)
}

const COUNTING_PROPERTIES: [&str; 5] = ["number", "count", "length", "size", "len"];
/// `byte_size(x)`, `x.byte_size`: the bytes of x, as `x.bytes` (size is the element count, user decision P40)
const BYTE_SIZE: &str = "byte_size";
/// The counting properties that are also functions: `number x` is the type conversion, not a count
const TYPE_WORDS_AMONG_COUNTING: [&str; 1] = ["number"];

fn is_counting_property(word: &str) -> bool {
	COUNTING_PROPERTIES.contains(&word)
}

/// Check if a node is pure data (not a statement/function call)
fn is_data_node(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
		Node::Symbol(s) => !is_function_keyword(s),
		Node::List(items, bracket, separator) => !is_unbracketed_block(items, bracket, separator) && items.iter().all(is_data_node),
		Node::Key(_, Op::Colon, _) => true,  // Key-value pairs are data
		Node::Key(left, op, right) if op.is_arithmetic() => is_data_node(left) && is_data_node(right), // `[1+2, 3]` keeps both items
		Node::Key(left, Op::Hash, right) => is_data_node(left) && is_data_node(right), // a count or element: `[#a, a#1]`
		Node::Key(value, Op::As, _) => is_data_node(value), // `[1.5f 2.5f]`
		_ => false,
	}
}

/// An unbracketed `;`/newline list is a block: its items run in order and the last one is the value
/// (`1;2;3` → 3, `'hello';(1 2 3 4);10` → 10). A program is one, and `{…}` is a block literal that keeps its items as a
/// value until it is run as a function body or branch. `(…)` and `[…]` are lists and keep all items.
pub fn is_unbracketed_block(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	matches!(separator, Separator::Semicolon | Separator::Newline) && *bracket == Bracket::None && items.len() > 1
}

/// Items that run rather than compute a value, making their list a block: assignments, definitions, `i++`, imports.
/// Control flow counts too, except inside `[…]`: there `if c then a else b` is just a computed element, as `p#2` is.
pub fn is_statement(item: &Node, bracket: &Bracket) -> bool {
	match item.drop_meta() {
		Node::Key(_, Op::Assign | Op::Define | Op::Inc | Op::Dec, _) => true,
		Node::Key(_, op, _) if op.is_compound_assign() => true,
		Node::Key(_, Op::Then | Op::Else | Op::Do, _) => *bracket != Bracket::Square,
		Node::Key(left, Op::Colon, _) => matches!(left.drop_meta(), Node::Symbol(s) if s == "global"),
		// a lowered `for` loop: `i=a; while …`; inside `[…]` a sequence ending in a value is a computed element (an awaited
		// task, `[a, b]` of task variables)
		Node::List(list_items, Bracket::None, separator) if is_unbracketed_block(list_items, &Bracket::None, separator) => {
			*bracket != Bracket::Square || list_items.last().is_some_and(|last| is_statement(last, &Bracket::None))
		}
		// a group that runs statements, `(y=1; y)`, or prints: in a block it runs, it is not an item
		Node::List(list_items, Bracket::Round, _) if *bracket != Bracket::Square && list_items.iter().any(|inner| is_statement(inner, &Bracket::Round)) => true,
		Node::List(list_items, _, _) if *bracket != Bracket::Square && matches!(list_items.as_slice(), [word, _] if is_word(word, PRINT_CALL)) => true,
		Node::List(list_items, _, _) if list_items.len() >= 2 => {
			matches!(list_items[0].drop_meta(), Node::Symbol(s) if is_function_keyword(s) || ["use", "import", "return", crate::host::TASK_CHECK].contains(&s.as_str()))
		}
		_ => false,
	}
}

/// `{a;b;c}` as a definition body: a sequence of statements to run, unlike the value list `{1 2 3}`
fn is_statement_block(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, Separator::Semicolon | Separator::Newline) => true,
		Node::List(items, Bracket::Curly, _) => matches!(items.as_slice(), [single] if is_update(single.drop_meta())), // `{x=x+1}`, `{n++}`
		_ => false,
	}
}

/// A statement changing a variable it reads: `x=x+1`, `x+=1`, `x++` (data like `{x=1}` reads nothing)
fn is_update(node: &Node) -> bool {
	match node {
		Node::Key(target, Op::Assign, value) => matches!(target.drop_meta(), Node::Symbol(name) if crate::wasp_parser::mentions(value, name)),
		Node::Key(target, op, _) if matches!(op, Op::Inc | Op::Dec) || op.is_compound_assign() => matches!(target.drop_meta(), Node::Symbol(_)),
		_ => false,
	}
}

/// Infer the Kind for an expression
/// Returns Int, Float, Text, etc. based on the expression's result type
/// Result kind of `left op right`: exact (Int, which includes ratios like `1/4`) unless an f64 is involved
/// The runtime function of `a op b` when an operand's kind is known only at run time (a field of a map parameter, an
/// element of a parsed JSON value): Int or Float is decided by the values (wasm_emitter list_ops NODE_ARITHMETIC)
pub fn node_arithmetic(left: Kind, op: &Op, right: Kind) -> Option<&'static str> {
	let runtime_kind = |kind: &Kind| matches!(kind, Kind::Data | Kind::Empty);
	let numeric = |kind: &Kind| matches!(kind, Kind::Int | Kind::Float) || runtime_kind(kind);
	// a value of run-time kind added to a list: node_add concatenates when it is a list too
	if *op == Op::Add && [left, right].contains(&Kind::Data) && [left, right].contains(&Kind::List) {
		return Some(crate::wasm_emitter::list_ops::NODE_ADD);
	}
	if ![left, right].iter().any(runtime_kind) || ![left, right].iter().all(numeric) {
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
		|| [left, right].contains(&Kind::Empty) && [left, right].iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint)))
		|| repeats_text(left, op, right)
	{
		// concatenation; a value held as a Node (a map value, an element of one) joining a text; `"ab"*2` repeats, see WasmGcEmitter::emit_text_repeat
		Kind::Text
	} else if [left, right].iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint | Kind::List | Kind::Error)) {
		Kind::Error // no implicit conversion (DESIGN.md "Dangerous implicitness"); an error operand stays an error
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

/// `base ^ 0.5`: an exact base with a non-integral literal exponent is no exact number, it is computed as f64
pub fn arithmetic_kind_of_operands(left: Kind, op: &Op, right: Kind, right_operand: &Node) -> Kind {
	let fractional_exponent = *op == Op::Pow && matches!(right_operand.drop_meta(), Node::Number(number) if f64::from(*number).fract() != 0.0);
	match arithmetic_kind(left, op, right) {
		Kind::Int if fractional_exponent => Kind::Float,
		kind => kind,
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
		// Arithmetic: upgrade to Float if either operand is Float
		Node::Key(left, op, right) if op.is_arithmetic() => {
			arithmetic_kind_of_operands(infer_type(left, scope), op, infer_type(right, scope), right)
		}
		// Assignment/definition: type comes from value
		Node::Key(_left, Op::Define | Op::Assign, right) => {
			infer_type(right, scope)
		}
		// Compound assignment: upgrade if either side is Float
		Node::Key(left, op, right) if op.is_compound_assign() => {
			let left_kind = infer_type(left, scope);
			let right_kind = infer_type(right, scope);
			// a text joined with a text, a character or a value held as a Node: as `s + x` (arithmetic_kind)
			let joins_text = matches!(left_kind, Kind::Text | Kind::Codepoint) && arithmetic_kind(left_kind, &Op::Add, right_kind) == Kind::Text;
			if op.base_op() == Op::Add && (joins_text || crate::wasm_emitter::text_builtins::concatenates(left_kind, right_kind)) {
				Kind::Text
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
		// `v as float` is an f64; `as int`, `as exact` stay exact Ints
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => Kind::Float,
		// Comparison operators return Int (boolean as 0/1)
		Node::Key(_, op, _) if op.is_comparison() => Kind::Int,
		// √x is irrational in general: an f64
		Node::Key(left, Op::Sqrt, _) if matches!(left.drop_meta(), Node::Empty) => Kind::Float,
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
		Node::Key(indexed, Op::Hash, index) if crate::wasp_parser::subscript_key(index)
			.and_then(|key| match key.drop_meta() { Node::Text(name) => scope.function_kind(&field_kind_key(name)), _ => None })
			.is_some() && !matches!(indexed.drop_meta(), Node::Empty) => {
			let Some(Node::Text(name)) = crate::wasp_parser::subscript_key(index).map(Node::drop_meta) else { unreachable!("guarded") };
			scope.function_kind(&field_kind_key(name)).expect("guarded")
		}
		Node::Key(indexed, Op::Hash, index) if !matches!(indexed.drop_meta(), Node::Empty) => element_kind(indexed, scope).unwrap_or_else(|| {
			let by_name = crate::wasp_parser::subscript_key(index).is_some_and(|key| matches!(key.drop_meta(), Node::Text(_) | Node::Char(_))
				|| matches!(infer_type(key, scope), Kind::Text | Kind::Codepoint));
			if by_name { Kind::Empty } else { Kind::Int }
		}),
		// Default to Int for other cases
		_ => Kind::Int,
	}
}

/// The kind of a non-empty list: a call's result, a statement sequence's last value, or a data list
fn infer_list_type(node: &Node, items: &[Node], bracket: &Bracket, separator: &Separator, scope: &Scope) -> Kind {
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
		if let Some(kind) = crate::wasm_emitter::text_builtins::text_builtin_kind(name, items.len() - 1) {
			return kind;
		}
		if name == crate::wasp_parser::TEXT_TIMES {
			return Kind::Text;
		}
		if name == crate::type_tests::TYPE_WORD && items.len() == 2 {
			return Kind::Symbol; // the type's name
		}
		// `puti x`, `puts t`: the output words give an Int (the number written, or the write's status), with or without
		// parentheses
		if crate::wasm_emitter::OUTPUT_WORDS.contains(&name.as_str()) && items.len() == 2 {
			return Kind::Int;
		}
		if name == PRINT_CALL && items.len() >= 2 {
			return match crate::wasp_parser::print_arguments_of(items, bracket).as_slice() {
				[printed] => held_kind(printed, || infer_type(printed, scope)), // `print x` is worth x, `print "c"` a text
				_ => Kind::Text, // `print a, b` is worth the joined text "a b"
			};
		}
	}
	// Check for function calls: (funcname args...) where first item is a symbol
	if items.len() >= 2 {
		if let Node::Symbol(s) = items[0].drop_meta() {
			if s == "fetch" { return Kind::Text; }
			// FFI/builtin function calls return Int by default
			// This handles strcmp, strlen, abs, etc.
			if crate::ffi::is_ffi_function(s) {
				// Get actual return type from FFI signature if available
				if let Some(sig) = crate::ffi::get_ffi_signature(s) {
					if !sig.results.is_empty() {
						return match sig.results[0] {
							wasm_encoder::ValType::F64 | wasm_encoder::ValType::F32 => Kind::Float,
							wasm_encoder::ValType::Ref(_) => Kind::Empty, // a Node: task_await_value
							_ => Kind::Int,
						};
					}
				}
				return Kind::Int;
			}
		}
	}
	// Type constructor: int("5"), float("1.5"), str(3), double 2
	if items.len() == 2 {
		if let Node::Symbol(s) = items[0].drop_meta() {
			if let Some(kind) = type_word_kind(s) {
				return kind;
			}
		}
	}
	// Function call with parentheses: a library word has its own result, any other call is assumed Int. A comma list
	// `(y, 4)` is a tuple, never the call y(4) (`f(a, b)` parses as `(f a b)`)
	if *bracket == Bracket::Round && items.len() >= 2 && *separator != Separator::Colon {
		if let Node::Symbol(name) = items[0].drop_meta() {
			if name == crate::library_words::SLICE || name == "reverse" {
				// a slice or reversal of a text is a text, of anything else a list
				return if matches!(infer_type(&items[1], scope), Kind::Text | Kind::Codepoint) { Kind::Text } else { Kind::List };
			}
			if name == crate::library_words::FIELD_WITH {
				// a copy of the object with one field set: a Node, whatever the object's kind is known as
				return match infer_type(&items[1], scope) { kind if kind.is_ref() => kind, _ => Kind::Empty };
			}
			return crate::library_words::result_kind(name).unwrap_or(Kind::Int);
		}
	}
	// Zero-arg function call: (funcname) with no args
	if *bracket == Bracket::Round && items.len() == 1 {
		if let Node::Symbol(s) = items[0].drop_meta() {
			if crate::ffi::is_ffi_function(s) {
				if let Some(sig) = crate::ffi::get_ffi_signature(s) {
					if !sig.results.is_empty() {
						return match sig.results[0] {
							wasm_encoder::ValType::F64 | wasm_encoder::ValType::F32 => Kind::Float,
							wasm_encoder::ValType::Ref(_) => Kind::Empty, // a Node: task_await_value
							_ => Kind::Int,
						};
					}
				}
				return Kind::Int;
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
fn element_kind(indexed: &Node, scope: &Scope) -> Option<Kind> {
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

pub(crate) fn branch_kind(branch: &Node, scope: &Scope) -> Kind {
	match branch.drop_meta() {
		Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => branch_kind(&statements[statements.len() - 1], scope),
		// a branch yielding ø (`if c then 3 else ø`) is a Node, as a variable holding ø is
		other => held_kind(other, || infer_type(other, scope)),
	}
}

/// Either branch a reference type (Text, Symbol, List…) or a character: the value is a Node, else a number
fn branches_kind(then_kind: Kind, else_kind: Kind) -> Kind {
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
	} else if [then_kind, else_kind].iter().any(|kind| kind.is_ref() || *kind == Kind::Codepoint) {
		Kind::Text
	} else if then_kind == Kind::Float || else_kind == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// Collect variables defined in node and populate scope
/// Returns count of temp locals needed (e.g., for while loops)
pub fn collect_variables(node: &Node, scope: &mut Scope) -> u32 {
	collect_variables_inner(node, scope, false, false)
}

/// The binding a `global` declaration introduces: `global x` (an Int) and `global x=7`; it has no local slot
fn global_binding(declaration: &Node, scope: &Scope) -> Option<Local> {
	let (name, value) = match declaration.drop_meta() {
		Node::Symbol(name) => (name, &Node::Number(Number::Int(0))),
		Node::Key(target, Op::Assign | Op::Define, value) => match target.drop_meta() {
			Node::Symbol(name) => (name, value.as_ref()),
			_ => return None,
		},
		_ => return None,
	};
	let (kind, type_node) = value_binding(value, scope);
	Some(Local { type_node, ..Local::new(0, name.clone(), kind) })
}

/// Kind and type of a variable bound to `value` without a declared type; a list keeps its element type
fn value_binding(value: &Node, scope: &Scope) -> (Kind, Option<Box<Node>>) {
	let kind = binding_kind(value, scope);
	(kind, (kind == Kind::List).then(|| Box::new(Node::Symbol(list_type_name(value, scope)))))
}

/// The body of a function definition `f(x) = …`, `f(x) := …`, `def f(x) {…}`: its variables are the function's own
fn function_definition_body(node: &Node) -> Option<&Node> {
	let starts_with_symbol = |items: &[Node]| matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)));
	match node.drop_meta() {
		Node::Key(left, Op::Assign | Op::Define, body) if matches!(left.drop_meta(), Node::List(items, _, _) if starts_with_symbol(items)) => Some(body),
		Node::List(items, _, _) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(keyword) if is_function_keyword(keyword)) => items.last(),
		_ => None,
	}
}

fn collect_variables_inner(node: &Node, scope: &mut Scope, skip_first_assign: bool, in_structure: bool) -> u32 {
	let node = node.drop_meta();
	if function_definition_body(node).is_some() {
		return 0;
	}
	if let Some((names, values)) = crate::tuples::destructuring(node) {
		let temporaries = values.iter().map(|value| collect_variables_inner(value, scope, false, in_structure)).sum();
		// with a starred name the values are unpacked from one list: each name holds a Node, the star a list
		let starred = names.iter().any(|name| crate::tuples::unstarred(name) != name);
		for (index, name) in names.iter().enumerate() {
			let rest = crate::tuples::unstarred(name);
			if scope.lookup(rest).is_none() && !scope.is_global(rest) {
				let kind = match (starred, rest != name) {
					(_, true) => Kind::List,
					(true, false) => Kind::Data,
					(false, false) => destructured_kind(values, index, scope),
				};
				scope.define(rest.to_string(), None, kind);
			}
		}
		return temporaries;
	}
	match node {
		// `c ? a : b`: both branches are code, never a tag `a:b` whose body holds attributes
		Node::Key(condition, Op::Question, then_else) if matches!(then_else.drop_meta(), Node::Key(_, Op::Colon, _)) => {
			let Node::Key(then, _, otherwise) = then_else.drop_meta() else { unreachable!("guarded") };
			collect_variables_inner(condition, scope, false, in_structure)
				+ collect_variables_inner(then, scope, false, in_structure)
				+ collect_variables_inner(otherwise, scope, false, in_structure)
		}
		// Global declarations: global:Key(name, =, value) - don't create local
		// Tag structures: html:body - body is structure context (attributes, not variables)
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					// Don't define local for global variable
					// But still count any variables in the value expression
					if let Some(global) = global_binding(right, scope).filter(|global| !scope.is_global(&global.name)) {
						scope.globals.insert(global.name.clone(), global);
					}
					return collect_variables_inner(right, scope, true, false);
				}
				// Symbol:body is a tag/structure - right side is structure context
				// Inside structures, Op::Assign is attribute, not variable
				return collect_variables_inner(left, scope, false, in_structure)
					+ collect_variables_inner(right, scope, false, true);
			}
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		// Define (:=) always creates a variable, Assign (=) only outside structure context
		Node::Key(left, Op::Define, right) => {
			if !skip_first_assign {
				if let Node::Symbol(name) = left.drop_meta() {
					if scope.lookup(name).is_none() && !scope.is_global(name) {
						let kind = binding_kind(right, scope);
						scope.define(name.clone(), None, kind);
					}
				}
			}
			collect_variables_inner(right, scope, false, in_structure)
		}
		// Assign creates variables only at top level (not inside structures)
		Node::Key(left, Op::Assign, right) => {
			// the variables the value binds come first, so its kind is known: `a = (t = 1 + 2; t)` makes a an Int
			let inner_temporaries = collect_variables_inner(right, scope, false, in_structure);
			if !skip_first_assign && !in_structure {
				// Check for typed declaration: Key(Key(name, Colon, type), Assign, value)
				match left.drop_meta() {
					Node::Symbol(name) if scope.lookup(name).is_none() && !scope.is_global(name) => {
						let declared = declared_type(left);
						let (kind, type_node) = match declared {
							Some(type_name) => (declared_kind(&type_name.name()).unwrap_or_else(|| binding_kind(right, scope)), Some(Box::new(type_name.clone()))),
							None => value_binding(right, scope),
						};
						scope.define(name.clone(), type_node, kind);
					}
					Node::Symbol(name) => {
						type_list_by_first_append(name, right, scope);
						widen_to_float(scope, name, right);
						widen_to_node(scope, name, right);
					}
					// Typed variable: x:int = 1 parses as Key(Key(x, Colon, int), Assign, 1)
					Node::Key(var_name, Op::Colon, type_node) => {
						if let Node::Symbol(name) = var_name.drop_meta() {
							if scope.lookup(name).is_none() {
								// Get kind from type annotation
								let type_str = type_node.drop_meta().to_string();
								let kind = declared_kind(&type_str).unwrap_or(Kind::Int);
								scope.define(name.clone(), Some(type_node.clone()), kind);
							}
						}
					}
					_ => {}
				}
			}
			inner_temporaries
		}
		// Compound assignments don't create new variables
		Node::Key(left, op, right) if op.is_compound_assign() => {
			if let Node::Symbol(name) = left.drop_meta() {
				widen_to_float(scope, name, right);
			}
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, Op::Do, right) => {
			// While loop needs temp locals for the result and for "the body ran"
			2 + collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
			// Integer abs needs a temp local for the if-then-else pattern
			1 + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, _, right) => {
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::List(items, _, _) => {
			items.iter().map(|item| collect_variables_inner(item, scope, false, in_structure)).sum()
		}
		_ => 0,
	}
}

/// `xs = []; xs.push(5)`, lowered to `xs = xs + [5]`: the empty list takes the type of its first appended elements
fn type_list_by_first_append(name: &str, value: &Node, scope: &mut Scope) {
	let Node::Key(list, Op::Add, appended) = value.drop_meta() else { return };
	let appends_to_itself = matches!(list.drop_meta(), Node::Symbol(target) if target == name);
	if !appends_to_itself || !matches!(appended.drop_meta(), Node::List(_, Bracket::Square, _)) {
		return;
	}
	let appended_type = list_type_name(appended, scope);
	if let Some(local) = scope.locals.get_mut(name).filter(|local| local.kind == Kind::Empty && local.type_node.is_none()) {
		local.kind = Kind::List;
		local.type_node = Some(Box::new(Node::Symbol(appended_type)));
	}
}

/// An exact variable that is later assigned an f64 (`x=10; x=floor(2.5)` with libm's floor) holds an f64 throughout,
/// as an expression mixing in an f64 is one; a declared type is kept (and checked elsewhere)
fn widen_to_float(scope: &mut Scope, name: &str, value: &Node) {
	let float_value = infer_type(value, scope).is_float();
	if let Some(local) = scope.locals.get_mut(name).filter(|local| local.kind == Kind::Int && local.type_node.is_none()) {
		if float_value {
			local.kind = Kind::Float;
		}
	}
}

/// The kinds a variable may hold whose values are told apart only by representation; a variable given values of two
/// of them that do not mix (a list, then an Int; a text, then an Int) is a compile error when both kinds are evident
/// from the source (P45, check_kind_changes); where the compiler only infers them it is held as a Node of run-time kind.
/// Int and Float widen to Float instead (widen_to_float), a text and a character are both texts.
const CONCRETE_KINDS: [Kind; 6] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint, Kind::List, Kind::Symbol];

fn widen_to_node(scope: &mut Scope, name: &str, value: &Node) {
	let assigned = binding_kind(value, scope);
	// a parameter's representation comes from its calls (infer_parameters_from_calls), not from the body
	let Some(local) = scope.locals.get_mut(name).filter(|local| !local.is_param && local.type_node.as_ref().is_none_or(|_| local.kind == Kind::List)) else { return };
	let mixes = |a: Kind, b: Kind| a == b || [a, b].iter().all(|kind| matches!(kind, Kind::Int | Kind::Float)) || [a, b].iter().all(|kind| matches!(kind, Kind::Text | Kind::Codepoint));
	if CONCRETE_KINDS.contains(&local.kind) && CONCRETE_KINDS.contains(&assigned) && !mixes(local.kind, assigned) {
		local.kind = Kind::Empty;
		local.type_node = None;
	}
}

/// The kind a value evidently has as written: a literal, a list literal, or arithmetic of numbers; None for anything else
/// (a variable, a call, a loop variable), whose kind only inference knows
fn evident_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Float(_)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) | Node::Char(_) => Some(Kind::Text),
		Node::List(_, Bracket::Square, _) => Some(Kind::List),
		Node::Key(left, op, right) if op.is_arithmetic() => match (evident_kind(left)?, evident_kind(right)?) {
			(Kind::Int, Kind::Int) => Some(Kind::Int),
			(Kind::Int | Kind::Float, Kind::Int | Kind::Float) => Some(Kind::Float),
			_ => None,
		},
		_ => None,
	}
}

/// P45 (user: "compile error unless we are in script mode, which is not defined yet"): a variable given values of
/// evidently different kinds that do not mix, `x = [1]; x = 5`, in the program or in one function body
pub fn check_kind_changes(program: &Node) -> Option<Diagnostic> {
	let mut bodies = vec![program];
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			bodies.push(body);
		}
	});
	bodies.into_iter().find_map(|body| first_kind_change(body, &mut HashMap::new()))
}

/// Walks the assignments in program order, not into function definitions or lambdas (their own scopes);
/// `kinds` holds each variable's evident kind, None once it was given a value of no evident kind
fn first_kind_change(node: &Node, kinds: &mut HashMap<String, Option<Kind>>) -> Option<Diagnostic> {
	let mixes = |a: Kind, b: Kind| a == b || [a, b].iter().all(|kind| matches!(kind, Kind::Int | Kind::Float));
	match node.drop_meta() {
		Node::Key(_, Op::Define | Op::Arrow | Op::FatArrow, _) => None,
		Node::Key(target, Op::Assign, value) => {
			if let Some(change) = first_kind_change(value, kinds) {
				return Some(change);
			}
			let Node::Symbol(name) = target.drop_meta() else { return None };
			let (earlier, given) = (kinds.get(name).copied(), evident_kind(value));
			if let (Some(Some(was)), Some(now)) = (earlier, given) {
				if !mixes(was, now) {
					let message = format!("{name} was {}, is given {}: use another name", kind_with_article(was), kind_with_article(now));
					return Some(Diagnostic::at(value, message));
				}
			}
			// a first evident value fixes the kind (Int widens to Float); any value of no evident kind ends the check
			let kind = match (earlier, given) {
				(None, given) => given,
				(Some(Some(was)), Some(now)) => Some(if now == Kind::Float { now } else { was }),
				_ => None,
			};
			kinds.insert(name.clone(), kind);
			None
		}
		Node::Key(left, _, right) => first_kind_change(left, kinds).or_else(|| first_kind_change(right, kinds)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_kind_change(item, kinds)),
		_ => None,
	}
}

/// Kind of a new variable bound to `value`: `x=ø` makes x an optional, held as a Node that is ø until assigned
/// The kind of the `index`-th name of `x, y = …`: the tuple function's value kind, or that of the value in that position
fn destructured_kind(values: &[Node], index: usize, scope: &Scope) -> Kind {
	match values {
		[call] => crate::tuples::call_parts(call)
			.and_then(|(function, _)| scope.function_kinds.get(&crate::tuples::element_key(function, index)).copied())
			.unwrap_or(Kind::Data),
		_ => values.get(index).map_or(Kind::Data, |value| binding_kind(value, scope)),
	}
}

fn binding_kind(value: &Node, scope: &Scope) -> Kind {
	held_kind(value, || infer_type(value, scope))
}

/// Kind of a variable or global holding `value`, whose expression kind is `inferred`
pub(crate) fn held_kind(value: &Node, inferred: impl FnOnce() -> Kind) -> Kind {
	match value.drop_meta() {
		Node::Empty => Kind::Empty,
		Node::List(items, Bracket::Curly, _) if items.is_empty() => Kind::Empty, // the empty block is ø as well
		Node::Char(_) => Kind::Text, // a variable holding "a" may later hold "ab": one-character texts are held as nodes
		// so does a copy of a character: `f(t) { n = t; n = prev[n] }` called with `f("F")`
		Node::Symbol(_) => match inferred() {
			Kind::Codepoint => Kind::Text,
			kind => kind,
		},
		_ => inferred(),
	}
}

/// Kind of a variable declared `x:T`; an optional `T?` may hold ø, so it is held as a Node
fn declared_kind(type_name: &str) -> Option<Kind> {
	match type_name.strip_suffix('?') {
		Some(_) => Some(Kind::Empty),
		None => builtin_type_kind(type_name),
	}
}

/// Outer variables a function body reads: bound in `outer`, not a parameter or local of the body.
/// Functions capture these by value when they are defined (DESIGN.md: immutable local bindings).
pub fn captured_variables(function: &UserFunctionDef, outer: &Scope) -> Vec<(String, Kind)> {
	let mut own = Scope::new();
	for param in &function.params {
		own.define(param.name.clone(), None, param_kind(param));
	}
	collect_variables(&function.body, &mut own);
	let mut captured: Vec<(String, Kind)> = vec![];
	function.body.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			let is_new = own.lookup(name).is_none() && !captured.iter().any(|(seen, _)| seen == name);
			if let Some(local) = outer.lookup(name).filter(|_| is_new) {
				captured.push((name.clone(), local.kind));
			}
		}
	});
	captured
}

/// A function may change a main-level variable only when it is declared `global` (wiki/effects.md: State effect).
/// Assigning one from a function would otherwise silently either shadow it (Python) or mutate it (JavaScript):
/// reading or updating it first (`n += 1`) is an error that educates; a fresh `n = value` is ambiguous and asks
/// (`local-or-global`). Names the user means as main's variable become `global` declarations of the program.
pub fn resolve_main_variable_assignments(program: Node) -> Result<Node, Node> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &program);
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	let mut meant_global: Vec<String> = vec![];
	let blocks = block_function_names(&program);
	let mut outside_blocks = Scope::new(); // a variable a block binds for itself is no outer variable
	collect_variables(&without_block_bodies(program.clone(), &blocks), &mut outside_blocks);
	for function in functions {
		let is_main_variable = |name: &String| main.lookup(name).is_some() && !main.is_global(name)
			&& !function.params.iter().any(|param| param.name == *name) && !declares_local(&function.body, name);
		let mut decided: HashSet<&String> = HashSet::new();
		for (node, name) in find_assignments(&function.body, &is_main_variable) {
			if !decided.insert(name) {
				continue;
			}
			if blocks.contains(&function.name) {
				if outside_blocks.lookup(name).is_some() {
					educate_block_assignment(node, name, &function.name);
				}
				continue;
			}
			if !starts_with_fresh_binding(&function.body, name) {
				return Err(Diagnostic::at(node, format!(
					"{name} is a main-level variable: declare it `global {name}` to change it from a function, or use a new local name"))
					.fix(format!("global {name}")).into_error());
			}
			if ask_local_or_global(node, name, &function.name, main_assignment(&program, name))? == MAIN_LEVEL_READING && !meant_global.contains(name) {
				meant_global.push(name.clone());
			}
		}
	}
	Ok(declare_global(program, &meant_global))
}

/// The functions defined as a block of statements, `inc := {x = x+1}`
fn block_function_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Define, body) = node {
			if let (Node::Symbol(name), true) = (name.drop_meta(), is_statement_block(body)) {
				names.insert(name.clone());
			}
		}
	});
	names
}

fn without_block_bodies(node: Node, blocks: &HashSet<String>) -> Node {
	match node {
		Node::Key(name, Op::Define, _) if matches!(name.drop_meta(), Node::Symbol(name) if blocks.contains(name)) => Node::Empty,
		Node::Key(left, op, right) => Node::Key(Box::new(without_block_bodies(*left, blocks)), op, Box::new(without_block_bodies(*right, blocks))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| without_block_bodies(item, blocks)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_block_bodies(*node, blocks)), data },
		other => other,
	}
}

/// A block captures the outer variables by value (user decision D7): its assignment changes the block's own copy,
/// so `x=1; inc:={x=x+1}; do inc; x` stays 1. Educate toward the forms that change x: `global x`, or returning the value.
fn educate_block_assignment(assignment: &Node, name: &str, block: &str) {
	crate::normalize::set_position_of(assignment);
	let written = crate::normalize::operand_text(assignment);
	let reason = format!("the block {block} captures {name} by value: its change stays inside the block");
	crate::normalize::advise(&written, &format!("global {name}"), &reason);
	crate::normalize::advise(&written, &format!("{name} = {block}()"), "or return the value from the block and assign it");
}

const LOCAL_OR_GLOBAL: &str = "local-or-global";
const LOCAL_KEYWORDS: [&str; 2] = ["let", "var"];
const MAIN_LEVEL_READING: usize = 1;

/// The main-level statement that first assigns `name`: `n=0` of `n=0; def f(x){n=5;x}`
fn main_assignment<'a>(program: &'a Node, name: &str) -> Option<&'a Node> {
	let statements = match program.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		_ => std::slice::from_ref(program),
	};
	statements.iter().find(|statement| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(assigned) if assigned == name)))
}

/// `n = …` inside f where main has an n: a new local of f (the default, as in Python) or main's n? The fix of main's n
/// declares it where main assigns it: `global n=0`
fn ask_local_or_global(assignment: &Node, name: &str, function: &str, main_assignment: Option<&Node>) -> Result<usize, Node> {
	use crate::diagnostic::{ask, reading, Ask, Fallback};
	let question = format!("does `{name} = …` inside {function} make a new local of {function}, or change the main-level {name}?");
	let main_level = reading(&format!("the main-level {name}"), &format!("global {name}"));
	let main_level = match main_assignment.and_then(crate::diagnostic::position) {
		Some((line, column)) => {
			let fix = crate::fixits::fix(&main_level.meaning, format!("{name} ="), format!("global {name} ="));
			main_level.fixed_by(fix.at(line, column))
		}
		None => main_level.replacing("", ""), // no main-level assignment to declare: no edit
	};
	let readings = vec![reading(&format!("a new local of {function}"), &format!("let {name} = …")).replacing(format!("{name} ="), format!("let {name} =")), main_level];
	ask(&Ask::new(LOCAL_OR_GLOBAL, question, readings, Fallback::Warning).written(&format!("{name} = …")).at_node(assignment))
}

/// `let n = …` / `var n = …` in `body`: n is explicitly the function's own
fn declares_local(body: &Node, name: &str) -> bool {
	let mut declared = false;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let [keyword, declaration] = items.as_slice() {
				declared |= matches!(keyword.drop_meta(), Node::Symbol(word) if LOCAL_KEYWORDS.contains(&word.as_str()))
					&& global_binding(declaration, &Scope::new()).is_some_and(|binding| binding.name == name);
			}
		}
	});
	declared
}

/// The program's first main-level assignment of each name becomes its `global` declaration (`global n` if none)
pub(crate) fn declare_global(program: Node, names: &[String]) -> Node {
	if names.is_empty() {
		return program;
	}
	let as_global = |declaration: Node| Node::Key(Box::new(Node::Symbol("global".to_string())), Op::Colon, Box::new(declaration));
	let (mut items, bracket, separator) = match program {
		Node::List(items, bracket, separator) => (items, bracket, separator),
		single => (vec![single], Bracket::None, Separator::Newline),
	};
	for name in names {
		let assigns = |item: &Node| matches!(item.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(target) if target == name));
		match items.iter().position(assigns) {
			Some(index) => items[index] = as_global(items[index].clone()),
			None => items.insert(0, as_global(Node::Symbol(name.clone()))),
		}
	}
	Node::List(items, bracket, separator)
}

/// Is the first mention of `name` in `body` a plain `name = value` not reading it (`primes = []`)? Then the body
/// binds its own local, as in Python; reading or updating it first (`n += 1`, `xs#i = v`) means main's variable.
fn starts_with_fresh_binding(body: &Node, name: &str) -> bool {
	let mut first_mention = None;
	body.visit(&mut |node| {
		if first_mention.is_some() {
			return;
		}
		match node {
			Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(target) if target == name) => {
				let mut reads = false;
				value.visit(&mut |part| reads |= matches!(part, Node::Symbol(symbol) if symbol == name));
				first_mention = Some(!reads);
			}
			Node::Symbol(symbol) if symbol == name => first_mention = Some(false),
			_ => {}
		}
	});
	first_mention.unwrap_or(false)
}

/// The assignments (kept with their positions) whose changed variable satisfies `wanted`, in source order
pub(crate) fn find_assignments<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	let mut found: Vec<(&'a Node, &'a String)> = assignment_target_root(node.drop_meta()).filter(|name| wanted(name)).map(|name| (node, name)).into_iter().collect();
	match node.drop_meta() {
		Node::Key(left, _, right) => found.extend(find_assignments(left, wanted).into_iter().chain(find_assignments(right, wanted))),
		Node::List(items, _, _) => found.extend(items.iter().flat_map(|item| find_assignments(item, wanted))),
		_ => {}
	}
	found
}

/// The variable an assignment changes: `n` in `n = …`, `n += …`, `n++` and `xs#i = …`
fn assignment_target_root(node: &Node) -> Option<&String> {
	let target = match node {
		Node::Key(target, Op::Assign | Op::Inc | Op::Dec, _) => target,
		Node::Key(target, op, _) if op.is_compound_assign() => target,
		_ => return None,
	};
	let mut target = target.drop_meta();
	while let Node::Key(container, Op::Hash | Op::Dot, _) = target {
		target = container.drop_meta();
	}
	match target {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

/// Scope for tracking variable bindings
#[derive(Clone, Debug, Default)]
pub struct Scope {
	pub locals: HashMap<String, Local>,
	pub types: HashMap<String, Node>,  // User-defined types
	pub function_kinds: HashMap<String, Kind>,  // Return kinds of user functions, which shadow FFI names like `pow`
	/// Variables that hold a closure → `closure_new` targets they may contain (per-site closure_call kinds)
	pub closure_variable_targets: HashMap<String, HashSet<String>>,
	pub globals: HashMap<String, Local>,  // Declared `global` (kind and type, no slot): assigning them later must not create a shadowing local
	pub parent: Option<Box<Scope>>,
}

impl Scope {
	pub fn new() -> Self {
		Scope::default()
	}

	pub fn child(&self) -> Self {
		Scope {
			locals: HashMap::new(),
			types: HashMap::new(),
			function_kinds: HashMap::new(),
			closure_variable_targets: HashMap::new(),
			globals: HashMap::new(),
			parent: Some(Box::new(self.clone())),
		}
	}

	pub fn with_function_kinds(function_kinds: HashMap<String, Kind>) -> Self {
		Scope { function_kinds, ..Scope::default() }
	}

	pub fn with_closure_targets(mut self, closure_variable_targets: HashMap<String, HashSet<String>>) -> Self {
		self.closure_variable_targets = closure_variable_targets;
		self
	}

	/// Targets a closure variable may hold, looking through parent scopes
	pub fn closure_targets_of(&self, name: &str) -> Option<&HashSet<String>> {
		self.closure_variable_targets.get(name).or_else(|| self.parent.as_ref().and_then(|parent| parent.closure_targets_of(name)))
	}

	pub fn is_global(&self, name: &str) -> bool {
		self.global(name).is_some()
	}

	pub fn global(&self, name: &str) -> Option<&Local> {
		self.globals.get(name).or_else(|| self.parent.as_ref().and_then(|parent| parent.global(name)))
	}

	/// The local or declared global a name refers to, for its kind and type
	pub fn binding(&self, name: &str) -> Option<&Local> {
		self.lookup(name).or_else(|| self.global(name))
	}

	pub fn function_kind(&self, name: &str) -> Option<Kind> {
		self.function_kinds.get(name).copied().or_else(|| self.parent.as_ref().and_then(|p| p.function_kind(name)))
	}

	/// Look up a variable by name, checking parent scopes
	pub fn lookup(&self, name: &str) -> Option<&Local> {
		self.locals.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup(name)))
	}

	/// The names of the variables in this scope and its parents
	pub fn local_names(&self) -> Vec<String> {
		let mut names: Vec<String> = self.locals.keys().cloned().collect();
		if let Some(parent) = &self.parent {
			names.extend(parent.local_names());
		}
		names
	}

	/// Define a new variable in current scope
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, kind: Kind) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			position,
			is_param: false,
			kind,
			data_pointer: 0,
			data_length: 0,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Define a function parameter
	pub fn define_param(&mut self, name: String, kind: Kind) -> Local {
		let mut local = self.define(name.clone(), None, kind);
		local.is_param = true;
		self.locals.insert(name, local.clone());
		local
	}

	/// Update a local's data pointer and length (for string assignments)
	pub fn set_local_data(&mut self, name: &str, pointer: u32, length: u32) {
		if let Some(local) = self.locals.get_mut(name) {
			local.data_pointer = pointer;
			local.data_length = length;
		}
	}

	/// Define a type in current scope
	pub fn define_type(&mut self, name: String, def: Node) {
		self.types.insert(name, def);
	}

	/// Look up a type by name
	pub fn lookup_type(&self, name: &str) -> Option<&Node> {
		self.types.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup_type(name)))
	}

	/// Get total number of locals (for WASM local declaration)
	pub fn local_count(&self) -> u32 {
		self.locals.len() as u32
	}
}

pub fn analyze(raw: Node) -> Node {
	let mut scope = Scope::new();
	if let Some(err) = check_type_errors(&raw, &mut scope) {
		return err;
	}
	raw
}

/// Check for type errors in the AST, returns Some(Node::Error) if found
fn check_type_errors(node: &Node, scope: &mut Scope) -> Option<Node> {
	check_type_errors_inner(node, scope, false)
}

fn check_type_errors_inner(node: &Node, scope: &mut Scope, in_structure: bool) -> Option<Node> {
	let node = node.drop_meta();
	if let Some(body) = function_definition_body(node) {
		return check_type_errors_inner(body, &mut Scope::new(), false);
	}
	match node {
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					return check_type_errors_inner(right, scope, false);
				}
				// Tag structure: check both sides, right is structure context
				if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
					return Some(err);
				}
				return check_type_errors_inner(right, scope, true);
			}
			if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
				return Some(err);
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, Op::Define, right) => {
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(err) = check_assignment(name, right, scope) {
					return Some(err);
				}
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, Op::Assign, right) => {
			if !in_structure {
				if let Node::Symbol(name) = left.drop_meta() {
					if let Some(err) = check_assignment(name, right, scope) {
						return Some(err);
					}
				}
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, _, right) => {
			if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
				return Some(err);
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::List(items, _, _) => {
			for item in items {
				if let Some(err) = check_type_errors_inner(item, scope, in_structure) {
					return Some(err);
				}
			}
			None
		}
		_ => None,
	}
}

/// `name = value` must keep the variable's kind; a new variable takes the value's kind
fn check_assignment(name: &str, value: &Node, scope: &mut Scope) -> Option<Node> {
	match scope.lookup(name) {
		Some(existing) => {
			let new_kind = written_kind(value, scope);
			(!types_compatible(existing.kind, new_kind)).then(|| type_error(name, existing.kind, new_kind))
		}
		None => {
			let kind = infer_type(value, scope);
			scope.define(name.to_string(), None, kind);
			None
		}
	}
}

fn type_error(name: &str, existing: Kind, new: Kind) -> Node {
	Node::Error(Box::new(Node::Text(format!(
		"type mismatch: cannot assign {} to variable '{}' of type {}",
		new, name, existing
	))))
}

/// Check if two types are compatible for assignment
fn types_compatible(existing: Kind, new: Kind) -> bool {
	match (existing, new) {
		// Same type is always compatible
		(a, b) if a == b => true,
		// Int and Float are NOT compatible (x=1; x=1.0 should fail)
		(Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => false,
		// Text and other types are NOT compatible
		(Kind::Text, _) | (_, Kind::Text) => false,
		// Codepoint and Int may be compatible (char as number)
		(Kind::Int, Kind::Codepoint) | (Kind::Codepoint, Kind::Int) => true,
		// Default: incompatible
		_ => false,
	}
}

/// Collect all function declarations from the AST into a FunctionRegistry
pub fn collect_functions(node: &Node) -> FunctionRegistry {
	let mut registry = FunctionRegistry::new();
	collect_functions_inner(node, &mut registry);
	registry
}

fn collect_functions_inner(node: &Node, registry: &mut FunctionRegistry) {
	let node = node.drop_meta();
	match node {
		// Pattern: fun/fn/def/define/function name(params...) body
		Node::List(items, _, _) if items.len() >= 2 => {
			if let Node::Symbol(keyword) = items[0].drop_meta() {
				if is_function_keyword(keyword) {
					if let Some(func) = parse_function_declaration(items, keyword) {
						registry.register(func);
						return;
					}
				}
			}
			// Recurse into list items
			for item in items {
				collect_functions_inner(item, registry);
			}
		}
		Node::Key(left, _, right) => {
			collect_functions_inner(left, registry);
			collect_functions_inner(right, registry);
		}
		_ => {}
	}
}

/// Parse a function declaration from a list starting with fun/fn/def/define/function
fn parse_function_declaration(items: &[Node], _keyword: &str) -> Option<Function> {
	// Structure: [keyword, ((name (type param)...) body)]
	// or: [keyword, ((name params...) body)]
	if items.len() < 2 {
		return None;
	}

	let decl = items[1].drop_meta();

	// Get function name and params from the declaration structure
	let (name, params, body) = match decl {
		// Pattern: ((name params...) body)
		Node::List(decl_items, _, _) if !decl_items.is_empty() => {
			let first = decl_items[0].drop_meta();
			match first {
				// (name params...)
				Node::List(name_params, _, _) if !name_params.is_empty() => {
					let func_name = name_params[0].name();
					let params = &name_params[1..];
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(func_name, params.to_vec(), body)
				}
				// Just a name symbol
				Node::Symbol(name) => {
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(name.clone(), Vec::new(), body)
				}
				_ => return None,
			}
		}
		_ => return None,
	};

	if name.is_empty() {
		return None;
	}

	let mut func = Function::new(&name);
	func.body = body;

	// Parse parameters
	for param in &params {
		let (param_name, param_kind) = parse_param(param);
		func.signature.add(&param_name, param_kind);
	}

	Some(func)
}

/// Parse a parameter node into (name, kind)
fn parse_param(param: &Node) -> (String, Kind) {
	let param = param.drop_meta();
	match param {
		// Pattern: (name:type) - single item list containing a Key
		Node::List(items, _, _) if items.len() == 1 => {
			parse_param(&items[0])
		}
		// Pattern: (type name) e.g., (float a)
		Node::List(items, _, _) if items.len() >= 2 => {
			let type_name = items[0].name();
			let param_name = items[1].name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Pattern: name:type
		Node::Key(left, Op::Colon, right) => {
			let param_name = left.name();
			let type_name = right.name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Just a name (infer type later)
		Node::Symbol(name) => (name.clone(), Kind::Int),
		_ => (String::new(), Kind::Int),
	}
}

/// Convert type name string to Kind
fn type_name_to_kind(name: &str) -> Kind {
	builtin_type_kind(name).unwrap_or(Kind::Int)
}

/// Kind of a built-in type name; an exact number (`exact`, `real`) is an Int that may hold a ratio (wasm_emitter/exact.rs)
pub fn builtin_type_kind(name: &str) -> Option<Kind> {
	Some(match canonical_type_name(&name.to_lowercase()) {
		"int" | "i32" | "i64" | "integer" | "long" | "exact" => Kind::Int,
		"float" | "f32" | "number" => Kind::Float,
		"string" | "str" | "text" => Kind::Text,
		"bool" | "boolean" => Kind::Int, // Booleans are i32/i64
		"char" | "codepoint" => Kind::Codepoint,
		"function" | "closure" => Kind::Function,
		fixed if crate::fixed_width::fixed_width(fixed).is_some() => Kind::Int,
		_ => return None,
	})
}

/// A plural type word denotes a list of that type: `ints`, `numbers` → `int`, `number`
pub fn plural_element_type(word: &str) -> Option<&str> {
	let singular = word.strip_suffix('s')?;
	type_word_kind(singular).map(|_| singular)
}

/// The type name `type(x)` reports for a list: a list whose elements are held as Nodes is a plain `list`
pub fn shown_list_type_name(list: &Node, scope: &Scope) -> String {
	let type_name = list_type_name(list, scope);
	if type_name == NODE_LIST_TYPE { LIST_WORD.to_string() } else { type_name }
}

/// The type name of a list: `list of int` when all items share a kind (or the variable is declared `ints`), else `list`
pub fn list_type_name(list: &Node, scope: &Scope) -> String {
	const PLAIN: &str = "list";
	match list.drop_meta() {
		Node::Symbol(name) => {
			let type_name = scope.binding(name).and_then(|local| local.type_node.as_ref()).map(|type_node| type_node.name());
			match type_name {
				Some(word) => match plural_element_type(&word) {
					Some(element) => format!("{PLAIN} of {element}"),
					None if word.starts_with(PLAIN) || word.starts_with(MAP_TYPE) => word,
					None => PLAIN.to_string(),
				},
				// `r = f()`: the list a function returns, its elements unknown until runtime
				None if scope.binding(name).is_none() && scope.function_kind(name) == Some(Kind::List) => NODE_LIST_TYPE.to_string(),
				// a list parameter of no declared element type: each caller may pass other elements
				None if scope.binding(name).is_some_and(|local| local.is_param) => NODE_LIST_TYPE.to_string(),
				None => PLAIN.to_string(),
			}
		}
		// Grouping: `([1 2])` has the type of `[1 2]`, as in infer_type
		Node::List(items, Bracket::Round, _) if items.len() == 1 => list_type_name(&items[0], scope),
		// a block `(out = ø; for …; out)` is worth its last value
		Node::List(items, Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline) if !items.is_empty() => list_type_name(&items[items.len() - 1], scope),
		// `zero_fill(count, zero)`: a list of the zero's type
		Node::List(items, _, _) if matches!(items.as_slice(), [call, _, _] if matches!(call.drop_meta(), Node::Symbol(name) if name == ZERO_FILL_CALL)) => {
			format!("{PLAIN} of {}", element_type_word(&items[2], scope))
		}
		Node::List(items, Bracket::Round, _) if items.len() == 2 && map_word(&items[0]).is_some() => {
			use crate::library_words::{MAP_ENTRIES, MAP_KEYS};
			match map_word(&items[0]) {
				Some(MAP_KEYS) => format!("{PLAIN} of text"), // the keys of a map are read as texts
				Some(MAP_ENTRIES) => list_type_name(&items[1], scope),
				_ => list_type_name(&items[1], scope).replacen(MAP_TYPE_PREFIX, &format!("{PLAIN} of "), 1), // the values
			}
		}
		// `sort(xs)`, `reverse(xs)`: the same elements
		Node::List(items, Bracket::Round, Separator::None) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if ORDER_WORDS.contains(&word.as_str())) => {
			list_type_name(&items[1], scope)
		}
		// the result of a call `f(x)` (no list literal of `f` and `x`): its elements are held as Nodes, of any type
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => NODE_LIST_TYPE.to_string(),
		// a `{key:value …}` map, `map of int` when all its values are ints
		Node::List(entries, Bracket::Curly, _) if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) => {
			let value_word = |value: &Node| match infer_type(value, scope) {
				Kind::List => list_type_name(value, scope),
				_ => element_type_word(value, scope),
			};
			let values: Vec<&Node> = entries.iter().filter_map(|entry| match entry.drop_meta() {
				Node::Key(_, _, value) => Some(value.as_ref()),
				_ => None,
			}).collect();
			// an empty `[]` fits any type of list: `{A:[["B", 4]], B:[]}` is a `map of list of list`
			let is_empty_list = |value: &&Node| matches!(value.drop_meta(), Node::Empty) || matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if items.is_empty());
			let typed: Vec<&Node> = values.iter().copied().filter(|value| !is_empty_list(value)).collect();
			let words: Vec<String> = if typed.is_empty() { &values } else { &typed }.iter().map(|value| value_word(value)).collect();
			let is_list_map = typed.len() < values.len() && words.iter().all(|word| word.starts_with(PLAIN));
			match common_type_word(&words) {
				Some(word) if is_list_map || typed.len() == values.len() => format!("{MAP_TYPE_PREFIX}{word}"),
				_ => MAP_TYPE.to_string(),
			}
		}
		// a value of a map: `graph["A"]` of a `map of list of int` is a `list of int`
		Node::Key(map, Op::Hash, _) => match list_type_name(map, scope).strip_prefix(MAP_TYPE_PREFIX) {
			Some(value_type) if value_type.starts_with(PLAIN) => value_type.to_string(),
			_ => PLAIN.to_string(),
		},
		Node::List(items, _, separator) => {
			let words: Vec<String> = items.iter().map(|item| element_type_word(item, scope)).collect();
			let is_block = matches!(separator, Separator::Semicolon | Separator::Newline);
			match common_type_word(&words) {
				Some(word) => format!("{PLAIN} of {word}"),
				// `[1 "a"]`: elements of different kinds, each held as its Node
				None if !words.is_empty() && !is_block => NODE_LIST_TYPE.to_string(),
				None => PLAIN.to_string(),
			}
		}
		_ => PLAIN.to_string(),
	}
}

/// The type name of a map literal, `map of <value type>` when its values share one
const MAP_TYPE: &str = "map";
/// Library words whose list result has the elements of their list argument
const ORDER_WORDS: [&str; 2] = ["sort", "reverse"];
const MAP_TYPE_PREFIX: &str = "map of ";
/// A list whose elements are known only at runtime (the result of a call): each element is held as a Node
const NODE_LIST_TYPE: &str = "list of node";
const LIST_WORD: &str = "list";
/// Between a variable and the name of a temporary made for it (`xs·range`, `xs·item`, `m·removed`)
const TEMPORARY_SEPARATOR: &str = "·";
/// Statements that name functions or modules instead of calling them
const IMPORT_WORDS: [&str; 3] = ["import", "use", "include"];
const LIST_OF_PREFIX: &str = "list of ";
const OF_WORD: &str = "of";

/// The map word a call names: `map_keys`, `map_values` or `map_entries`
fn map_word(call: &Node) -> Option<&'static str> {
	use crate::library_words::{MAP_ENTRIES, MAP_KEYS, MAP_VALUES};
	[MAP_KEYS, MAP_VALUES, MAP_ENTRIES].into_iter().find(|word| matches!(call.drop_meta(), Node::Symbol(name) if name == word))
}

const INT_WORD: &str = "int";
const RATIONAL_WORD: &str = "rational";
const FLOAT_WORD: &str = "float";
const NUMBER_WORD: &str = "number";
/// Irrational constants are `real`, although the underlying representation may still be exact or float: the type name is
/// the contract, the representation may change
const REAL_WORD: &str = "real";
const REAL_CONSTANTS: [&str; 2] = ["π", "pi"];

/// The type word of a number literal: whole numbers are `int` (also `2.0`), exact fractions and decimals `rational`,
/// approximations (`1.5f`, √2, complex) `float`
pub fn number_type_word(number: &Number) -> &'static str {
	match number {
		Number::Int(_) | Number::BigInt(_) => INT_WORD,
		Number::Quotient(..) | Number::BigQuotient(_) => RATIONAL_WORD,
		Number::Real(_) => REAL_WORD,
		Number::Float(value) if Number::is_exact_decimal(*value) => if value.fract() == 0.0 { INT_WORD } else { RATIONAL_WORD },
		_ => FLOAT_WORD,
	}
}

/// The type word of a number written as a literal or as a quotient of integer literals (`3/4`); `None` for any other node
pub fn literal_number_type_word(node: &Node) -> Option<&'static str> {
	match node.drop_meta() {
		Node::Number(number) => Some(number_type_word(number)),
		Node::Symbol(name) if REAL_CONSTANTS.contains(&name.as_str()) => Some(REAL_WORD),
		Node::Key(numerator, Op::Div, denominator) => match (numerator.drop_meta(), denominator.drop_meta()) {
			(Node::Number(Number::Int(numerator)), Node::Number(Number::Int(denominator))) if *denominator != 0 => {
				Some(if numerator % denominator == 0 { INT_WORD } else { RATIONAL_WORD })
			}
			_ => None,
		},
		_ => None,
	}
}

fn element_type_word(item: &Node, scope: &Scope) -> String {
	literal_number_type_word(item).map(str::to_string).unwrap_or_else(|| infer_type(item, scope).to_string())
}

/// The one type word all element words fit: the same word, `rational` for a mix of `int` and `rational` (int is a special
/// case of rational), `number` for any other mix of numbers; `None` when the elements are not all numbers or all alike
fn common_type_word(words: &[String]) -> Option<String> {
	let first = words.first()?;
	if words.iter().all(|word| word == first) {
		return Some(first.clone());
	}
	let is_number = |word: &String| [INT_WORD, RATIONAL_WORD, REAL_WORD, FLOAT_WORD, NUMBER_WORD].contains(&word.as_str());
	let is_exact = |word: &String| word == INT_WORD || word == RATIONAL_WORD;
	if !words.iter().all(is_number) {
		return None;
	}
	Some(if words.iter().all(is_exact) { RATIONAL_WORD } else { NUMBER_WORD }.to_string())
}

/// Semantic checks run before emission; the first violation comes back as an error value
pub fn diagnose(program: &Node) -> Option<Node> {
	check_type_word_functions(program)
		.or_else(|| check_parameter_annotations(program))
		.or_else(|| check_declared_types(program, &mut HashMap::new()))
		.or_else(|| check_constants(program, &mut HashMap::new()))
		.or_else(|| check_null_use(program, &mut HashMap::new()))
		.or_else(|| check_boolean_arithmetic(program))
		.or_else(|| check_ambiguous_calls(program))
		.or_else(|| check_call_arity(program))
		.map(Diagnostic::into_error)
}

/// `sin(0, 5)`, `pow(2)`, `√(16, 2)`: a math function or a one-value operator given another number of values, which
/// used to drop or invent values silently
fn check_call_arity(program: &Node) -> Option<Diagnostic> {
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	call_arity_error(program, &context)
}

fn call_arity_error(node: &Node, context: &Context) -> Option<Diagnostic> {
	let values = |count: usize| if count == 1 { "1 value".to_string() } else { format!("{count} values") };
	match node {
		Node::Meta { node, .. } => call_arity_error(node, context),
		// `import (sin, floor) from 'm'` names functions, it calls none
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if IMPORT_WORDS.contains(&word.as_str())) => None,
		Node::List(items, bracket, _) => {
			if let (Bracket::Round, Some(Node::Symbol(name))) = (bracket, items.first().map(Node::drop_meta)) {
				let libm = crate::ffi::LIBM_F64_FUNCTIONS.iter().filter(|(function, _)| function == name).map(|(_, arity)| *arity);
				let arities: Vec<usize> = libm.chain(crate::wasm_emitter::text_builtins::text_builtin_arities(name)).collect();
				let given = items.len() - 1;
				if !arities.is_empty() && !context.user_functions.contains_key(name) && !arities.contains(&given) {
					return Some(Diagnostic::at(node, format!("{name} takes {}, got {given}", values(arities[0]))));
				}
			}
			items.iter().find_map(|item| call_arity_error(item, context))
		}
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs), operand) if empty.is_nothing() => match operand.drop_meta() {
			Node::List(items, Bracket::Round, Separator::Colon) if items.len() > 1 => Some(Diagnostic::at(node, format!("{op} takes 1 value, got {}", operand.serialize()))),
			_ => call_arity_error(operand, context),
		},
		Node::Key(left, _, right) => call_arity_error(left, context).or_else(|| call_arity_error(right, context)),
		_ => None,
	}
}

/// `double := it*2` or `double(x) := …`: a type word names a type, never a function (user decision P20)
fn check_type_word_functions(program: &Node) -> Option<Diagnostic> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	let mut clashes: Vec<&String> = ctx.user_functions.keys().filter(|name| type_word_kind(name).is_some()).collect();
	clashes.sort();
	let name = clashes.first()?;
	let mut definition = None;
	program.visit(&mut |node| {
		if let (None, Node::Key(left, Op::Define | Op::Assign, _)) = (definition, node) {
			let head = match left.drop_meta() {
				Node::List(items, _, _) => items.first().map(Node::drop_meta),
				other => Some(other),
			};
			if matches!(head, Some(Node::Symbol(word)) if word == *name) {
				definition = Some(node);
			}
		}
	});
	Some(Diagnostic::at(definition.unwrap_or(program), TYPE_WORD_FUNCTION_CLASH.replace("{name}", name)))
}

/// Warnings that do not stop compilation: well-defined code that likely does not mean what it says
pub fn lint(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	lint_into(program, &mut warnings);
	warnings.extend(kebab_ambiguities(program));
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			if uses_it_outside_loops(body) {
				hidden_function_it(body, &mut warnings);
			}
		}
	});
	warnings
}

/// `for 1..4 {…}`: a loop that binds the implicit `it`
/// `for 1..4 {…}` and `for i in xs {…}`: loops that bind the implicit `it` (to the item); their iterable and body
fn it_loop(items: &[Node]) -> Option<(&Node, &Node)> {
	let (keyword, iterable, body) = match items {
		[keyword, iterable, body] => (keyword, iterable, body),
		[keyword, _, in_word, iterable, body] if is_word(in_word, "in") => (keyword, iterable, body),
		_ => return None,
	};
	(is_word(keyword, "for") && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))).then_some((iterable, body))
}

/// Does a function body read its implicit parameter `it`, not counting the `it` its loops bind?
fn uses_it_outside_loops(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => word == "it",
		Node::Key(left, _, right) => uses_it_outside_loops(left) || uses_it_outside_loops(right),
		Node::List(items, _, _) if it_loop(items).is_some() => it_loop(items).is_some_and(|(iterable, _)| uses_it_outside_loops(iterable)),
		Node::List(items, _, _) => items.iter().any(uses_it_outside_loops),
		_ => false,
	}
}

/// The name the fix of a hidden `it` gives the function's `it`, so the loop can read it
const FUNCTION_IT: &str = "outer_it";

/// The loops in a function with an implicit `it` whose own `it` hides the function's (user decision #23: kept, warned)
fn hidden_function_it(body: &Node, warnings: &mut Vec<Diagnostic>) {
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if it_loop(items).is_some_and(|(_, body)| uses_it(body)) {
				let renamed = crate::library_words::substitute(node.clone(), "it", &Node::Symbol(FUNCTION_IT.to_string()));
				let explicit = format!("{FUNCTION_IT}=it; {}", renamed.serialize().trim());
				warnings.push(Diagnostic::at(node, "the loop's `it` hides the function's `it` inside the loop")
					.fix(format!("name the function's it before the loop: {FUNCTION_IT}=it; for … {{… {FUNCTION_IT} …}}"))
					.offer("the function's `it` inside the loop", node.serialize().trim(), explicit));
			}
		}
	});
}

fn lint_into(node: &Node, warnings: &mut Vec<Diagnostic>) {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			if let (Op::Or, Node::Key(condition, Op::And, then)) = (op, left.drop_meta()) {
				let (condition, then, otherwise) = (condition.serialize(), then.serialize(), right.serialize());
				let explicit = format!("if {condition} then {then} else {otherwise}");
				warnings.push(Diagnostic::at(node, format!("`{condition} and {then} or {otherwise}` yields {otherwise} whenever {then} is falsy"))
					.fix(&explicit).offer(format!("{then} whenever {condition} holds"), format!("{condition} and {then} or {otherwise}"), explicit));
			}
			if *op == Op::As && is_arithmetic(left) {
				let diagnostic = Diagnostic::at(node, conversion_of_arithmetic_warning(left, right));
				let written = format!("{} as {}", left.serialize(), right.serialize());
				let readings = conversion_readings(left, right);
				let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
				let diagnostic = diagnostic.fix(forms.join(" or "));
				warnings.push(readings.iter().fold(diagnostic, |diagnostic, (meaning, form)| diagnostic.offer(meaning, &written, form)));
			}
			if *op == Op::Mod && (is_negative(left) || is_negative(right)) {
				let (a, b) = (left.serialize(), right.serialize());
				warnings.push(Diagnostic::at(node, negative_modulo_warning(left, right))
					.offer("the truncated remainder of C/Java/JS", format!("{} % {}", a.trim(), b.trim()), format!("{} rem {}", a.trim(), b.trim())));
			}
			lint_into(left, warnings);
			lint_into(right, warnings);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| lint_into(item, warnings)),
		_ => {}
	}
}

/// `name:literal` as a data binding: the name and its value
fn data_binding(item: &Node) -> Option<(&str, &Node)> {
	let Node::Key(key, Op::Colon, value) = item.drop_meta() else { return None };
	let (Node::Symbol(name), Node::Number(_) | Node::Text(_) | Node::Char(_)) = (key.drop_meta(), value.drop_meta()) else { return None };
	Some((name, value.drop_meta()))
}

fn assigned_names(program: &Node) -> HashSet<&str> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, _) = node {
			if let Node::Symbol(name) = target.drop_meta() {
				names.insert(name.as_str());
			}
		}
	});
	names
}

/// A data key `a-b:2` also reads as the subtraction `a - b` when `a` and `b` are variables
fn kebab_ambiguities(program: &Node) -> Vec<Diagnostic> {
	let variables = assigned_names(program);
	let mut warnings = vec![];
	program.visit(&mut |item| {
		let Some((name, _)) = data_binding(item) else { return };
		let parts: Vec<&str> = name.split('-').collect();
		if parts.len() < 2 || !parts.iter().all(|part| variables.contains(part)) {
			return;
		}
		let message = format!("`{name}` is a data key here, but {} are also variables: `{}` would subtract", parts.join(" and "), parts.join(" - "));
		warnings.push(Diagnostic::at(item, message).fix(format!("write {} for the subtraction", parts.join(" - "))));
	});
	warnings
}

/// A symbol that is not a variable reads the value of the data key of that name given earlier in the same block:
/// `a-b:2 c-d:4 a-b` is 2 (the data is the scope)
pub fn resolve_data_scope(program: Node) -> Node {
	let mut variables: HashSet<String> = assigned_names(&program).into_iter().map(String::from).collect();
	let mut context = Context::new();
	extract_user_functions(&mut context, &program);
	variables.extend(context.user_functions.values().flat_map(|function| function.params.iter().map(|param| param.name.clone())));
	let resolved = resolve_blocks(program, &variables);
	subtract_kebab_variables(resolved, &variables)
}

/// A hyphenated name that is no data key and whose parts are all variables is their difference: `a=5; b=1; a-b` is 4.
/// The names of data keys and assignment targets stay as they are.
fn subtract_kebab_variables(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| variables.contains(*part)) {
				let mut terms = parts.into_iter().map(|part| Node::Symbol(part.to_string()));
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(term)))
			} else {
				Node::Symbol(name)
			}
		}
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => {
			Node::Key(left, op, Box::new(subtract_kebab_variables(*right, variables)))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(subtract_kebab_variables(*left, variables)), op, Box::new(subtract_kebab_variables(*right, variables))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| subtract_kebab_variables(item, variables)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(subtract_kebab_variables(*node, variables)), data },
		other => other,
	}
}

fn resolve_blocks(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut keys: HashMap<String, Node> = HashMap::new();
			let mut resolved = Vec::with_capacity(items.len());
			let mut ends_in_lookup = false;
			for item in items {
				let item = resolve_blocks(item, variables);
				ends_in_lookup = matches!(item.drop_meta(), Node::Symbol(name) if keys.contains_key(name));
				let item = substitute_keys(item, &keys);
				if let Some((name, value)) = data_binding(&item) {
					if !variables.contains(name) {
						keys.insert(name.to_string(), value.clone());
					}
				}
				resolved.push(item);
			}
			match resolved.pop() {
				Some(value) if ends_in_lookup => value,
				Some(last) => {
					resolved.push(last);
					Node::List(resolved, bracket, separator)
				}
				None => Node::List(resolved, bracket, separator),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(resolve_blocks(*left, variables)), op, Box::new(resolve_blocks(*right, variables))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(resolve_blocks(*node, variables)), data },
		other => other,
	}
}

fn substitute_keys(node: Node, keys: &HashMap<String, Node>) -> Node {
	if keys.is_empty() {
		return node;
	}
	match node {
		Node::Symbol(name) => keys.get(&name).cloned().unwrap_or(Node::Symbol(name)),
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => Node::Key(left, op, Box::new(substitute_keys(*right, keys))),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute_keys(*left, keys)), op, Box::new(substitute_keys(*right, keys))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute_keys(item, keys)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute_keys(*node, keys)), data },
		other => other,
	}
}

/// An ungrouped binary arithmetic expression: `2 * 1.5`, not `(2 * 1.5)`
fn is_arithmetic(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Rem | Op::Pow))
}

fn conversion_of_arithmetic_warning(left: &Node, right: &Node) -> String {
	format!("`{} as {}` converts the whole `{}`, not only its last operand", left.serialize(), right.serialize(), left.serialize())
}

/// Both readings of `a * b as T` with their explicit forms: the whole expression, or the nearest operand
fn conversion_readings(left: &Node, right: &Node) -> Vec<(String, String)> {
	let (whole, target) = (left.serialize(), right.serialize());
	let mut readings = vec![(format!("convert the whole {whole}"), format!("({whole}) as {target}"))];
	if let Node::Key(first, op, last) = left {
		let tight = match last.drop_meta() {
			Node::Number(_) => format!("{}:{target}", last.serialize()),
			_ => format!("({} as {target})", last.serialize()),
		};
		readings.push((format!("convert only {}", last.serialize()), format!("{} {} {tight}", first.serialize(), op.as_str())));
	}
	readings
}

/// A negative literal or a negation: `-7`, `-x`, `(-7)`
fn is_negative(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n < 0,
		Node::Number(Number::Float(f)) => *f < 0.0,
		Node::Number(Number::Quotient(numerator, _)) => *numerator < 0,
		Node::Number(Number::BigQuotient(q)) => q.is_negative(),
		Node::Number(Number::BigInt(big)) => big.sign() == num_bigint::Sign::Minus,
		Node::Key(left, Op::Neg | Op::Sub, _) => matches!(left.drop_meta(), Node::Empty),
		Node::List(items, _, _) if items.len() == 1 => is_negative(&items[0]),
		_ => false,
	}
}

/// `%` is Euclidean (0 ≤ r < |b|); C, Java, JS and Rust truncate, Python floors
fn negative_modulo_warning(left: &Node, right: &Node) -> String {
	let (a, b) = (left.serialize(), right.serialize());
	let (a, b) = (a.trim(), b.trim());
	let values = match (left.drop_meta(), right.drop_meta()) {
		(Node::Number(Number::Int(x)), Node::Number(Number::Int(y))) => x.checked_rem_euclid(*y).zip(x.checked_rem(*y)),
		_ => None,
	};
	match values {
		Some((euclidean, truncated)) if euclidean != truncated => format!(
			"`{a} % {b}` is {euclidean}: % is Euclidean as in mathematics; C/Java/JS give {truncated}; use `rem` for the truncated remainder"
		),
		_ => format!("`{a} % {b}`: % is Euclidean as in mathematics (never negative), unlike C/Java/JS; use `rem` for the truncated remainder"),
	}
}

/// A braceless argument takes arithmetic (`f 3-1` is `f(3-1)`, also in `1 + f 3-1`), so a second braceless call inside it
/// is ambiguous (wiki/precedence.md): `square 3 + square 3` reads as `square(3 + square 3)` or `square(3) + square(3)`.
/// Bad.md's recursive `fib it-1 + fib it-2` would silently mean `fib(it-1 + fib(it-2))`, so it is rejected with both readings.
fn check_ambiguous_calls(node: &Node) -> Option<Diagnostic> {
	const KEYWORDS: [&str; 10] = ["return", "const", "let", "var", "def", "fun", "fn", "use", "import", "include"];
	fn braceless_call(node: &Node) -> Option<(&String, &Node)> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => match items[0].drop_meta() {
				Node::Symbol(head) if !KEYWORDS.contains(&head.as_str()) && !CONSTANT_KEYWORDS.contains(&head.as_str()) => Some((head, &items[1])),
				_ => None,
			},
			_ => None,
		}
	}
	fn holds_call(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => holds_call(left) || holds_call(right),
			other => braceless_call(other).is_some(),
		}
	}
	fn explicit(node: &Node) -> String {
		if let Some((head, argument)) = braceless_call(node) {
			return format!("{head}({})", explicit(argument));
		}
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => format!("{} {} {}", explicit(left), op.as_str(), explicit(right)),
			other => other.serialize(),
		}
	}
	if let Some((head, argument)) = braceless_call(node) {
		if let Node::Key(left, op, right) = argument.drop_meta() {
			if op.is_arithmetic() && holds_call(argument) {
				let whole = format!("{head}({})", explicit(argument));
				let first = format!("{head}({}) {} {}", explicit(left), op.as_str(), explicit(right));
				let written = format!("{head} {}", argument.drop_meta().serialize());
				return Some(Diagnostic::at(node, format!("ambiguous braceless call: {written}"))
					.fix(format!("{first} or {whole}"))
					.offer(format!("{head} of {} only", left.serialize()), &written, &first)
					.offer(format!("{head} of the whole {}", argument.drop_meta().serialize()), &written, &whole));
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => check_ambiguous_calls(left).or_else(|| check_ambiguous_calls(right)),
		Node::List(items, _, _) => items.iter().find_map(check_ambiguous_calls),
		_ => None,
	}
}

/// Booleans are not numbers: `true + true` is rejected, not 2 (the runtime still encodes them as Int 1/0)
fn check_boolean_arithmetic(node: &Node) -> Option<Diagnostic> {
	fn is_boolean(operand: &Node) -> bool {
		match operand.drop_meta() {
			Node::True | Node::False => true,
			Node::Key(left, op, _) => op.is_comparison() || (*op == Op::Not && matches!(left.drop_meta(), Node::Empty)),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => is_boolean(&items[0]),
			_ => false,
		}
	}
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() && (is_boolean(left) || is_boolean(right)) => {
			let expression = node.drop_meta().serialize();
			let converted = |operand: &Node| if is_boolean(operand) { format!("int({})", operand.serialize()) } else { operand.serialize() };
			let explicit = format!("{} {} {}", converted(left), op.as_str(), converted(right));
			Some(Diagnostic::at(node, format!("arithmetic on a boolean: {expression}")).fix(&explicit)
				.offer("count true as 1 and false as 0", expression.as_str(), explicit))
		}
		Node::Key(left, _, right) => check_boolean_arithmetic(left).or_else(|| check_boolean_arithmetic(right)),
		Node::List(items, _, _) => items.iter().find_map(check_boolean_arithmetic),
		_ => None,
	}
}

/// Why a variable cannot be used as a plain value before an `if x {…}` check
#[derive(Clone, Copy, PartialEq)]
enum Unchecked {
	Null,  // `x=ø` (wiki/null.md)
	Error, // `x=fetch …`: a remote call may fail (DESIGN.md "Effects": Error as Result<T, E>)
}

impl Unchecked {
	fn of(value: &Node) -> Option<Unchecked> {
		match value.drop_meta() {
			Node::Empty => Some(Unchecked::Null),
			_ if crate::host::fetch_call(value).is_some() => Some(Unchecked::Error),
			_ => None,
		}
	}

	fn describe(self, name: &str) -> String {
		match self {
			Unchecked::Null => format!("{name} may be ø (null)"),
			Unchecked::Error => format!("{name} may be an error (fetch can fail)"),
		}
	}
}

/// `x=ø` makes x possibly null, `x=fetch …` possibly an error: arithmetic or member access on it needs an
/// `if x {…}` check first (ø and errors are falsy). A fetch used directly as an operand is never checked.
fn check_null_use(node: &Node, nullable: &mut HashMap<String, Unchecked>) -> Option<Diagnostic> {
	let possibly_null = |operand: &Node, nullable: &HashMap<String, Unchecked>| match operand.drop_meta() {
		Node::Symbol(name) => nullable.get(name).map(|reason| (name.clone(), reason.describe(name), None)),
		fetched if crate::host::fetch_call(fetched).is_some() => {
			let call = fetched.serialize();
			Some(("result".to_string(), format!("{call} may be an error (fetch can fail)"), Some(call)))
		}
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			// `out = out + xs` building a list from ø, the empty list: a concatenation, no use of a null
			let builds_list = matches!(value.drop_meta(), Node::Key(left, Op::Add, _) if left.drop_meta() == target.drop_meta()
				&& nullable.get(&target.name()) == Some(&Unchecked::Null));
			let found = if builds_list {
				let Node::Key(_, _, right) = value.drop_meta() else { unreachable!("guarded") };
				check_null_use(right, nullable)
			} else {
				check_null_use(value, nullable)
			};
			let target = match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => &**name, // x:int?=ø
				other => other,
			};
			if let Node::Symbol(name) = target.drop_meta() {
				match Unchecked::of(value) {
					Some(reason) => nullable.insert(name.clone(), reason),
					None => nullable.remove(name),
				};
			}
			found
		}
		Node::Key(if_condition, Op::Then, then) => {
			let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else {
				return check_null_use(if_condition, nullable).or_else(|| check_null_use(then, nullable));
			};
			let mut narrowed = nullable.clone();
			if let Node::Symbol(name) = condition.drop_meta() {
				narrowed.remove(name);
			}
			check_null_use(condition, nullable).or_else(|| check_null_use(then, &mut narrowed))
		}
		// ø is the empty list: appending to it or concatenating a list needs no check (`a=(); a.add(1)`), nor popping a
		// stack built in a loop (`st=[]; for t in ts { if … { st.pop() } else { st.add(t) } }`)
		Node::Key(list, Op::Dot, call) if (updates_list(list, call) || popped_list(list, call).is_some()) && nullable.get(&list.name()) != Some(&Unchecked::Error) => {
			let found = check_null_use(call, nullable);
			nullable.remove(&list.name()); // no longer empty
			found
		}
		// ø is the empty list: it counts 0 (`b=[]; b.count`)
		Node::Key(list, Op::Dot, property) if matches!(list.drop_meta(), Node::Symbol(_)) && is_counting_property(&property.name()) && nullable.get(&list.name()) != Some(&Unchecked::Error) => None,
		Node::Key(left, Op::Add, right) if [left, right].iter().any(|side| matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))) => {
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, op, right) if op.is_arithmetic() || *op == Op::Dot => {
			let operand = possibly_null(left, nullable).or_else(|| if *op == Op::Dot { None } else { possibly_null(right, nullable) });
			if let Some((name, reason, call)) = operand {
				let expression = node.drop_meta().serialize();
				let fix = match call {
					Some(call) => format!("{name} = {call}; if {name} {{ {} }}", expression.replace(&call, &name)),
					None => format!("if {name} {{ {expression} }}"),
				};
				return Some(Diagnostic::at(node, format!("{reason} in {expression}")).fix(fix));
			}
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, _, right) => check_null_use(left, nullable).or_else(|| check_null_use(right, nullable)),
		Node::List(items, _, _) => items.iter().find_map(|item| check_null_use(item, nullable)),
		_ => None,
	}
}

/// `const x=…` binds x once; any later assignment, compound assignment, increment or element assignment is rejected
fn check_constants(node: &Node, constants: &mut HashMap<String, String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::List(items, _, _) => {
			let statements = match items.as_slice() {
				[keyword, declaration, rest @ ..] if is_constant_keyword(keyword) || is_word(keyword, IMMUTABLE_LET) => {
					let keyword = keyword.name();
					let Node::Key(target, Op::Assign | Op::Define, value) = declaration.drop_meta() else {
						return Some(Diagnostic::at(declaration, format!("{keyword} needs a value: {}", declaration.serialize())).fix(format!("{keyword} x = 5")));
					};
					if let Some(found) = check_constants(value, constants) {
						return Some(found);
					}
					constants.insert(target.name(), keyword);
					rest
				}
				_ => items.as_slice(),
			};
			statements.iter().find_map(|statement| check_constants(statement, constants))
		}
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() => {
			let place = match target.drop_meta() {
				Node::Key(name, Op::Hash, _) => name.name(),
				other => other.name(),
			};
			if let Some(keyword) = constants.get(&place) {
				let assignment = node.drop_meta().serialize();
				return Some(if keyword == IMMUTABLE_LET {
					Diagnostic::at(node, format!("{place} is let (immutable), cannot assign it again: {assignment}"))
						.fix(format!("declare it with var or plain `{place} =` if it changes"))
				} else {
					Diagnostic::at(node, format!("{place} is const, cannot assign it again: {assignment}"))
						.fix(format!("use a new name instead of {place}, or declare it without const"))
				});
			}
			check_constants(value, constants)
		}
		Node::Key(left, _, right) => check_constants(left, constants).or_else(|| check_constants(right, constants)),
		_ => None,
	}
}

/// A parameter annotation must name a builtin or a user-defined type, `x:flaot` is a typo
fn check_parameter_annotations(program: &Node) -> Option<Diagnostic> {
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	let mut user_types = crate::type_kinds::TypeRegistry::new();
	collect_all_types(&mut user_types, program);
	let traits = crate::traits::Traits::of(program);
	context.user_functions.values().flat_map(|function| &function.params).find_map(|param| {
		let annotation = param.annotation.as_ref()?;
		let type_name = annotation.name();
		let is_known_name = |name: &str| type_word_kind(name).is_some() || user_types.get_by_name(name).is_some() || traits.is_trait(name);
		let known = match type_name.strip_prefix(LIST_OF_PREFIX) {
			Some(element) => is_known_name(element) || names_list_type(element),
			None => annotated_kind(annotation).is_some() || user_types.get_by_name(type_name.trim_end_matches('?')).is_some(),
		};
		(!known).then(|| Diagnostic::at(annotation, format!("unknown type {type_name} of parameter {}", param.name)))
	})
}

/// The declared name and type of the target `x:int` or `int x`
fn declaring_name_and_type(target: &Node) -> Option<(String, String)> {
	let (name, type_name) = match target {
		Node::Key(name, Op::Colon, type_name) => (name.drop_meta().clone(), type_name.drop_meta().clone()),
		prefixed => number_type_prefix(prefixed)?,
	};
	match (name, type_name) {
		(Node::Symbol(name), Node::Symbol(type_name)) => Some((name, type_name)),
		_ => None,
	}
}

/// `x:int=…` declares x's type; every value assigned to x later must fit it
fn check_declared_types(node: &Node, declared: &mut HashMap<String, String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			let declaration = match target.drop_meta() {
				Node::Symbol(name) => declared.get_key_value(name).map(|(name, type_name)| (name.clone(), type_name.clone())),
				declaring => declaring_name_and_type(declaring),
			};
			if let Some((name, type_name)) = declaration {
				declared.insert(name.clone(), type_name.clone());
				if let Some(mismatch) = assignment_mismatch(node, &name, &type_name, value) {
					return Some(mismatch);
				}
			}
			check_declared_types(value, declared)
		}
		Node::Key(left, _, right) => check_declared_types(left, declared).or_else(|| check_declared_types(right, declared)),
		Node::List(items, _, _) => {
			let prefixed = items.windows(2).filter_map(|pair| prefixed_declaration(&pair[0], &pair[1]));
			prefixed.chain(items.iter().cloned()).find_map(|item| check_declared_types(&item, declared))
		}
		_ => None,
	}
}

fn assignment_mismatch(assignment: &Node, name: &str, type_name: &str, value: &Node) -> Option<Diagnostic> {
	if let Some(base) = type_name.strip_suffix('?') {
		return match value.drop_meta() {
			Node::Empty => None,
			_ => assignment_mismatch(assignment, name, base, value),
		};
	}
	if matches!(value.drop_meta(), Node::Empty) && builtin_type_kind(type_name).is_some() {
		let message = format!("type mismatch: {name} is declared {type_name}, cannot assign ø");
		return Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:{type_name}? to allow ø")));
	}
	let expected = builtin_type_kind(type_name)?;
	let actual = literal_kind(value)?;
	let exact_decimal = canonical_type_name(type_name) == "exact" && actual == Kind::Float;
	let one_character_text = expected == Kind::Text && actual == Kind::Codepoint; // `"a"` parses as a codepoint
	if expected == actual || (expected == Kind::Float && actual == Kind::Int) || exact_decimal || one_character_text {
		return None;
	}
	let value_text = value.serialize();
	let message = format!("type mismatch: {name} is declared {type_name}, cannot assign {} {value_text}", format!("{actual:?}").to_lowercase());
	Some(Diagnostic::at(assignment, message).fix(format!("{name}={type_name}({value_text}) or declare {name}:{}", format!("{actual:?}").to_lowercase())))
}

/// `x:T = v` → `x = v` with T kept as metadata on x (see `declared_type`);
/// the widening of an Int literal assigned to a float becomes an explicit Float literal, a codepoint assigned to a text a Text
pub fn lower_declarations(node: Node) -> Node {
	let variables = assigned_names(&node).into_iter().map(str::to_string).collect();
	let mut names = Names { values: names_used_as_values(&node), variables };
	names.values.extend(names.variables.iter().cloned());
	lower_declarations_among(node, &names)
}

/// A function that indexes or counts a list parameter (`def swap(arr, i, j) { … arr[i] … }`) works on a local copy of it,
/// `arr·list = arr`, which the emitter holds as an array (list_dispatch.rs `$NodeList`): one conversion per call
/// instead of a walk per index
pub fn indexed_parameter_copies(node: Node) -> Node {
	let maps = map_parameters(&node);
	parameter_copies(node, &maps)
}

fn parameter_copies(node: Node, maps: &HashSet<(String, usize)>) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			let function = items[0].name();
			let mut body = parameter_copies(*body, maps);
			// a parameter that is assigned (`arr = swap(arr, i, j)`) would convert at every assignment: it keeps walking
			for (index, parameter) in items[1..].iter().enumerate().filter_map(|(index, parameter)| Some((index, parameter_symbol(parameter)?))) {
				if indexes(&body, &parameter) && !assigned_from_call_in_loop(&body, &parameter) {
					body = with_copy(body, &parameter, LIST_COPY_SUFFIX);
				} else if maps.contains(&(function.clone(), index)) && keys(&body, &parameter) && !assigns(&body, &parameter) {
					body = with_copy(body, &parameter, crate::wasm_emitter::MAP_COPY_SUFFIX); // a hash table (map_backend.rs)
				}
			}
			Node::Key(head, op, Box::new(body))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(parameter_copies(*left, maps)), op, Box::new(parameter_copies(*right, maps))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| parameter_copies(item, maps)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(parameter_copies(*node, maps)), data },
		other => other,
	}
}

/// The parameters every call passes a map: a map literal of name keys, or a variable only ever assigned one. Only those
/// may become hash tables; a list or an instance given there keeps the generic way
fn map_parameters(program: &Node) -> HashSet<(String, usize)> {
	let is_map_literal = |value: &Node| matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.iter().all(|item|
		matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if matches!(key.drop_meta(), Node::Symbol(key) | Node::Text(key) if !key.starts_with(crate::node::ATTRIBUTE_MARK)))));
	let mut assigned: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, value) = part {
		if let Node::Symbol(name) = target.drop_meta() {
			*assigned.entry(name.clone()).or_insert(true) &= is_map_literal(value);
		}
	});
	let is_map = |argument: &Node| is_map_literal(argument) || matches!(argument.drop_meta(), Node::Symbol(name) if assigned.get(name) == Some(&true));
	let mut passed: HashMap<(String, usize), bool> = HashMap::new();
	calls_outside_heads(program, &mut |items| for (function, arguments) in called_functions(items) {
		for (index, argument) in arguments.into_iter().enumerate() {
			*passed.entry((function.clone(), index)).or_insert(true) &= is_map(argument);
		}
	});
	passed.into_iter().filter(|(_, maps)| *maps).map(|(position, _)| position).collect()
}

/// Every list of the program but a definition's head `(f a b) := …`, which names parameters and calls nothing
fn calls_outside_heads<'a>(node: &'a Node, action: &mut dyn FnMut(&'a [Node])) {
	match node.drop_meta() {
		Node::Key(head, Op::Define | Op::Assign, body) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) => calls_outside_heads(body, action),
		Node::Key(left, _, right) => {
			calls_outside_heads(left, action);
			calls_outside_heads(right, action);
		}
		Node::List(items, _, _) => {
			action(items);
			items.iter().for_each(|item| calls_outside_heads(item, action));
		}
		_ => {}
	}
}

const LIST_COPY_SUFFIX: &str = "·list";

fn parameter_symbol(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Colon, _) => match name.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// Does the body index (`xs#i`) or count (`#xs`) the variable in a loop, not by a text key: once is cheaper as a walk
/// than a conversion
fn indexes(body: &Node, name: &str) -> bool {
	let mut found = false;
	let mut loops = vec![];
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part { loops.push(part) });
	loops.into_iter().for_each(|body| body.visit(&mut |part| {
		let Node::Key(list, Op::Hash, index) = part else { return };
		let counted = if matches!(list.drop_meta(), Node::Empty) { index } else { list };
		found |= !is_text_key(index) && matches!(counted.drop_meta(), Node::Symbol(symbol) if symbol == name);
	}));
	found
}

/// Is the variable assigned a call's result inside a loop (`arr = swap(arr, i, j)`, a conversion per iteration)
fn assigned_from_call_in_loop(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part {
		part.visit(&mut |inner| if let Node::Key(target, Op::Assign, value) = inner {
			let is_call = matches!(value.drop_meta(), Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
			found |= is_call && matches!(target.drop_meta(), Node::Symbol(target) if target == name);
		});
	});
	found
}

/// Does the body set an entry of the map variable (`m[k] = v`) or look one up in a loop, by a text key
fn keys(body: &Node, name: &str) -> bool {
	let is_name = |part: &Node| matches!(part.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let by_key = is_text_key;
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		found |= matches!(target.drop_meta(), Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index));
		found |= is_name(target) && field_update(value, name);
	});
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part {
		part.visit(&mut |inner| found |= matches!(inner, Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index)));
	});
	found
}

/// `field_with(m, "k", v)` of the variable m with a text key: the lowered `m["k"] = v`
fn field_update(value: &Node, name: &str) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, map, key, _]
		if word.name() == crate::library_words::FIELD_WITH && matches!(map.drop_meta(), Node::Symbol(map) if map == name) && is_text(key)))
}

/// An index by a text key: `m["k"]`, `m["k\(i)"]` (arriving as `"k" + text_form(i)`), not a position
fn is_text_key(index: &Node) -> bool {
	is_text(crate::wasp_parser::subscript_key(index).unwrap_or(index))
}

fn is_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Text(_) | Node::Char(_) => true,
		Node::Key(left, Op::Add, right) => is_text(left) || is_text(right),
		_ => false,
	}
}

/// Is the variable itself assigned in the body (`m = …`, `m += …`), other than by setting an entry
fn assigns(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(target, op, value) = part {
		let is_variable = matches!(target.drop_meta(), Node::Symbol(symbol) if symbol == name);
		found |= is_variable && (op.is_compound_assign() || (*op == Op::Assign && !field_update(value, name)));
	});
	found
}

/// `name·list = name` (or another copy suffix) first, then the body with every `name` read as the copy
fn with_copy(body: Node, name: &str, suffix: &str) -> Node {
	let copy = format!("{name}{suffix}");
	let renamed = rename_symbol(body, name, &copy);
	let start = Node::Key(Box::new(Node::Symbol(copy)), Op::Assign, Box::new(Node::Symbol(name.to_string())));
	match renamed {
		Node::List(items, Bracket::Curly, separator) => Node::List(std::iter::once(start).chain(items).collect(), Bracket::Curly, match separator {
			Separator::Semicolon | Separator::Newline => separator,
			_ => Separator::Semicolon,
		}),
		other => Node::List(vec![start, other], Bracket::Curly, Separator::Semicolon),
	}
}

fn rename_symbol(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		Node::Key(left, op, right) => Node::Key(Box::new(rename_symbol(*left, from, to)), op, Box::new(rename_symbol(*right, from, to))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| rename_symbol(item, from, to)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rename_symbol(*node, from, to)), data },
		other => other,
	}
}

/// The names of a program: `variables` it assigns (`return count` returns the variable, it is no `return.count`), and
/// every word it uses as a value or binds (a parameter, a loop variable): `chars[i]` then indexes, `int[3]` makes zeros
struct Names {
	variables: HashSet<String>,
	values: HashSet<String>,
}

/// Words that stand as values somewhere: not as the head of a call, a type annotation (`x:int`, `as int`) or the
/// element type of `int[3]`
fn names_used_as_values(program: &Node) -> HashSet<String> {
	fn visit(node: &Node, names: &mut HashSet<String>) {
		match node.drop_meta() {
			Node::Symbol(name) => {
				names.insert(name.clone());
			}
			Node::Key(left, Op::Colon | Op::As, _) => visit(left, names),
			Node::Key(element, Op::Hash, index) if matches!(element.drop_meta(), Node::Symbol(_)) => visit(index, names),
			Node::Key(left, _, right) => {
				visit(left, names);
				visit(right, names);
			}
			Node::List(items, _, _) => {
				let called = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) && items.len() > 1;
				items.iter().skip(called as usize).for_each(|item| visit(item, names))
			}
			_ => {}
		}
	}
	let mut names = HashSet::new();
	visit(program, &mut names);
	names
}

fn lower_declarations_among(node: Node, names: &Names) -> Node {
	let variables = &names.variables;
	let lower = |node| lower_declarations_among(node, names);
	let node = match crate::for_loop::lower(node) {
		Ok(loop_as_while) => return lower(loop_as_while),
		Err(node) => node,
	};
	match node {
		Node::List(items, _, _) if print_walk(&items, variables).is_some() => lower(print_walk(&items, variables).expect("guarded")),
		Node::List(items, bracket, separator) if counting_phrase(&items, &bracket, &separator, variables).is_some() => {
			lower(counting_phrase(&items, &bracket, &separator, variables).expect("guarded"))
		}
		Node::List(items, bracket, separator) if of_type_declaration(&items, &bracket, &separator).is_some() => {
			lower(of_type_declaration(&items, &bracket, &separator).expect("guarded"))
		}
		Node::List(items, _, _) if hashed_unit_count(&items).is_some() => {
			lower(hashed_unit_count(&items).expect("guarded"))
		}
		Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) && unit_count(&counted).is_some() => {
			lower(unit_count(&counted).expect("guarded"))
		}
		// `x : 100 int` and `x:int[100]` declare x as a zero-filled list of 100 ints
		declaration if typed_array_declaration(&declaration).is_some() => {
			let (name, zeros) = typed_array_declaration(&declaration).expect("guarded");
			Node::Key(Box::new(name), Op::Assign, Box::new(zeros))
		}
		// `letters = char[3]` and `upcases = 26 * char` are zero-filled typed arrays like `x : 100 int`
		Node::Key(target, Op::Assign, value) if typed_array_value(&value).is_some() => {
			Node::Key(target, Op::Assign, Box::new(typed_array_value(&value).expect("guarded")))
		}
		// `x as number = 9` declares `x:number=9`
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Key(name, Op::As, type_node)
			if matches!(name.drop_meta(), Node::Symbol(_)) && is_declaration_type(type_node)) => {
			let Node::Key(name, Op::As, type_node) = target.drop_meta().clone() else { unreachable!("guarded") };
			lower(Node::Key(Box::new(Node::Key(name, Op::Colon, type_node)), Op::Assign, value))
		}
		// `x:[number]=v` is `x:list of number=v`
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Key(_, Op::Colon, type_node) if bracketed_list_type(type_node).is_some()) => {
			let Node::Key(name, Op::Colon, type_node) = target.drop_meta().clone() else { unreachable!("guarded") };
			let typed = Node::Key(name, Op::Colon, Box::new(bracketed_list_type(&type_node).expect("guarded")));
			lower(Node::Key(Box::new(typed), Op::Assign, value))
		}
		// `x:[number]` is `x:list of number`
		Node::Key(name, Op::Colon, type_node) if bracketed_list_type(&type_node).is_some() => {
			Node::Key(name, Op::Colon, Box::new(bracketed_list_type(&type_node).expect("guarded")))
		}
		// `r=1…3` stores the list [1 2 3]; the range itself only lives in a `for` header
		Node::Key(target, Op::Assign, value) if range_elements(&value).is_some() => {
			Node::Key(target, Op::Assign, Box::new(range_elements(&value).expect("guarded")))
		}
		// `xs = a..b` of computed bounds: the list a loop over the range collects
		Node::Key(target, Op::Assign, value) if computed_range(&target, &value).is_some() => {
			lower(Node::Key(target.clone(), Op::Assign, Box::new(computed_range(&target, &value).expect("guarded"))))
		}
		// `fast x=v` → `x:fast=v`, parsed either as `(fast x)=v` or as the statement pair `fast (x=v)`;
		// `double(x) := x+x` and `double x := x+x` stay function definitions
		Node::Key(target, Op::Assign, value) if number_type_prefix(&target).is_some() => {
			let (type_name, name) = number_type_prefix(&target).expect("guarded");
			lower(Node::Key(Box::new(Node::Key(Box::new(name), Op::Colon, Box::new(type_name))), Op::Assign, value))
		}
		Node::List(items, bracket, separator) if items.windows(2).any(|pair| paired_declaration(&pair[0], &pair[1]).is_some()) => {
			let mut lowered = Vec::with_capacity(items.len());
			let mut items = items.into_iter().peekable();
			while let Some(item) = items.next() {
				match items.peek().and_then(|next| paired_declaration(&item, next)) {
					Some(declaration) => {
						items.next();
						lowered.push(lower(declaration));
					}
					None => lowered.push(lower(item)),
				}
			}
			if lowered.len() == 1 {
				lowered.remove(0)
			} else {
				Node::List(lowered, bracket, separator)
			}
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			let value = Box::new(lower(*value));
			match target.drop_meta() {
				Node::Key(name, Op::Colon, type_name) if matches!((name.drop_meta(), type_name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
					let value = match (builtin_type_kind(&type_name.name()), value.drop_meta()) {
						(Some(Kind::Float), Node::Number(number @ (Number::Int(_) | Number::BigInt(_)))) => Box::new(Node::Number(Number::Float((*number).into()))),
						(Some(Kind::Text), Node::Char(character)) => Box::new(Node::Text(character.to_string())),
						_ => value,
					};
					Node::Key(Box::new(Node::meta(name.drop_meta().clone(), type_name.drop_meta().clone())), op, value)
				}
				_ => Node::Key(target, op, value),
			}
		}
		Node::Key(list, Op::Dot, call) if inserted_element(&list, &call).is_some() => lowered_insert(list, &call),
		Node::Key(list, Op::Dot, call) if appended_element(&list, &call).is_some() => {
			let element = lower(appended_element(&list, &call).expect("guarded").clone());
			let appended = Node::Key(list.clone(), Op::Add, Box::new(Node::List(vec![element], Bracket::Square, Separator::Space)));
			Node::Key(list, Op::Assign, Box::new(appended))
		}
		Node::Key(list, Op::Dot, call) if popped_list(&list, &call).is_some() => popped_list(&list, &call).expect("guarded"),
		Node::Key(map, Op::Dot, call) if removed_key(&map, &call).is_some() => lower(removed_key(&map, &call).expect("guarded")),
		// `int[n]`, `#(int[n])`: n zeros of the type, wherever it stands, unless the word is a variable (`chars[i]`)
		Node::Key(element, Op::Hash, one_based) if zero_filled_subscript(&element, &one_based, &names.values).is_some() => {
			zero_filled_subscript(&element, &one_based, &names.values).expect("guarded")
		}
		// x² and x³ are x^2 and x^3 for emission; the parse keeps the suffix operators
		Node::Key(base, op, _) if op.suffix_exponent().is_some() => {
			let exponent = op.suffix_exponent().expect("guarded");
			Node::Key(Box::new(lower(*base)), Op::Pow, Box::new(Node::int(exponent)))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		// `const x=v` → `x=v`; check_constants already enforced the single assignment; `let x=v` and `var x=v` → `x=v`
		Node::List(items, bracket, separator) if items.len() >= 2 && is_declaration_keyword(&items[0]) => {
			let mut declaration = items.into_iter().skip(1).map(lower).collect::<Vec<_>>();
			if declaration.len() == 1 {
				declaration.remove(0)
			} else {
				Node::List(declaration, bracket, separator)
			}
		}
		Node::List(items, Bracket::None, _) if applied_object(&items).is_some() => {
			let (object, key) = applied_object(&items).expect("guarded");
			lower(crate::wasp_parser::subscript(object, key))
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// `x:list of int=[1 2]` (parsed as the items `x:list`, `of`, `int=[1 2]`) is `x:"list of int"=[1 2]`, the same type as `x:list<int>`;
/// nested applications chain: `list of list of int`
fn of_type_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let [declaration, of, ..] = items else { return None };
	let Node::Key(name, Op::Colon, head) = declaration.drop_meta() else { return None };
	let Node::Symbol(mut type_name) = head.drop_meta().clone() else { return None };
	if !is_word(of, "of") {
		return None;
	}
	let mut next = 2;
	let mut assignment = None;
	while let Some(item) = items.get(next) {
		next += 1;
		match item.drop_meta() {
			Node::Symbol(word) => type_name = format!("{type_name} of {word}"),
			Node::Key(word, op, value) => {
				let Node::Symbol(word) = word.drop_meta() else { return None };
				type_name = format!("{type_name} of {word}");
				assignment = Some((*op, value.clone()));
				break;
			}
			_ => return None,
		}
		match items.get(next) {
			Some(of) if is_word(of, "of") => next += 1,
			_ => break,
		}
	}
	let typed_name = Node::Key(name.clone(), Op::Colon, Box::new(Node::Symbol(type_name)));
	let declared = match assignment {
		Some((op, value)) => Node::Key(Box::new(typed_name), op, value),
		None => typed_name,
	};
	match &items[next..] {
		[] => Some(declared),
		rest => Some(Node::List([vec![declared], rest.to_vec()].concat(), bracket.clone(), separator.clone())),
	}
}

/// `{a:1 b:2}(key)`: applying an object to a key looks it up, like `{a:1 b:2}[key]`
fn applied_object(items: &[Node]) -> Option<(Node, Node)> {
	let [object, argument] = items else { return None };
	let is_object = matches!(object.drop_meta(), Node::List(entries, Bracket::Curly, _)
		if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))));
	match argument.drop_meta() {
		Node::List(key, Bracket::Round, _) if is_object && key.len() == 1 => Some((object.drop_meta().clone(), key[0].drop_meta().clone())),
		_ => None,
	}
}

/// Methods that append one element; with value semantics `x.add(v)` rebinds `x = x + [v]`
const APPEND_METHODS: [&str; 4] = ["add", "append", "push", "insert"];
const POP_METHOD: &str = "pop";
const REMOVE_METHOD: &str = "remove";
const POP_TEMPORARY: &str = "pop_tmp";
const INSERT_METHOD: &str = "insert";

/// Pseudo-call `list_insert_at(list, position, value)`: the list with value inserted at the 0-based position
pub const INSERT_AT_CALL: &str = "list_insert_at";
/// Pseudo-call `insert_in_either_order(list, a, b)`: `xs.insert(a, b)` before the kinds decide which is the position
pub const INSERT_EITHER_CALL: &str = "insert_in_either_order";
const AT_WORD: &str = "at";

/// The name of a constant field key (`p.x`, `p["x"]`) when it is spelled with letters, digits and `_`:
/// the runtime error for its miss is a function named after it (`no_field_x`), and the trace of the trap carries the name.
/// A symbol key `p[k]` is evaluated when `k` is a variable, so it is not a constant.
pub fn constant_field_name(key: &Node) -> Option<String> {
	let name = match key.drop_meta() {
		Node::Text(name) => name.clone(),
		Node::Char(letter) => letter.to_string(),
		_ => return None,
	};
	(!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')).then_some(name)
}

/// Methods that change the list variable they are called on: `xs.add(v)`, `xs.insert(v, at:1)`, `xs.pop()`
pub fn is_list_mutating_method(name: &str) -> bool {
	APPEND_METHODS.contains(&name) || name == POP_METHOD || name == REMOVE_METHOD
}

/// The element of `x.add(v)` when x is a variable
fn appended_element<'a>(list: &Node, call: &'a Node) -> Option<&'a Node> {
	let Node::Symbol(_) = list.drop_meta() else { return None };
	match call.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(method) if APPEND_METHODS.contains(&method.as_str())) => Some(&items[1]),
		_ => None,
	}
}

/// `xs.pop()` when xs is a variable: the last item, removed from xs (Python's list.pop())
fn popped_list(list: &Node, call: &Node) -> Option<Node> {
	let Node::Symbol(name) = list.drop_meta() else { return None };
	match call.drop_meta() {
		Node::List(items, _, _) if matches!(items.as_slice(), [method] if is_word(method, POP_METHOD)) => Some(crate::wasp_parser::parse(&format!(
			"({POP_TEMPORARY} = {name}#count({name}); {name} = slice({name}, 0, count({name})-1); {POP_TEMPORARY})"))),
		_ => None,
	}
}

/// `m.remove(k)` when m is a variable: the value of k, its entry removed from m (P35 default, Python's dict.pop)
fn removed_key(map: &Node, call: &Node) -> Option<Node> {
	let Node::Symbol(name) = map.drop_meta() else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [method, key] = items.as_slice() else { return None };
	if !is_word(method, REMOVE_METHOD) {
		return None;
	}
	let call = |word: &str, arguments: Vec<Node>| Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None);
	let removed = Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}removed"));
	let value = call(crate::library_words::MAP_GET_OR, vec![map.clone(), key.clone(), Node::Empty]);
	let without = call(crate::library_words::MAP_WITHOUT, vec![map.clone(), key.clone()]);
	Some(Node::List(vec![
		Node::Key(Box::new(removed.clone()), Op::Assign, Box::new(value)),
		Node::Key(Box::new(map.clone()), Op::Assign, Box::new(without)),
		removed,
	], Bracket::Round, Separator::Semicolon))
}

/// `xs.insert(a, b)` when xs is a variable. Wasp writes `insert(value, position)`, Python `insert(position, value)`:
/// the order is never guessed (wiki/Footguns.md "Guessing intent"), `at:` or the kinds decide
fn inserted_element(list: &Node, call: &Node) -> Option<Inserted> {
	let Node::Symbol(_) = list.drop_meta() else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [method, first, second] = items.as_slice() else { return None };
	if !is_word(method, INSERT_METHOD) {
		return None;
	}
	let position_of = |node: &Node| match node.drop_meta() {
		Node::Key(word, Op::Colon, position) if is_word(word, AT_WORD) => Some(position.as_ref().clone()),
		_ => None,
	};
	Some(match (position_of(first), position_of(second)) {
		(None, Some(position)) => Inserted::At(position, first.clone()),
		(Some(position), None) => Inserted::At(position, second.clone()),
		_ => Inserted::EitherOrder(first.clone(), second.clone()),
	})
}

/// The arguments of `xs.insert(…)`: `insert(x, at: i)` names the position, `insert(a, b)` leaves the order to the kinds
enum Inserted {
	At(Node, Node),
	EitherOrder(Node, Node),
}

/// `xs = list_insert_at(xs, position, value)`, or `insert_in_either_order(xs, a, b)` that the emitter resolves by the
/// kinds of a and b: the one Int is the position, two Ints are ambiguous (Python and wasp order differ)
fn lowered_insert(list: Box<Node>, call: &Node) -> Node {
	let (pseudo_call, first, second) = match inserted_element(&list, call).expect("guarded") {
		Inserted::At(position, value) => (INSERT_AT_CALL, position, value),
		Inserted::EitherOrder(first, second) => (INSERT_EITHER_CALL, first, second),
	};
	let arguments = vec![Node::Symbol(pseudo_call.to_string()), *list.clone(), lower_declarations(first), lower_declarations(second)];
	Node::Key(list, Op::Assign, Box::new(Node::List(arguments, Bracket::Round, Separator::None)))
}

/// `xs.add(v)`, `xs.insert(i, v)`: an update of the list variable that ø (the empty list) allows
fn updates_list(list: &Node, call: &Node) -> bool {
	appended_element(list, call).is_some() || inserted_element(list, call).is_some()
}

/// The zero value of an element type word (`int`, `ints`, `float`, `text` …)
fn zero_element(type_word: &str) -> Option<Node> {
	let element = plural_element_type(type_word).unwrap_or(type_word);
	Some(match type_word_kind(element)? {
		Kind::Int => Node::int(0),
		// `0.0f`: a bare 0.0 is an exact decimal, so an Int
		Kind::Float => Node::Key(Box::new(Node::float(0.0)), Op::As, Box::new(Node::Symbol(FLOAT_WORD.to_string()))),
		Kind::Text => Node::Text(String::new()),
		Kind::Codepoint => Node::Char('\0'),
		_ => return None,
	})
}

/// Pseudo-call `zero_fill(count, zero)`: the emitter builds the zero-filled list of `count` elements with a runtime loop,
/// so neither the program nor the compiler grows with the size of the array
pub const ZERO_FILL_CALL: &str = "zero_fill";

/// The zero-filled list of `count` (any number expression) elements of the type word
pub fn zero_list(count: Node, type_word: &str) -> Option<Node> {
	let zero = zero_element(type_word)?;
	Some(Node::List(vec![Node::Symbol(ZERO_FILL_CALL.to_string()), count, zero], Bracket::Round, Separator::None))
}

/// `[x]*n` and `n*[x]` with a list literal: Python repeats the list, NumPy multiplies each element (wiki/Footguns.md
/// "Lists and arithmetic"), so the user is asked; unanswered it is an error naming both explicit forms.
/// Likewise `[1 2 3]+4`: append or add to each element?
pub fn lower_list_times(node: Node) -> Node {
	let list_arithmetic = |key: &Node, positioned: &Node| list_times(key, positioned).or_else(|| list_plus(key, positioned));
	match node {
		Node::Meta { node: inner, data } => {
			let positioned = Node::Meta { node: inner.clone(), data: data.clone() };
			list_arithmetic(&inner, &positioned).unwrap_or_else(|| Node::Meta { node: Box::new(lower_list_times(*inner)), data })
		}
		Node::Key(left, op, right) => {
			let key = Node::Key(Box::new(lower_list_times(*left)), op, Box::new(lower_list_times(*right)));
			list_arithmetic(&key, &key).unwrap_or(key)
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_list_times).collect(), bracket, separator),
		other => other,
	}
}

const LIST_TIMES_TOPIC: &str = "list-times";

fn list_times(key: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Mul, right) = key.drop_meta() else { return None };
	let (list, count) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) => (left.as_ref(), right.as_ref()),
		(false, true) => (right.as_ref(), left.as_ref()),
		_ => return None,
	};
	let written = key.drop_meta().serialize();
	let (list_text, count_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(count));
	let readings = vec![
		crate::diagnostic::reading("repeat the list", &format!("{count_text} times {list_text}")),
		crate::diagnostic::reading("multiply each element", &format!("{list_text}.map(x => x*{count_text})")),
	];
	let question = crate::diagnostic::Ask::new(LIST_TIMES_TOPIC, format!("type error: list * number: does `{written}` repeat the list or multiply each element?"),
		readings, crate::diagnostic::Fallback::Error).written(&written).at_node(positioned);
	Some(match crate::diagnostic::ask(&question) {
		Ok(0) => filled_list(count.clone(), list).unwrap_or_else(|| crate::node::error("`n times [x]` repeats one element: `3 times [0]`")),
		Ok(_) => {
			let mapped = crate::wasp_parser::parse(&format!("({}).map(item => item * ({}))", list.serialize(), count.serialize()));
			lower_list_times(mapped)
		}
		Err(error) => error,
	})
}

const LIST_PLUS_TOPIC: &str = "list-plus";

fn is_list_literal(side: &Node) -> bool {
	matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))
}

/// `[1 2 3]+4` and `4+[1 2 3]` with a list literal and a number: append (prepend) or add to each element?
/// Unanswered it is an error naming `[1 2 3] + [4]` and `[1 2 3] .+ 4`
fn list_plus(key: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Add, right) = key.drop_meta() else { return None };
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let (list, number, list_first) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) if is_number(right) => (left.as_ref(), right.as_ref(), true),
		(false, true) if is_number(left) => (right.as_ref(), left.as_ref(), false),
		_ => return None,
	};
	let written = key.drop_meta().serialize();
	let (list_text, number_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(number));
	let (joining, joined) = match list_first {
		true => ("append", format!("{list_text} + [{number_text}]")),
		false => ("prepend", format!("[{number_text}] + {list_text}")),
	};
	let readings = vec![
		crate::diagnostic::reading(joining, &joined),
		crate::diagnostic::reading("add to each element", &format!("{list_text} .+ {number_text}")),
	];
	let question = crate::diagnostic::Ask::new(LIST_PLUS_TOPIC, format!("type error: list + number: does `{written}` {joining} {number_text} or add it to each element?"),
		readings, crate::diagnostic::Fallback::Error).written(&written).at_node(positioned);
	let singleton = Node::List(vec![number.clone()], Bracket::Square, Separator::Space);
	Some(match crate::diagnostic::ask(&question) {
		Ok(0) if list_first => Node::Key(Box::new(list.clone()), Op::Add, Box::new(singleton)),
		Ok(0) => Node::Key(Box::new(singleton), Op::Add, Box::new(list.clone())),
		Ok(_) => element_wise(list.clone(), Op::Add, number.clone()),
		Err(error) => error,
	})
}

/// The element name of the lambda an element-wise operator maps with: no wasp program writes it
const EACH_ELEMENT: &str = "each_element";

/// `xs .+ n` (also `.-`, `.*`, `./`): the operator applied to each element, `xs.map(each_element => each_element + n)`
pub fn element_wise(list: Node, op: Op, operand: Node) -> Node {
	let element = || Box::new(Node::Symbol(EACH_ELEMENT.to_string()));
	let lambda = Node::Key(element(), Op::FatArrow, Box::new(Node::Key(element(), op, Box::new(operand))));
	let call = Node::List(vec![Node::Symbol("map".to_string()), lambda], Bracket::Round, Separator::None);
	Node::Key(Box::new(list), Op::Dot, Box::new(call))
}

/// `n times [x]`: the list of n copies of x (`zero_fill(n, x)`); `[x]*n` stays ambiguous (Python repeats, NumPy multiplies)
pub fn filled_list(count: Node, list: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Square, _) = list.drop_meta() else { return None };
	let [element] = items.as_slice() else { return None };
	Some(Node::List(vec![Node::Symbol(ZERO_FILL_CALL.to_string()), count, element.clone()], Bracket::Round, Separator::None))
}

/// The zero-filled list of the array type written `int[100]` (a 1-based subscript, see `subscript`)
fn subscripted_array_type(type_node: &Node) -> Option<Node> {
	match type_node.drop_meta() {
		Node::Key(element, Op::Hash, one_based) => match (element.drop_meta(), one_based.drop_meta()) {
			(Node::Symbol(word), Node::Number(Number::Int(one_based))) => zero_list(Node::int(one_based - 1), word),
			_ => None,
		},
		_ => None,
	}
}

/// `int[n]` parsed as the subscript `int#(n+1)`: the list of n zeros when the type word is no variable
fn zero_filled_subscript(element: &Node, one_based: &Node, variables: &HashSet<String>) -> Option<Node> {
	let Node::Symbol(word) = element.drop_meta() else { return None };
	if variables.contains(word) {
		return None;
	}
	let count = match one_based.drop_meta() {
		Node::Number(Number::Int(one_based)) => Node::int(one_based - 1),
		other => crate::wasp_parser::subscript_key(other)?.clone(),
	};
	zero_list(count, word)
}

/// `int[100]` or `100 * int`: the zero-filled list of that many elements (the parser reads `int[n]` as one already)
fn typed_array_value(value: &Node) -> Option<Node> {
	match value.drop_meta() {
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(call)) if call == ZERO_FILL_CALL) => Some(value.clone()),
		Node::Key(count, Op::Mul, element) => match element.drop_meta() {
			Node::Symbol(word) => zero_list(count.as_ref().clone(), word),
			_ => None,
		},
		other => subscripted_array_type(other),
	}
}

/// The type `[number]`: `list of number` for one element type word
fn bracketed_list_type(type_node: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Square, _) = type_node.drop_meta() else { return None };
	let [element] = items.as_slice() else { return None };
	let Node::Symbol(word) = element.drop_meta() else { return None };
	type_word_kind(word)?;
	Some(Node::Symbol(format!("list of {word}")))
}

/// `x:int[100]` as the variable and its zero-filled list
fn typed_array_declaration(node: &Node) -> Option<(Node, Node)> {
	let Node::Key(name, Op::Colon, type_node) = node.drop_meta() else { return None };
	let Node::Symbol(_) = name.drop_meta() else { return None };
	Some((name.drop_meta().clone(), typed_array_value(type_node)?))
}

/// `x : 100` followed by the element type `int`: the declaration `x = [0 … 0]`
fn counted_array_declaration(declared_count: &Node, type_word: &Node) -> Option<Node> {
	let Node::Key(name, Op::Colon, count) = declared_count.drop_meta() else { return None };
	match (name.drop_meta(), count.drop_meta(), type_word.drop_meta()) {
		(Node::Symbol(_), Node::Number(Number::Int(count)), Node::Symbol(word)) => {
			Some(Node::Key(Box::new(name.drop_meta().clone()), Op::Assign, Box::new(zero_list(Node::int(*count), word)?)))
		}
		_ => None,
	}
}

/// Two neighbouring statements that together form one declaration
fn paired_declaration(first: &Node, second: &Node) -> Option<Node> {
	prefixed_declaration(first, second).or_else(|| counted_array_declaration(first, second))
}

/// The type a lowered declaration `x:T = v` attached to its target x
fn declared_type(target: &Node) -> Option<&Node> {
	match target {
		Node::Meta { data, .. } if matches!(data.as_ref(), Node::Symbol(_)) => Some(data),
		_ => None,
	}
}

/// A builtin type written before a declaration: `int`, `string`, `float`, `real`, `fast` … (see `canonical_type_name`)
fn is_declaration_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if builtin_type_kind(name).is_some() || plural_element_type(name).is_some())
}

/// `(fast x)` as the target of `fast x=v`: the type and the name
fn number_type_prefix(target: &Node) -> Option<(Node, Node)> {
	match target.drop_meta() {
		Node::List(items, bracket, _) if *bracket != Bracket::Round && items.len() == 2 && is_declaration_type(&items[0]) && matches!(items[1].drop_meta(), Node::Symbol(_)) => {
			Some((items[0].drop_meta().clone(), items[1].drop_meta().clone()))
		}
		_ => None,
	}
}

/// The statement pair `fast`, `x=v` as the declaration `x:fast=v`
fn prefixed_declaration(type_name: &Node, next: &Node) -> Option<Node> {
	match next.drop_meta() {
		Node::Key(name, op @ (Op::Assign | Op::Define), value) if is_declaration_type(type_name) && matches!(name.drop_meta(), Node::Symbol(_)) => {
			let target = Node::Key(Box::new(name.drop_meta().clone()), Op::Colon, Box::new(type_name.drop_meta().clone()));
			Some(Node::Key(Box::new(target), *op, value.clone()))
		}
		_ => None,
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

/// Extract user-defined functions from the AST into context
/// Infer return type of a function body given its parameters
/// The parameters and variables of a function body, typed
fn function_body_scope(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Scope {
	let mut scope = Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(closure_variable_targets.clone());
	scope.globals = globals.clone(); // `d = o; return d` of a declared global keeps the global's kind
	for param in params {
		scope.define_param(param.name.clone(), param_kind(param));
	}
	collect_variables(body, &mut scope);
	scope
}

/// The kinds of the values `return a, b` gives back, position by position over every such return:
/// one kind when all agree, Float for Int mixed with Float, else a Node
fn infer_tuple_kinds(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Vec<Kind> {
	let Some(arity) = crate::tuples::tuple_arity(body) else { return vec![] };
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	let mut kinds: Vec<Option<Kind>> = vec![None; arity];
	body.visit(&mut |node| {
		for (kind, value) in kinds.iter_mut().zip(crate::tuples::returned_values(node).unwrap_or_default()) {
			let value_kind = infer_type(value, &scope);
			*kind = Some(match *kind {
				None => value_kind,
				Some(known) if known == value_kind => known,
				Some(known) if known.is_primitive() && value_kind.is_primitive() && (known.is_float() || value_kind.is_float()) => Kind::Float,
				Some(_) => Kind::Data,
			});
		}
	});
	kinds.into_iter().map(|kind| kind.unwrap_or(Kind::Data)).collect()
}

fn infer_function_return_kind(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Kind {
	if crate::tuples::tuple_arity(body).is_some() {
		return Kind::List; // used whole, a tuple function's values are packed into a list
	}
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	// `return error("…")` is the failure path: it does not decide what the function returns
	let is_error = |value: &Node| {
		let value = match value.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "return") => &items[1],
			other => other,
		};
		crate::pipeline::returned_error_message(value).is_some() // `error("…")` or `raise …`
	};
	// `if c { return "text" }; …`: a returned Node makes the function return Nodes
	// (`return [dist, prev]`: a List when every returned Node is one)
	let mut returned_nodes: Vec<Kind> = vec![];
	let mut returned: Vec<Kind> = vec![];
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "return") && !is_error(&items[1]) {
				// `return` alone returns ø: a Node, so the function's numbers are Nodes too
				let kind = if matches!(items[1].drop_meta(), Node::Empty) { Kind::Empty } else { infer_type(&items[1], &scope) };
				returned.push(kind);
				if kind.is_ref() && !returned_nodes.contains(&kind) {
					returned_nodes.push(kind);
				}
			}
		}
	});
	let last = match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => &statements[statements.len() - 1],
		other => other,
	};
	let last_kind = match (is_error(last), returned.first()) {
		(true, Some(_)) if returned.contains(&Kind::Float) && returned.iter().all(|kind| !kind.is_ref()) => Kind::Float,
		(true, Some(first)) if returned.iter().all(|kind| kind == first) => *first,
		(true, Some(_)) => Kind::Empty, // returns of different kinds: a Node of unknown kind
		// `while true { …; return left }` with left a float: a float function, whatever the loop is worth
		_ => match infer_type(last, &scope) {
			Kind::Int if returned.contains(&Kind::Float) => Kind::Float,
			kind => kind,
		},
	};
	match returned_nodes.as_slice() {
		_ if last_kind.is_ref() => last_kind,
		[] => last_kind,
		[Kind::Empty] => Kind::Empty,
		[Kind::List] => Kind::List,
		_ => Kind::Empty, // Nodes of different kinds (a text here, a list there): known only at run time
	}
}

/// `number` is the exact numeric tower (Int); the other builtin type names have their own kind;
/// a list type (`list`, `list of int`, `[int]`, `int[]`, `ints`) is a List
pub(crate) fn annotated_kind(type_node: &Node) -> Option<Kind> {
	if crate::type_constructor::instance_parts_marked(type_node) {
		return Some(Kind::Key); // `p:person` of a declared type (traits::lower_conformances): an instance
	}
	let type_name = type_node.name();
	if bracketed_list_type(type_node).is_some() || names_list_type(&type_name) {
		return Some(Kind::List);
	}
	type_word_kind(&type_name)
}

fn names_list_type(type_name: &str) -> bool {
	type_name == "list" || type_name.starts_with("list of ") || plural_element_type(type_name).is_some()
}

/// The kind a builtin type word names, as annotation (`x:double`) or constructor (`double 2`)
/// The key under which Scope::function_kinds holds the declared kind of a field (as tuples::element_key holds a
/// tuple element's): `v.x` reads a float when every declared type with a field x declares it `float`
pub fn field_kind_key(field: &str) -> String {
	format!(".{field}")
}

/// The fields of the declared types whose declarations all agree on one builtin kind, keyed by field_kind_key
pub fn declared_field_kinds(registry: &crate::type_kinds::TypeRegistry) -> HashMap<String, Kind> {
	let mut kinds: HashMap<String, Option<Kind>> = HashMap::new();
	for field in registry.types().iter().flat_map(|type_def| &type_def.fields) {
		let kind = type_word_kind(&field.type_name.to_lowercase());
		kinds.entry(field_kind_key(&field.name)).and_modify(|known| if *known != kind { *known = None }).or_insert(kind);
	}
	kinds.into_iter().filter_map(|(key, kind)| Some((key, kind?))).collect()
}

/// The declared field kinds of a program
fn program_field_kinds(program: &Node) -> HashMap<String, Kind> {
	let mut registry = crate::type_kinds::TypeRegistry::new();
	collect_all_types(&mut registry, program);
	declared_field_kinds(&registry)
}

pub fn type_word_kind(type_name: &str) -> Option<Kind> {
	match type_name {
		"number" => Some(Kind::Int),
		_ => builtin_type_kind(type_name),
	}
}

/// A parameter takes its declared `x:type` kind, else the kind of its default value (a fresh value per call); otherwise Int
pub fn param_kind(param: &Param) -> Kind {
	if let Some(kind) = param.annotation.as_ref().and_then(annotated_kind) {
		return kind;
	}
	match param.default.as_ref().map(|value| argument_literal_kind(value).unwrap_or_else(|| infer_type(value, &Scope::new()))) {
		Some(kind @ (Kind::Float | Kind::Text | Kind::List)) => kind,
		_ => param.used_as.unwrap_or(Kind::Int),
	}
}

/// Every definition form (`f(x) := …`, `fn`, `def`, `function`) infers its parameter and return kinds alike
fn user_function(name: &str, params: Vec<Param>, body: &Node) -> UserFunctionDef {
	let params = with_usage_kinds(params, body);
	let return_kind = infer_function_return_kind(&params, body, &HashMap::new(), &HashMap::new(), &HashMap::new());
	UserFunctionDef { name: name.to_string(), params, body: Box::new(body.clone()), return_kind, tuple_kinds: vec![], func_index: None }
}

/// Undeclared parameters that the body indexes (`xs#2`, `it[1]`) or reads as a map (`g.keys()`, `xs.has(x)`) take a list
fn with_usage_kinds(params: Vec<Param>, body: &Node) -> Vec<Param> {
	let mut indexed: HashSet<String> = HashSet::new();
	let mut called: HashSet<String> = HashSet::new();
	let mut aliases: Vec<(String, String)> = vec![];
	body.visit(&mut |node| {
		if let Some(name) = crate::closures::called_closure(node) {
			called.insert(name.to_string());
		}
		if let Node::List(items, Bracket::Round, _) = node {
			if let [word, map, ..] = items.as_slice() {
				// `t.left = 3` lowers to `t = field_with(t, "left", 3)`: t is an object too
				let is_map_word = matches!(word.drop_meta(), Node::Symbol(call) if crate::library_words::MAP_WORD_FUNCTIONS.contains(&call.as_str()) || call == crate::library_words::FIELD_WITH);
				if let (true, Node::Symbol(name)) = (is_map_word, map.drop_meta()) {
					indexed.insert(name.clone());
				}
			}
		}
		if let Some(Node::Symbol(name)) = used_sequence(node).map(Node::drop_meta) {
			indexed.insert(name.clone());
		}
		if let Node::Key(alias, Op::Assign | Op::Define, source) = node {
			if let (Node::Symbol(alias), Node::Symbol(source)) = (alias.drop_meta(), source.drop_meta()) {
				aliases.push((alias.clone(), source.clone()));
			}
		}
	});
	// `for x in xs` walks the copy `x·items = xs`: the source of a used sequence is one too
	while let Some((_, source)) = aliases.iter().find(|(alias, source)| indexed.contains(alias) && !indexed.contains(source)) {
		indexed.insert(source.clone());
	}
	params.into_iter().map(|param| {
		let used_as = if called.contains(&param.name) {
			Some(Kind::Function)
		} else if indexed.contains(&param.name) {
			Some(Kind::List)
		} else {
			None
		};
		Param { used_as, ..param }
	}).collect()
}

/// The sequence a node uses: indexed `xs#2`, counted `#xs`, `count xs`, `xs.length`
fn used_sequence(node: &Node) -> Option<&Node> {
	let is_counting_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if is_counting_property(name) && !TYPE_WORDS_AMONG_COUNTING.contains(&name.as_str()));
	match node {
		Node::Key(list, Op::Hash, counted) => Some(if list.is_nothing() { counted } else { list }),
		Node::Key(list, Op::Dot, property) if is_counting_word(property) => Some(list),
		Node::List(items, _, _) => match items.as_slice() {
			[word, counted] if is_counting_word(word) => Some(counted),
			_ => None,
		},
		_ => None,
	}
}

/// A parameter forwarded to another function's parameter takes its kind: `f(grid) { g(grid) }` with g indexing grid
fn infer_forwarded_parameters(ctx: &mut Context) {
	loop {
		let mut forwarded: Vec<(String, usize, Kind)> = vec![];
		for function in ctx.user_functions.values() {
			function.body.visit(&mut |node| {
				let Node::List(items, _, _) = node else { return };
				// `closure_new(target, captured…)` passes the captured values to the target's first parameters
				let items = match crate::closures::as_closure_new(node) {
					Some(_) => &items[1..],
					None => &items[..],
				};
				let Some(Node::Symbol(callee)) = items.first().map(Node::drop_meta) else { return };
				let Some(called) = ctx.user_functions.get(callee) else { return };
				for (argument, called_param) in items[1..].iter().zip(&called.params) {
					let Node::Symbol(name) = argument.drop_meta() else { continue };
					let Some(index) = function.params.iter().position(|param| &param.name == name) else { continue };
					let (param, kind) = (&function.params[index], param_kind(called_param));
					if kind != Kind::Int && param_kind(param) == Kind::Int && param.annotation.is_none() && param.default.is_none() {
						forwarded.push((function.name.clone(), index, kind));
					}
				}
			});
		}
		if forwarded.is_empty() {
			return;
		}
		for (name, index, kind) in forwarded {
			ctx.user_functions.get_mut(&name).expect("collected from user functions").params[index].used_as = Some(kind);
		}
	}
}

/// Recognizes patterns:
/// - `name(param) = body` → Key(List[name, param], Assign, body)
/// - `name := body` → Key(Symbol(name), Define, body) (uses implicit `it`)
pub fn extract_user_functions(ctx: &mut Context, node: &Node) {
	extract_user_functions_inner(ctx, node);
	crate::closures::register_closure_calls(ctx, node);
	infer_parameters_from_calls(ctx, node);
	infer_forwarded_parameters(ctx);
	infer_closure_parameters(ctx, node);
	infer_forwarded_parameters(ctx); // the kinds closures gave their parameters reach the functions that pass them
	let globals = declared_globals(node);
	ctx.field_kinds = program_field_kinds(node);
	let globals = with_closure_captures(ctx, node, globals);
	refine_return_kinds(ctx, &globals);
	// a widened parameter can make the arguments it passes on floats too: until nothing changes (each round widens one)
	let parameter_count: usize = ctx.user_functions.values().map(|function| function.params.len()).sum();
	for _ in 0..parameter_count {
		if !widen_parameters(ctx, node, &globals) {
			break;
		}
		refine_return_kinds(ctx, &globals);
	}
	crate::closures::type_closure_calls(ctx);
}

/// A parameter that the calls pass one kind other than Int takes that kind: `mul(v, 1.0 / length(v))` with length
/// returning a float, `print_tree(tree.left, prefix + "│ ")` passing a text. infer_parameters_from_calls knows only
/// literal arguments; this pass runs once the return kinds are known. It only changes undeclared Int parameters, and
/// leaves a parameter alone when calls disagree. True when a parameter changed.
fn widen_parameters(ctx: &mut Context, program: &Node, globals: &HashMap<String, Local>) -> bool {
	let mut function_kinds: HashMap<String, Kind> = ctx.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind)).collect();
	function_kinds.extend(ctx.field_kinds.clone());
	let mut passed: HashMap<(String, usize), HashSet<Kind>> = HashMap::new();
	let mut main = Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(ctx.closure_variable_targets.clone());
	collect_variables(program, &mut main);
	collect_argument_kinds(program, &main, ctx, &mut passed);
	for function in ctx.user_functions.values() {
		let scope = function_body_scope(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets);
		collect_argument_kinds(&function.body, &scope, ctx, &mut passed);
	}
	let mut changed = false;
	for ((name, index), kinds) in passed {
		let kinds: Vec<Kind> = kinds.into_iter().filter(|kind| *kind != Kind::Int && *kind != Kind::Empty).collect();
		let [kind] = kinds.as_slice() else { continue };
		let param = &mut ctx.user_functions.get_mut(&name).expect("collected from known functions").params[index];
		if param.annotation.is_none() && param.default.is_none() && matches!(param.used_as, None | Some(Kind::Int)) {
			param.used_as = Some(*kind);
			changed = true;
		}
	}
	changed
}

/// The user functions a list calls and their arguments: `f(a, b)`, and a task start that runs f in another instance with
/// them: `task·go(f, a, b)` (lower_tasks), `task_spawn("f", a, b)`, and `task_spawn_values("f·node", [a, b])`, which
/// calls the wrapper f·node with the list and through it f with the items (resolve_tasks)
fn called_functions(items: &[Node]) -> Vec<(String, Vec<&Node>)> {
	let Some(Node::Symbol(head)) = items.first().map(Node::drop_meta) else { return vec![] };
	let started = match items.get(1).map(Node::drop_meta) {
		Some(Node::Text(name) | Node::Symbol(name)) => Some(name.clone()),
		_ => None,
	};
	match head.as_str() {
		crate::declarations::TASK_GO | crate::host::TASK_SPAWN => started.map(|name| (name, items[2..].iter().collect())).into_iter().collect(),
		crate::host::TASK_SPAWN_VALUES | crate::host::GUARDED_CALL => match (started, items.get(2)) {
			(Some(wrapper), Some(list)) => {
				let mut calls = vec![(wrapper.clone(), vec![list])];
				if let (Some(function), Node::List(arguments, _, _)) = (wrapper.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX), list.drop_meta()) {
					calls.push((function.to_string(), arguments.iter().collect()));
				}
				calls
			}
			_ => vec![],
		},
		name => vec![(name.to_string(), items[1..].iter().collect())],
	}
}

/// The kinds each call in `node` passes each parameter of a user function, not looking into nested definitions
fn collect_argument_kinds(node: &Node, scope: &Scope, ctx: &Context, passed: &mut HashMap<(String, usize), HashSet<Kind>>) {
	if function_definition_body(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::List(items, _, _) => {
			for (name, arguments) in called_functions(items) {
				if let Some(function) = ctx.user_functions.get(&name) {
					for (index, argument) in arguments.into_iter().enumerate().take(function.params.len()) {
						passed.entry((name.clone(), index)).or_default().insert(infer_type(argument, scope));
					}
				}
			}
			items.iter().for_each(|item| collect_argument_kinds(item, scope, ctx, passed));
		}
		Node::Key(left, _, right) => {
			collect_argument_kinds(left, scope, ctx, passed);
			collect_argument_kinds(right, scope, ctx, passed);
		}
		Node::Meta { node, .. } => collect_argument_kinds(node, scope, ctx, passed),
		_ => {}
	}
}

/// Free variables a function reads from main (or an enclosing function) keep their kinds for return-kind inference:
/// without them `if x > limit then limit else x` types `limit` as Symbol, the function as Text, and a numeric `+` of
/// two calls traps (g-rT0c). Closures need the same (`t = "!"; shout = s => s + t`). Kinds are merged only where no
/// declared global of that name exists.
fn with_closure_captures(ctx: &Context, program: &Node, mut globals: HashMap<String, Local>) -> HashMap<String, Local> {
	let mut outer = Scope::new();
	collect_variables(program, &mut outer);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	for function in functions {
		let enclosing = ctx.enclosing_functions.get(&function.name).and_then(|name| ctx.user_functions.get(name));
		let captured: Vec<(String, Kind)> = match enclosing {
			Some(enclosing) => {
				let mut enclosing_scope = Scope::new();
				for param in &enclosing.params {
					enclosing_scope.define_param(param.name.clone(), param_kind(param));
				}
				collect_variables(&enclosing.body, &mut enclosing_scope);
				let mut captured = captured_variables(function, &enclosing_scope);
				captured.extend(
					captured_variables(function, &outer)
						.into_iter()
						.filter(|(name, _)| enclosing_scope.lookup(name).is_none()),
				);
				captured
			}
			None => captured_variables(function, &outer),
		};
		// the binding itself, its declared or literal type with it (`k = {a: 10}`: `k.a` is an Int there too)
		for (name, kind) in captured {
			let binding = outer.lookup(&name).cloned().map(|local| Local { kind, ..local });
			globals.entry(name.clone()).or_insert_with(|| binding.unwrap_or_else(|| Local::new(0, name, kind)));
		}
	}
	globals
}

/// The program's `global` declarations with their kinds
fn declared_globals(program: &Node) -> HashMap<String, Local> {
	let mut scope = Scope::new();
	collect_variables(program, &mut scope);
	scope.globals
}

/// `f -x` with a user or built-in function `f` is the call `f(-x)`: a function is never an operand of a subtraction.
/// A built-in name that the program also binds as a variable or parameter (`exp-1`) stays a subtraction.
pub fn lower_negated_calls(node: Node) -> Node {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &node);
	let mut bound: HashSet<String> = ctx.user_functions.values().flat_map(|function| function.params.iter().map(|param| param.name.clone())).collect();
	crate::library_words::collect_assigned_names(&node, &mut bound);
	negate_calls(node, &ctx.user_functions, &bound)
}

/// The functions a word can apply: the program's functions that take arguments and the built-in real functions
pub fn applicable_function_names(node: &Node) -> HashSet<String> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, node);
	let user_functions = ctx.user_functions.into_iter().filter(|(_, function)| !function.params.is_empty()).map(|(name, _)| name);
	user_functions.chain(crate::real::FUNCTIONS.iter().map(|name| name.to_string())).collect()
}

fn negate_calls(node: Node, functions: &HashMap<String, UserFunctionDef>, bound: &HashSet<String>) -> Node {
	let is_function = |operand: &Node| match operand.drop_meta() {
		Node::Symbol(name) => functions.get(name).is_some_and(|function| !function.params.is_empty())
			|| (crate::real::FUNCTIONS.contains(&name.as_str()) && !bound.contains(name)),
		_ => false,
	};
	match node {
		Node::Key(function, Op::Sub, argument) if is_function(&function) => {
			let negated = Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(negate_calls(*argument, functions, bound)));
			Node::List(vec![*function, negated], Bracket::Round, Separator::None)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(negate_calls(*left, functions, bound)), op, Box::new(negate_calls(*right, functions, bound))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| negate_calls(item, functions, bound)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(negate_calls(*node, functions, bound)), data },
		other => other,
	}
}

/// A lifted lambda is never called by name (closures.rs): its captured values take the kinds of the variables captured (a
/// parameter of the enclosing function, a literal variable), its parameters the kinds of the literal arguments of the closure
/// calls of its arity
fn infer_closure_parameters(ctx: &mut Context, program: &Node) {
	let variable_kinds = variable_kinds(program, ctx, true);
	loop {
		let mut inferred: Vec<(String, usize, Kind)> = vec![];
		let scopes = std::iter::once((program, HashMap::new())).chain(ctx.user_functions.values().map(|function| {
			let params: HashMap<String, Kind> = function.params.iter().map(|param| (param.name.clone(), param_kind(param))).collect();
			(function.body.as_ref(), params)
		}));
		for (body, params) in scopes {
			body.visit(&mut |node| {
				let value_kind = |value: &Node| match value.drop_meta() {
					Node::Symbol(name) => params.get(name).or(variable_kinds.get(name)).copied(),
					_ => argument_literal_kind(value),
				};
				if let Some((target, captured)) = crate::closures::as_closure_new(node) {
					inferred.extend(captured.iter().enumerate().filter_map(|(index, value)| Some((target.to_string(), index, value_kind(value)?))));
				}
				let Node::List(items, _, _) = node else { return };
				let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else { return };
				let Some(arity) = crate::closures::closure_call_arity(name) else { return };
				for (target, captured) in ctx.closure_targets.iter().filter(|(target, captured)| ctx.user_functions.get(target).is_some_and(|function| function.params.len() == captured + arity)) {
					inferred.extend(items[2..].iter().enumerate().filter_map(|(index, argument)| Some((target.clone(), captured + index, value_kind(argument)?))));
				}
			});
		}
		let mut changed = false;
		for (target, index, kind) in inferred {
			let Some(param) = ctx.user_functions.get_mut(&target).and_then(|function| function.params.get_mut(index)) else { continue };
			if kind != Kind::Int && param.annotation.is_none() && param.default.is_none() && param.used_as.is_none() {
				param.used_as = Some(kind);
				changed = true;
			}
		}
		if !changed {
			return;
		}
	}
}

/// `a List`, `an Int`
pub fn kind_with_article(kind: Kind) -> String {
	with_article(&format!("{kind:?}"))
}

/// `a photo`, `an Int`, `an image`
pub fn with_article(name: &str) -> String {
	let article = if name.starts_with(|first: char| "AEIOUaeiou".contains(first)) { "an" } else { "a" };
	format!("{article} {name}")
}

/// The kind a call argument certainly has, judged from the literal alone
pub fn argument_literal_kind(argument: &Node) -> Option<Kind> {
	match argument.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _) => Some(infer_type(argument, &Scope::new())),
		Node::Empty => Some(Kind::List),
		// `1.5 as float`, `x as float`: the kind the conversion names
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => Some(Kind::Float),
		_ if crate::closures::as_closure_new(argument).is_some() => Some(Kind::Function),
		Node::List(entries, Bracket::Curly, _) if entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) => Some(Kind::List), // a map
		_ => None,
	}
}

/// The kinds of variables only ever assigned literals of one kind: `s="abcd"; f(s)` passes a Text.
/// A parameter of the same name shadows the variable (wiki/Footguns.md "Parameter shadowing"), so it is not judged.
pub(crate) fn literal_variable_kinds(program: &Node, ctx: &Context) -> HashMap<String, Kind> {
	variable_kinds(program, ctx, false)
}

/// literal_variable_kinds where a value of unknown kind leaves the literal's kind standing: `s = 0.5 as float; s = g(s)`
/// keeps s a Float, which types the closure g takes (infer_closure_parameters)
fn variable_kinds(program: &Node, ctx: &Context, unknown_keeps_kind: bool) -> HashMap<String, Kind> {
	let mut kinds: HashMap<String, Option<Kind>> = HashMap::new();
	// a parameter of the same name shadows the variable; not for closures, whose captured parameters bear its name
	for param in ctx.user_functions.values().flat_map(|function| &function.params).filter(|_| !unknown_keeps_kind) {
		kinds.insert(param.name.clone(), None);
	}
	let mut literal: HashSet<String> = HashSet::new();
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let kind = argument_literal_kind(value);
		if kind.is_none() && unknown_keeps_kind {
			return;
		}
		literal.insert(name.clone());
		kinds.entry(name.clone()).and_modify(|known| if *known != kind { *known = None }).or_insert(kind);
	});
	kinds.into_iter().filter(|(name, _)| !unknown_keeps_kind || literal.contains(name)).filter_map(|(name, kind)| Some((name, kind?))).collect()
}

/// A decimal literal, `2.5` or `2.0` (an exact decimal, so its kind may be Int): a float an int parameter refuses (P49, P49b)
fn is_decimal_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(crate::extensions::numbers::Number::Float(_)))
}

/// Variables assigned a decimal literal (`y = 2.5`, `y = 3.0`)
fn decimal_variables(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = node {
			if let (Node::Symbol(name), true) = (target.drop_meta(), is_decimal_literal(value)) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// An undeclared parameter takes the kind of its arguments when every call agrees, over the kind its use suggests
/// (an indexed parameter is a List unless it is passed a Text); calls that disagree are a reported conflict.
/// Parameters that only get Int arguments (or none) keep their usage kind. Arguments that are neither literals nor
/// literal variables are not judged here: a wrong one is still refused at the call by the emitter.
fn infer_parameters_from_calls(ctx: &mut Context, program: &Node) {
	let variable_kinds = literal_variable_kinds(program, ctx);
	let argument_kind = |argument: &Node| argument_literal_kind(argument).or_else(|| match argument.drop_meta() {
		Node::Symbol(name) => variable_kinds.get(name).copied(),
		_ => None,
	});
	let mut argument_kinds: HashMap<(String, usize), Vec<Kind>> = HashMap::new();
	// a float passed to a declared int parameter loses digits: a compile error (P49)
	let mut float_for_int: Vec<String> = Vec::new();
	// a parameter of the same name shadows the variable (as in literal_variable_kinds)
	let mut decimal_variables = decimal_variables(program);
	ctx.user_functions.values().flat_map(|function| &function.params).for_each(|param| { decimal_variables.remove(&param.name); });
	let has_digits_an_int_loses = |argument: &Node| is_decimal_literal(argument) || argument_kind(argument) == Some(Kind::Float)
		|| matches!(argument.drop_meta(), Node::Symbol(name) if decimal_variables.contains(name));
	program.visit(&mut |node| {
		let Node::List(items, _, _) = node else { return };
		for (name, arguments) in called_functions(items) {
			let Some(function) = ctx.user_functions.get(&name) else { continue };
			for (index, argument) in arguments.into_iter().enumerate().take(function.params.len()) {
				let declared_int = function.params[index].annotation.is_some() && param_kind(&function.params[index]) == Kind::Int;
				if declared_int && has_digits_an_int_loses(argument) {
					// a whole float serializes as `2`; its decimal point is what makes it no int
					let written = match argument.drop_meta() {
						Node::Number(crate::extensions::numbers::Number::Float(value)) if value.fract() == 0.0 => format!("{value:.1}"),
						other => other.serialize(),
					};
					float_for_int.push(format!("{written} is no int: write {written} as int"));
				}
				if let Some(kind) = argument_kind(argument) {
					let kinds = argument_kinds.entry((name.clone(), index)).or_default();
					if !kinds.contains(&kind) {
						kinds.push(kind);
					}
				}
			}
		}
	});
	ctx.parameter_conflicts.extend(float_for_int);
	for ((name, index), kinds) in argument_kinds {
		let function = ctx.user_functions.get_mut(&name).expect("call sites were collected from known functions");
		let param = &mut function.params[index];
		if param.annotation.is_some() || param.default.is_some() {
			continue;
		}
		match kinds.as_slice() {
			[Kind::Int] => {}
			[kind] => param.used_as = Some(*kind),
			[first, second, ..] => ctx.parameter_conflicts.push(format!(
				"{name} is called with {} and {} for parameter {}: annotate it",
				kind_with_article(*first), kind_with_article(*second), param.name)),
			[] => {}
		}
	}
}

/// Infer every return kind again knowing all user functions (they shadow FFI names, recursion assumes Int first),
/// until the kinds settle: a call of a float-returning function is itself Float
fn refine_return_kinds(ctx: &mut Context, globals: &HashMap<String, Local>) {
	let mut function_kinds: HashMap<String, Kind> = ctx.user_functions.keys().map(|name| (name.clone(), Kind::Int)).collect();
	function_kinds.extend(ctx.field_kinds.clone());
	for _ in 0..=ctx.user_functions.len() {
		let inferred: Vec<(String, Kind)> = ctx.user_functions.values()
			.map(|function| {
				let kind = crate::closures::closure_call_kind(&function.name, ctx, &function_kinds)
					.unwrap_or_else(|| infer_function_return_kind(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets));
				// `if t == ø { return [] }; return f(t.left) + [x]`: assumed to return an Int, the recursion makes an
				// error of the last value; assumed to return a Node of unknown kind, it settles
				let kind = if kind == Kind::Error && function_kinds.get(&function.name) == Some(&Kind::Int) {
					let mut unknown_recursion = function_kinds.clone();
					unknown_recursion.insert(function.name.clone(), Kind::Empty);
					infer_function_return_kind(&function.params, &function.body, &unknown_recursion, globals, &ctx.closure_variable_targets)
				} else {
					kind
				};
				(function.name.clone(), kind)
			})
			.collect();
		let tuples: Vec<(String, Kind)> = ctx.user_functions.values()
			.flat_map(|function| {
				let kinds = infer_tuple_kinds(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets);
				kinds.into_iter().enumerate().map(|(index, kind)| (crate::tuples::element_key(&function.name, index), kind)).collect::<Vec<_>>()
			})
			.collect();
		let settled = inferred.iter().chain(&tuples).all(|(name, kind)| function_kinds.get(name) == Some(kind));
		function_kinds.extend(inferred);
		function_kinds.extend(tuples);
		if settled {
			break;
		}
	}
	for function in ctx.user_functions.values_mut() {
		function.return_kind = function_kinds[&function.name];
		function.tuple_kinds = (0..crate::tuples::tuple_arity(&function.body).unwrap_or(0))
			.map(|index| function_kinds[&crate::tuples::element_key(&function.name, index)])
			.collect();
	}
}

fn extract_user_functions_inner(ctx: &mut Context, node: &Node) {
	extract_user_functions_in(ctx, node, None);
}

/// Nested `def` / `f() = …` inside a function body becomes a top-level UserFunctionDef named `outer·inner`, with call
/// sites in the enclosing body rewritten to that mangled name (wiki/charged.md §3 nested defs; card g-qUkY step 1).
fn extract_user_functions_in(ctx: &mut Context, node: &Node, enclosing: Option<&str>) {
	let node = node.drop_meta();
	match node {
		// Pattern: name(param1, param2, ...) = body
		Node::Key(left, Op::Assign, body) => {
			if let Node::List(items, bracket, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						register_user_function(ctx, name, extract_params(items, bracket), body, enclosing);
						return;
					}
				}
			}
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, body, enclosing);
		}
		// Pattern: name x := body (with explicit parameter x using $0 or `it`)
		Node::Key(left, Op::Define, body) => {
			if let Node::List(items, bracket, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						let params = extract_params(items, bracket);
						let implicit_param = uses_dollar_param(body) || uses_it(body);
						let written_call = *bracket == Bracket::Round; // `f() := [1, 2]` defines f without parameters
						if !params.is_empty() || implicit_param || written_call {
							let params = if params.is_empty() && implicit_param { vec![Param::untyped("it")] } else { params };
							register_user_function(ctx, name, params, body, enclosing);
							return;
						}
					}
				}
			}
			// Pattern: name := body (implicit `it` parameter, or none for a statement block)
			if let Node::Symbol(name) = left.drop_meta() {
				if uses_it(body) || uses_dollar_param(body) || is_statement_block(body) {
					// a block of statements takes `it` when it reads it outside its own `for 1..n {…it…}` loops
					let takes_it = !is_statement_block(body) || uses_it_outside_loops(body) || uses_dollar_param(body);
					let params = if takes_it { vec![Param::untyped("it")] } else { vec![] };
					register_user_function(ctx, name, params, body, enclosing);
					return;
				}
			}
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, body, enclosing);
		}
		// Check for def/fun/fn syntax
		Node::List(items, _, _) => {
			if items.len() >= 2 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					if is_function_keyword(s) {
						if let Some(func_def) = extract_def_function(&items[1..]) {
							register_user_function(ctx, &func_def.name, func_def.params, &func_def.body, enclosing);
							return;
						}
					}
				}
			}
			for item in items {
				extract_user_functions_in(ctx, item, enclosing);
			}
		}
		Node::Key(left, _, right) => {
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, right, enclosing);
		}
		_ => {}
	}
}

/// Between an enclosing function and a nested `def` lifted out of its body: `outer·inner`
const NESTED_DEF_SEPARATOR: &str = "·";

fn qualify_nested_name(enclosing: Option<&str>, name: &str) -> String {
	match enclosing {
		Some(parent) => format!("{parent}{NESTED_DEF_SEPARATOR}{name}"),
		None => name.to_string(),
	}
}

fn register_user_function(ctx: &mut Context, short_name: &str, params: Vec<Param>, body: &Node, enclosing: Option<&str>) {
	let name = qualify_nested_name(enclosing, short_name);
	let (body, renames) = lift_nested_defs_from_body(ctx, body.clone(), &name);
	let body = rename_nested_calls(body, &renames);
	if let Some(parent) = enclosing {
		ctx.enclosing_functions.insert(name.clone(), parent.to_string());
	}
	ctx.user_functions.insert(name.clone(), user_function(&name, params, &body));
}

/// Nested function definitions in `body` become UserFunctionDefs named `parent·short`; their statements stay (emit skips
/// them) and call sites are renamed via the returned map.
fn lift_nested_defs_from_body(ctx: &mut Context, body: Node, parent: &str) -> (Node, HashMap<String, String>) {
	let mut renames = HashMap::new();
	let body = lift_nested_defs_walk(ctx, body, parent, &mut renames);
	(body, renames)
}

fn lift_nested_defs_walk(ctx: &mut Context, node: Node, parent: &str, renames: &mut HashMap<String, String>) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lift_nested_defs_walk(ctx, *node, parent, renames)), data },
		Node::Key(left, op @ (Op::Assign | Op::Define), body) => {
			if let Node::List(items, bracket, separator) = left.drop_meta() {
				if let Some(Node::Symbol(short)) = items.first().map(Node::drop_meta) {
					let mangled = qualify_nested_name(Some(parent), short);
					renames.insert(short.clone(), mangled.clone());
					register_user_function(ctx, short, extract_params(items, bracket), &body, Some(parent));
					// LHS must use the mangled name so emit's defined_function_name finds it in user_functions
					let mut items = items.clone();
					items[0] = Node::Symbol(mangled);
					return Node::Key(Box::new(Node::List(items, bracket.clone(), separator.clone())), op, body);
				}
			}
			Node::Key(
				Box::new(lift_nested_defs_walk(ctx, *left, parent, renames)),
				op,
				Box::new(lift_nested_defs_walk(ctx, *body, parent, renames)),
			)
		}
		Node::Key(left, op, right) => Node::Key(
			Box::new(lift_nested_defs_walk(ctx, *left, parent, renames)),
			op,
			Box::new(lift_nested_defs_walk(ctx, *right, parent, renames)),
		),
		// quoted data (the program a run-time block carries) is not code of this body
		quoted if crate::run_time_blocks::is_data(&quoted) => quoted,
		Node::List(items, bracket, separator) => {
			if items.len() >= 2 {
				if let Node::Symbol(keyword) = items[0].drop_meta() {
					if is_function_keyword(keyword) {
						if let Some(func_def) = extract_def_function(&items[1..]) {
							renames.insert(func_def.name.clone(), qualify_nested_name(Some(parent), &func_def.name));
							register_user_function(ctx, &func_def.name, func_def.params, &func_def.body, Some(parent));
							return Node::List(items, bracket, separator);
						}
					}
				}
			}
			Node::List(
				items.into_iter().map(|item| lift_nested_defs_walk(ctx, item, parent, renames)).collect(),
				bracket,
				separator,
			)
		}
		other => other,
	}
}

/// `inner()` / `inner()+1` inside the enclosing body: the call head becomes `outer·inner`
fn rename_nested_calls(node: Node, renames: &HashMap<String, String>) -> Node {
	if renames.is_empty() {
		return node;
	}
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(rename_nested_calls(*node, renames)), data },
		Node::Key(left, op, right) => Node::Key(
			Box::new(rename_nested_calls(*left, renames)),
			op,
			Box::new(rename_nested_calls(*right, renames)),
		),
		Node::List(items, bracket, separator) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| rename_nested_calls(item, renames)).collect();
			if bracket == Bracket::Round {
				if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
					if let Some(mangled) = renames.get(name) {
						items[0] = Node::Symbol(mangled.clone());
					}
				}
			}
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

/// The parameters after the function name in a signature list. Only a spaced signature `f int x` pairs a type word with
/// the next name; in the call form `f(T, y)` every item is a parameter (the parser made `f(int x)` the item `x:int`)
fn extract_params(signature: &[Node], bracket: &Bracket) -> Vec<Param> {
	// `f(xs: list of int, y)`: the comma groups `xs:list of int` as one spaced item
	let flat: Vec<&Node> = signature.iter().skip(1).flat_map(|item| match item.drop_meta() {
		Node::List(parts, _, Separator::Space | Separator::None) if matches!(parts.first().map(Node::drop_meta), Some(Node::Key(_, Op::Colon, _))) => parts.iter().collect(),
		_ => vec![item],
	}).collect();
	let mut items = flat.into_iter().peekable();
	let mut params = vec![];
	while let Some(item) = items.next() {
		let type_first_name = match (item.drop_meta(), items.peek().map(|next| next.drop_meta())) {
			(Node::Symbol(type_name), Some(Node::Symbol(name))) if *bracket != Bracket::Round && type_word_kind(type_name).is_some() => Some(name),
			_ => None,
		};
		match type_first_name {
			Some(name) => {
				params.push(Param { name: name.clone(), annotation: Some(item.clone()), default: None, used_as: None });
				items.next();
			}
			None => params.extend(extract_param(item).map(|param| with_list_annotation(param, &mut items))),
		}
	}
	params
}

/// `xs: T list` and `xs: list of T` (the parser leaves `list`, or `of T`, as the next items) annotate xs as a `list of T`:
/// T a type word, a declared type or a trait (`sort(xs: Comparable list)`)
fn with_list_annotation<'a>(param: Param, items: &mut std::iter::Peekable<impl Iterator<Item = &'a Node>>) -> Param {
	let Some(annotation) = param.annotation.as_ref().map(Node::name) else { return param };
	let next_word = |items: &mut std::iter::Peekable<_>| match items.peek().map(|next: &&Node| next.drop_meta()) {
		Some(Node::Symbol(word)) => Some(word.clone()),
		_ => None,
	};
	let element = match next_word(items).as_deref() {
		Some(LIST_WORD) => annotation,
		Some(OF_WORD) if annotation == LIST_WORD => {
			items.next();
			match next_word(items) {
				Some(element) => element,
				None => return param,
			}
		}
		_ => return param,
	};
	items.next();
	Param { annotation: Some(Node::Symbol(format!("{LIST_OF_PREFIX}{element}"))), ..param }
}

/// Extract parameter name, annotated kind and optional default value from a parameter node
fn extract_param(item: &Node) -> Option<Param> {
	match item.drop_meta() {
		Node::Symbol(s) => Some(Param::untyped(s)),
		Node::Key(n, Op::Colon, type_name) => {
			if let Node::Symbol(s) = n.drop_meta() {
				Some(Param { name: s.clone(), annotation: Some(type_name.as_ref().clone()), default: None, used_as: None })
			} else {
				None
			}
		}
		Node::Key(n, Op::Assign, default) => {
			if let Node::Symbol(s) = n.drop_meta() {
				Some(Param { name: s.clone(), annotation: None, default: Some(default.as_ref().clone()), used_as: None })
			} else {
				None
			}
		}
		_ => None,
	}
}

/// Check if a node uses the implicit `it` parameter
fn uses_it(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s == "it" => true,
		Node::Key(left, _, right) => uses_it(left) || uses_it(right),
		Node::List(items, _, _) => items.iter().any(uses_it),
		_ => false,
	}
}

/// Check if a node uses $n parameter references (e.g., $0, $1)
fn uses_dollar_param(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s.starts_with('$') && s[1..].parse::<u32>().is_ok() => true,
		Node::Key(left, _, right) => uses_dollar_param(left) || uses_dollar_param(right),
		Node::List(items, _, _) => items.iter().any(uses_dollar_param),
		_ => false,
	}
}

/// Extract function from def/fun/fn syntax
pub(crate) fn extract_def_function(items: &[Node]) -> Option<UserFunctionDef> {
	if items.is_empty() {
		return None;
	}
	let first = items[0].drop_meta();

	// Pattern 1: def (name params...): body
	if let Node::Key(sig, Op::Colon, body) = first {
		if let Node::List(sig_items, bracket, _) = sig.drop_meta() {
			if !sig_items.is_empty() {
				if let Node::Symbol(name) = sig_items[0].drop_meta() {
					return Some(user_function(name, extract_params(sig_items, bracket), body));
				}
			}
		}
	}

	// Pattern 2: def ((name params...) {body})
	if let Node::List(inner_items, _, _) = first {
		if inner_items.len() >= 2 {
			if let Node::List(sig_items, _, _) = inner_items[0].drop_meta() {
				if !sig_items.is_empty() {
					if let Node::Symbol(name) = sig_items[0].drop_meta() {
						let params: Vec<Param> = sig_items
							.iter()
							.skip(1)
							.flat_map(|item| {
								match item.drop_meta() {
									Node::List(param_items, _, _) => {
										param_items.iter().filter_map(extract_param).collect::<Vec<_>>()
									}
									_ => extract_param(item).into_iter().collect(),
								}
							})
							.collect();
						return Some(user_function(name, params, &inner_items[1]));
					}
				}
			}
		}
	}
	None
}

/// `count x`, `length x`, `size x`: the runtime function counting the elements of x; a text counts its graphemes
/// (user-perceived characters). Bytes are only counted by an explicit unit (`x.bytes`); a user function of that name wins.
pub fn counting_function(name: &str, ctx: &Context) -> Option<&'static str> {
	if ctx.user_functions.contains_key(name) {
		return None;
	}
	if name == BYTE_SIZE {
		return Some("node_bytes");
	}
	(is_counting_property(name) && !TYPE_WORDS_AMONG_COUNTING.contains(&name)).then_some("node_count")
}

/// `x.count`, `x.length`, `x.size`, and the explicit units `x.bytes` (memory), `x.chars` (code points), `x.graphemes`
pub fn counting_method(name: &str, ctx: &Context) -> Option<&'static str> {
	match name {
		_ if is_counting_property(name) => Some("node_count"),
		"bytes" | BYTE_SIZE => Some("node_bytes"),
		"chars" | "codepoints" => Some("text_codepoint_count"),
		"graphemes" => Some("text_grapheme_count"),
		_ => counting_function(name, ctx),
	}
}

/// The counting method of a text unit, named in the singular or the plural: `byte`, `chars`, `codepoint`, `graphemes`.
/// A char is a code point, as `x.chars` and the `char` type; the user-perceived character is a grapheme.
pub(crate) fn text_unit(word: &str) -> Option<&'static str> {
	match word {
		"byte" | "bytes" => Some("bytes"),
		"char" | "chars" | "character" | "characters" | "codepoint" | "codepoints" => Some("codepoints"),
		"grapheme" | "graphemes" => Some("graphemes"),
		_ => None,
	}
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(s) if s == word)
}

/// The items after the first `skip` as one node: the item itself, or the list of them
fn rest_of(items: &[Node], skip: usize, bracket: &Bracket, separator: &Separator) -> Node {
	match &items[skip..] {
		[single] => single.clone(),
		rest => Node::List(rest.to_vec(), bracket.clone(), separator.clone()),
	}
}

/// A text seen in one unit, `byte in t` or `t as bytes`: counting it counts that unit, `t.bytes`
fn unit_count(node: &Node) -> Option<Node> {
	let count_of = |text: Node, unit: &Node| {
		let Node::Symbol(word) = unit.drop_meta() else { return None };
		Some(Node::Key(Box::new(text), Op::Dot, Box::new(Node::Symbol(text_unit(word)?.to_string()))))
	};
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => unit_count(&items[0]),
		Node::List(items, bracket, separator) if items.len() >= 3 && is_word(&items[1], "in") && text_unit(&items[0].name()).is_some() => {
			count_of(rest_of(items, 2, bracket, separator), &items[0])
		}
		Node::Key(text, Op::As, unit) => count_of(text.as_ref().clone(), unit),
		// `"äb" in bytes`: the unit last
		Node::List(items, bracket, separator) if items.len() >= 3 && is_word(&items[items.len() - 2], "in") => {
			let (unit, text) = (&items[items.len() - 1], &items[..items.len() - 2]);
			count_of(rest_of(text, 0, bracket, separator), unit)
		}
		_ => None,
	}
}

/// `#bytes in t`, parsed as the items `#bytes`, `in`, `t`: the count of that unit, `t.bytes`
fn hashed_unit_count(items: &[Node]) -> Option<Node> {
	let [hashed, keyword, _, ..] = items else { return None };
	let Node::Key(empty, Op::Hash, unit) = hashed.drop_meta() else { return None };
	if !matches!(empty.drop_meta(), Node::Empty) || !is_word(keyword, "in") {
		return None;
	}
	let Node::Symbol(word) = unit.drop_meta() else { return None };
	let counted = rest_of(items, 2, &Bracket::None, &Separator::Space);
	Some(Node::Key(Box::new(counted), Op::Dot, Box::new(Node::Symbol(text_unit(word)?.to_string()))))
}

/// `x size`, `x count`, `x length`, `x number`: a counting property word after a name is the getter `x.size`,
/// unless the name is a keyword (`return count`) or the word a variable of the program
fn property_of_name(items: &[Node], separator: &Separator, variables: &HashSet<String>) -> Option<Node> {
	let [name, property] = items else { return None };
	let (Node::Symbol(name_text), Node::Symbol(property_text)) = (name.drop_meta(), property.drop_meta()) else { return None };
	let names_no_object = is_counting_property(name_text) || PROPERTYLESS_KEYWORDS.contains(&name_text.as_str());
	if *separator != Separator::Space || names_no_object || !is_counting_property(property_text) || variables.contains(property_text) {
		return None;
	}
	Some(Node::Key(Box::new(name.clone()), Op::Dot, Box::new(property.clone())))
}

/// `number of x`, `count of x`, `length of x`, `size of x` → `count x`;
/// of a unit, `number of bytes in t` → `t.bytes` (as `#(byte in t)`, `#(t as bytes)`); `byte count of x` → `x.bytes`
/// `print chars in "hello"`: a name no variable has, in a collection, prints each item as `for chars in "hello": print it`
fn print_walk(items: &[Node], variables: &HashSet<String>) -> Option<Node> {
	let [print, phrase] = items else { return None };
	let Node::List(phrase, Bracket::None | Bracket::Round, _) = phrase.drop_meta() else { return None };
	// `chars in "hello"` arrives as the words or as `chars (in "hello")`, optionally after `all`
	let phrase = match phrase.as_slice() {
		[all, rest @ ..] if is_word(all, "all") => rest,
		words => words,
	};
	let (name, in_word, collection) = match phrase {
		[name, in_word, collection] => (name, in_word, collection),
		[name, rest] => match rest.drop_meta() {
			Node::List(rest, _, _) if rest.len() == 2 => (name, &rest[0], &rest[1]),
			_ => return None,
		},
		_ => return None,
	};
	let Node::Symbol(name) = name.drop_meta() else { return None };
	if !is_word(print, "print") || !is_word(in_word, "in") || variables.contains(name) {
		return None;
	}
	let body = Node::List(vec![print.clone(), Node::Symbol(name.clone())], Bracket::Curly, Separator::Space);
	Some(Node::List(vec![Node::Symbol("for".into()), Node::Symbol(name.clone()), in_word.clone(), collection.clone(), body], Bracket::None, Separator::Space))
}

fn counting_phrase(items: &[Node], bracket: &Bracket, separator: &Separator, variables: &HashSet<String>) -> Option<Node> {
	if let [unit, count, of, _, ..] = items {
		if is_word(unit, "byte") && is_word(count, "count") && is_word(of, "of") {
			let counted = rest_of(items, 3, bracket, separator);
			return Some(Node::Key(Box::new(counted), Op::Dot, Box::new(Node::Symbol("bytes".to_string()))));
		}
	}
	if let [count, unit, of, _, ..] = items {
		if is_word(count, "count") && is_word(of, "of") {
			if let Some(unit_property) = matches!(unit.drop_meta(), Node::Symbol(_)).then(|| text_unit(&unit.name())).flatten() {
				return Some(Node::Key(Box::new(rest_of(items, 3, bracket, separator)), Op::Dot, Box::new(Node::Symbol(unit_property.to_string()))));
			}
		}
	}
	if let Some(property) = property_of_name(items, separator, variables) {
		return Some(property);
	}
	let [word, of, _, ..] = items else { return None };
	let Node::Symbol(word) = word.drop_meta() else { return None };
	if !is_counting_property(word) {
		return None;
	}
	let counter = "count";
	if !is_word(of, "of") {
		return None;
	}
	let counted = rest_of(items, 2, bracket, separator);
	Some(unit_count(&counted).unwrap_or_else(|| Node::List(vec![Node::Symbol(counter.to_string()), counted], bracket.clone(), separator.clone())))
}

/// The list a range of integer literals stands for: `1..4` is [1 2 3], `1…4` and `1 to 4` are [1 2 3 4]; not an empty or computed range
fn range_elements(range: &Node) -> Option<Node> {
	let Node::Key(start, op @ (Op::Range | Op::To), end) = range.drop_meta() else { return None };
	let (Node::Number(Number::Int(start)), Node::Number(Number::Int(end))) = (start.drop_meta(), end.drop_meta()) else { return None };
	let last = if *op == Op::To { *end } else { end - 1 };
	(start <= &last).then(|| Node::List((*start..=last).map(Node::int).collect(), Bracket::Square, Separator::Space))
}

/// `xs = a..b` of computed bounds: `(xs·range = ø; for xs·item in a..b { xs·range = xs·range + [xs·item] }; xs·range)`
fn computed_range(target: &Node, range: &Node) -> Option<Node> {
	let Node::Symbol(name) = target.drop_meta() else { return None };
	if !matches!(range.drop_meta(), Node::Key(_, Op::Range | Op::To, _)) || range_elements(range).is_some() {
		return None;
	}
	let symbol = |suffix: &str| Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}{suffix}"));
	let (items, item) = (symbol("range"), symbol("item"));
	let appended = Node::Key(Box::new(items.clone()), Op::Add, Box::new(Node::List(vec![item.clone()], Bracket::Square, Separator::Space)));
	let body = Node::List(vec![Node::Key(Box::new(items.clone()), Op::Assign, Box::new(appended))], Bracket::Curly, Separator::Semicolon);
	let walk = Node::List(vec![Node::Symbol("for".into()), item, Node::Symbol("in".into()), range.clone(), body], Bracket::None, Separator::Space);
	let start = Node::Key(Box::new(items.clone()), Op::Assign, Box::new(Node::Empty));
	Some(Node::List(vec![start, walk, items], Bracket::Round, Separator::Semicolon))
}

fn require_counter(ctx: &mut Context, counter: &'static str) {
	if counter == "node_bytes" {
		ctx.required_functions.insert("node_count");
	}
	ctx.required_functions.insert(counter);
}

/// Analyze node tree for non-default required functions.
/// Default functions (new_empty, new_int, new_float, new_text, new_symbol, new_codepoint, new_key, new_list)
/// are always included and don't need to be inserted here.
pub fn analyze_required_functions(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::Number(number) => {
			let exact_decimal = matches!(number, Number::Float(value) if Number::is_exact_decimal(*value));
			if exact_decimal || !matches!(number, Number::Int(n) if crate::wasm_emitter::is_fixnum(*n)) {
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
		}
		Node::Text(text) => {
			if let Some(number) = crate::wasp_parser::number_in_text(text) {
				analyze_required_functions(ctx, &Node::Number(number));
			}
		}
		// run_block hands exact numbers back by composing them from fixnums (tasks.rs Builders)
		Node::Symbol(name) if name == crate::host::RUN_BLOCK => ctx.required_functions.extend([crate::wasm_emitter::INT_RUNTIME, "new_int"]),
		Node::Empty | Node::Symbol(_) | Node::Char(_) | Node::True | Node::False => {}
		Node::Key(key, op, value) => {
			if op.is_arithmetic()
				|| op.is_shift()
				|| op.is_compound_assign()
				|| matches!(op, Op::Inc | Op::Dec | Op::Neg | Op::Abs | Op::Square | Op::Cube | Op::Xor)
			{
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
			if matches!(op, Op::Eq | Op::Ne) {
				ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
			}
			if matches!(op, Op::If | Op::While | Op::Question | Op::Not | Op::And | Op::Or) {
				ctx.required_functions.insert(crate::wasm_emitter::IS_TRUTHY);
			}
			if *op == Op::Assign || op.is_compound_assign() {
				if let Node::Key(_, Op::Hash, _) = key.drop_meta() {
					ctx.required_functions.insert("node_with_at");
					analyze_required_functions(ctx, key);
					analyze_required_functions(ctx, value);
					return;
				}
			}
			if *op == Op::As && matches!(value.name().to_lowercase().as_str(), "string" | "str" | "text") {
				ctx.required_functions.insert("list_join"); // `x as string` of a variable joins its text
			}
			if *op == Op::As && value.name().to_lowercase() == "list" {
				ctx.required_functions.extend(["text_chars", "list_reverse", "text_reverse"]); // `x as list` of a text
			}
			if *op == Op::Pow {
				ctx.required_functions.insert("i64_pow");
			} else if *op == Op::Square || *op == Op::Cube {
				analyze_required_functions(ctx, key);
				return;
			} else if op.is_prefix() && matches!(key.drop_meta(), Node::Empty) {
				analyze_required_functions(ctx, value);
				return;
			} else if *op == Op::Hash {
				if matches!(key.drop_meta(), Node::Empty) {
					ctx.required_functions.insert("node_count");
				} else {
					ctx.required_functions.insert("node_index_at");
					ctx.required_functions.insert("map_get");
					if let Some(name) = crate::wasp_parser::subscript_key(value).and_then(constant_field_name) {
						ctx.missing_field_names.insert(name);
					}
					ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
					ctx.required_functions.insert("string_char_at");
					ctx.required_functions.insert("list_node_at");
					ctx.required_functions.insert("list_at");
				}
			} else if *op == Op::Dot {
				let method_name = match value.drop_meta() {
					Node::Symbol(s) => Some(s.clone()),
					Node::List(items, _, _) if items.len() == 1 => {
						if let Node::Symbol(s) = items[0].drop_meta() {
							Some(s.clone())
						} else {
							None
						}
					}
					_ => None,
				};
				if let Some(counter) = method_name.and_then(|method| counting_method(&method, ctx)) {
					require_counter(ctx, counter);
					return;
				}
			}
			analyze_required_functions(ctx, key);
			analyze_required_functions(ctx, value);
		}
		Node::List(items, _, _) => {
			if items.is_empty() {
				return;
			}
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if fn_name == ZERO_FILL_CALL {
					ctx.required_functions.insert(ZERO_FILL_CALL);
				}
				if fn_name == INSERT_AT_CALL || fn_name == INSERT_EITHER_CALL {
					ctx.required_functions.insert(INSERT_AT_CALL);
				}
				if fn_name == crate::type_tests::IS_TYPE {
					ctx.required_functions.insert(crate::type_tests::NODE_KIND_IN);
				}
				if fn_name == crate::switch::NO_CASE_CALL {
					ctx.missing_case_labels.extend(items.get(1).map(|label| label.name()));
				}
				if fn_name == crate::wasm_emitter::text_builtins::TEXT_FORM {
					ctx.required_functions.insert("list_join");
				}
				if fn_name == crate::library_words::FIELD_WITH {
					ctx.required_functions.extend([crate::library_words::FIELD_WITH, crate::wasm_emitter::VALUES_EQUAL]);
				}
				if ctx.ffi_imports.contains_key(fn_name.as_str()) {
					for item in items.iter().skip(1) {
						analyze_required_functions(ctx, item);
					}
					return;
				}
				if items.len() == 2 {
					if let Some(counter) = counting_function(fn_name, ctx) {
						require_counter(ctx, counter);
						return;
					}
				}
			}
			for item in items {
				analyze_required_functions(ctx, item);
			}
		}
		Node::Data(_) => {
			ctx.required_functions.insert("new_data");
		}
		Node::Meta { node, .. } => {
			analyze_required_functions(ctx, node);
		}
		Node::Type { name, body } => {
			ctx.required_functions.insert("new_type");
			ctx.type_registry.register_from_node(node);
			analyze_required_functions(ctx, name);
			analyze_required_functions(ctx, body);
		}
		Node::Error(inner) => {
			analyze_required_functions(ctx, inner);
		}
	}
}

/// The callee of `name(args)`: a call is a symbol applied with round brackets and no space before the arguments.
/// `(name args)`, `(name, args)` and `[name args]` are data.
pub fn call_name<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator) -> Option<&'a str> {
	match (items, bracket, separator) {
		([head, _, ..], Bracket::Round, Separator::None) => match head.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

/// Recursively collect all type definitions from the AST into the TypeRegistry
/// This pre-scan enables forward references (use a type before defining it)
pub fn collect_all_types(registry: &mut crate::type_kinds::TypeRegistry, node: &Node) {
	match node.drop_meta() {
		Node::Type { .. } => {
			registry.register_from_node(node);
		}
		Node::Key(l, _, r) => {
			collect_all_types(registry, l);
			collect_all_types(registry, r);
		}
		Node::List(items, _, _) => {
			for item in items {
				collect_all_types(registry, item);
			}
		}
		Node::Meta { node, .. } => collect_all_types(registry, node),
		_ => {}
	}
}

/// Extract FFI imports from "import X from Y" and "use Y" statements, and the libm functions called without one
pub fn extract_ffi_imports(ctx: &mut Context, node: &Node) {
	extract_declared_ffi_imports(ctx, node);
	add_implicit_libm_imports(ctx, node);
}

/// libm functions the emitter does not implement itself (rounding and √ are builtins): a call of one the program neither
/// imports nor defines links it from libm, as `import f from 'm'` would; without that it compiled to its argument
fn add_implicit_libm_imports(ctx: &mut Context, node: &Node) {
	let is_builtin = |name: &str| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name) || name == "sqrt";
	let mut implicit: Vec<&str> = crate::ffi::LIBM_F64_FUNCTIONS.iter().map(|(name, _)| *name).filter(|name| !is_builtin(name)).collect();
	implicit.push(LIBM_LN); // ffi.rs signs it as libm's log
	let mut called = HashSet::new();
	node.visit(&mut |part| {
		if let Node::List(items, bracket, separator) = part {
			if let Some(name) = call_name(items, bracket, separator).filter(|name| implicit.contains(name)) {
				called.insert(name.to_string());
			}
		}
	});
	called.retain(|name| !ctx.ffi_imports.contains_key(name));
	if called.is_empty() {
		return;
	}
	let mut defined = Context::new();
	extract_user_functions(&mut defined, node);
	for name in called.iter().filter(|name| !defined.user_functions.contains_key(*name)) {
		add_ffi_import(ctx, name, "m");
	}
}

/// The natural logarithm under its usual name, libm's log
const LIBM_LN: &str = "ln";

fn extract_declared_ffi_imports(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::List(items, _, _) => {
			if !items.is_empty() {
				match items[0].drop_meta() {
					Node::Symbol(first_sym) => {
						if first_sym == "import" && items.len() >= 2 {
							if items.len() == 2 {
								let lib = items[1].name();
								add_ffi_lib(ctx, &lib);
								return;
							}
							let func_names = imported_names(&items[1]);
							if items.len() >= 3 {
								if let Node::Key(ref key, _, ref value) = items[2].drop_meta() {
									if key.name() == "from" {
										let lib = value.name();
										func_names.iter().for_each(|func_name| add_ffi_import(ctx, func_name, &lib));
										return;
									}
								}
							}
							if items.len() >= 4 && items[2].name() == "from" {
								let lib = items[3].name();
								func_names.iter().for_each(|func_name| add_ffi_import(ctx, func_name, &lib));
								return;
							}
						} else if first_sym == "use" && items.len() >= 2 {
							let lib = items[1].name();
							add_ffi_lib(ctx, &lib);
							return;
						}
					}
					Node::List(inner_items, _, _) if inner_items.len() >= 2 => {
						if let Node::Symbol(inner_first) = inner_items[0].drop_meta() {
							if inner_first == "use" {
								let lib = inner_items[1].name();
								add_ffi_lib(ctx, &lib);
							}
						}
					}
					_ => {}
				}
			}
			for item in items {
				extract_declared_ffi_imports(ctx, item);
			}
		}
		Node::Key(ref key, _, ref value) => {
			if key.name() == "import" {
				if let Node::Key(ref from_key, _, ref lib) = value.drop_meta() {
					if from_key.name() == "from" {
						let func_name = key.name();
						let lib_name = lib.name();
						add_ffi_import(ctx, &func_name, &lib_name);
						return;
					}
				}
			}
			extract_declared_ffi_imports(ctx, key);
			extract_declared_ffi_imports(ctx, value);
		}
		Node::Meta { ref node, .. } => {
			extract_declared_ffi_imports(ctx, node);
		}
		_ => {}
	}
}

/// Calls of the host words (`sleep(ms)`, `random()` …) import them, unless the program defines a function of that name
pub fn extract_host_words(ctx: &mut Context, node: &Node) {
	match node.drop_meta() {
		Node::List(items, _, _) => {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				if crate::host::HOST_WORDS.contains(&name.as_str()) && !ctx.user_functions.contains_key(name) {
					add_ffi_import(ctx, name, crate::host::HOST_LIBRARY);
					// the host builds a caught stack overflow's Error with the module's own error_of
					if name == crate::host::GUARDED_CALL {
						ctx.required_functions.insert(crate::wasm_emitter::text_builtins::ERROR_OF);
					}
					// a program that controls tasks polls at its loops, where a paused task waits (browser)
					if name == crate::host::TASK_CONTROL {
						add_ffi_import(ctx, crate::host::TASK_POLL, crate::host::HOST_LIBRARY);
					}
				}
			}
			items.iter().for_each(|item| extract_host_words(ctx, item));
		}
		Node::Key(left, _, right) => {
			extract_host_words(ctx, left);
			extract_host_words(ctx, right);
		}
		_ => {}
	}
}

/// The function names of `import sin from 'm'` and of the group `import (sin, floor, fabs) from 'm'`
fn imported_names(names: &Node) -> Vec<String> {
	match names.drop_meta() {
		Node::List(items, _, _) => items.iter().map(Node::name).collect(),
		single => vec![single.name()],
	}
}

/// Add an FFI import by function name
fn add_ffi_import(ctx: &mut Context, name: &str, library: &str) {
	use crate::ffi::{get_ffi_signature, get_ffi_signature_from_lib};

	let sig = get_ffi_signature_from_lib(name, library)
		.or_else(|| get_ffi_signature(name));

	if let Some(sig) = sig {
		ctx.ffi_imports.insert(name.to_string(), sig);
	}
}

/// Add all common functions from a library
fn add_ffi_lib(ctx: &mut Context, lib: &str) {
	let lib_alias = crate::ffi::resolve_library_alias(lib);
	if lib_alias == "m" {
		for (name, _) in crate::ffi::LIBM_F64_FUNCTIONS {
			add_ffi_import(ctx, name, "m");
		}
	} else if lib_alias == "c" {
		for name in ["strlen", "atoi", "atol", "atof", "strcmp", "strncmp", "rand"] {
			add_ffi_import(ctx, name, "c");
		}
	} else {
		add_ffi_lib_dynamic(ctx, lib);
	}
}

/// Dynamically discover and add all functions from a library via header parsing
fn add_ffi_lib_dynamic(ctx: &mut Context, lib: &str) {
	use crate::ffi::get_signatures_from_headers;

	let signatures = get_signatures_from_headers(lib);
	if signatures.is_empty() {
		// the program is analysed several times (effects, emission), the library is reported once
		static WARNED_LIBRARIES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
		let mut warned = WARNED_LIBRARIES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		if !warned.iter().any(|known| known == lib) {
			eprintln!("[FFI] Warning: No functions found for library '{}'", lib);
			warned.push(lib.to_string());
		}
		return;
	}

	for (name, sig) in signatures {
		ctx.ffi_imports.insert(name.clone(), sig.clone());
	}
}
