//! List node emission - handles all List(items, bracket, separator) patterns

use crate::analyzer::{call_name, is_unbracketed_block, type_word_kind};
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::normalize::hints as norm;
use wasm_encoder::*;

use super::{WasmGcEmitter, ROUNDING_FUNCTIONS};

/// Names the emitter resolves itself, besides user functions, imports, type words and counting functions
const BUILTIN_CALLS: [&str; 15] = [
	"return", "fetch", "puts", "puti", "putl", "putf", "fd_write", "range", "type", "use",
	crate::min_max::EMPTY_EXTREMUM_CALL, crate::switch::NO_CASE_CALL, crate::analyzer::ZERO_FILL_CALL, crate::analyzer::INSERT_AT_CALL,
	crate::analyzer::INSERT_EITHER_CALL,
];

const PRINT: &str = "print";

impl WasmGcEmitter {
	/// Does `name(args)` resolve to something callable: user function, import, builtin, type word or declared type?
	/// A variable is not callable.
	pub(super) fn resolves_call(&self, name: &str) -> bool {
		self.ctx.user_functions.contains_key(name)
			|| self.ctx.ffi_imports.contains_key(name)
			|| self.ctx.type_registry.get_by_name(name).is_some()
			|| type_word_kind(&name.to_lowercase()).is_some()
			|| is_function_keyword(name)
			|| name == PRINT
			|| BUILTIN_CALLS.contains(&name)
			|| crate::library_words::is_runtime_word(name)
			|| name == crate::type_tests::IS_TYPE
			|| ROUNDING_FUNCTIONS.contains(&name)
			|| crate::analyzer::counting_function(name, &self.ctx).is_some()
			|| super::text_builtins::is_text_builtin(name)
			|| crate::ffi::get_ffi_signature(name).is_some()
	}

	/// `reverse(xs)`, `split(text, separator)` …: the library words with a runtime function; returns whether it was one
	fn emit_library_word_call(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let Some(name) = call_name(items, bracket, separator) else { return false };
		let Some((_, function)) = super::library_ops::LIBRARY_FUNCTIONS.iter().find(|(word, _)| *word == name) else { return false };
		for argument in &items[1..] {
			self.emit_node_instructions(func, argument);
		}
		self.emit_call(func, function);
		true
	}

	/// A call nothing resolves is an error value at the call; returns whether it was one
	pub(super) fn reject_unresolved_call(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let Some(name) = call_name(items, bracket, separator) else {
			return false;
		};
		if self.resolves_call(name) {
			return false;
		}
		let diagnostic = crate::diagnostic::Diagnostic::at(&items[0], format!("undefined function: {name}"));
		self.emit_type_error(func, diagnostic.to_string());
		true
	}

	/// Emit instructions for List(items, bracket, separator) nodes
	/// Dispatches based on list contents and bracket type
	pub(super) fn emit_list_node(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) {
		if items.is_empty() {
			self.emit_call(func, "new_empty");
			return;
		}

		if items.len() == 1 && *bracket != Bracket::Square {
			// Check for zero-argument function call: (funcname)
			if *bracket == Bracket::Round {
				if let Node::Symbol(fn_name) = items[0].drop_meta() {
					if self.ctx.user_functions.contains_key(fn_name) {
						self.emit_user_function_call(func, fn_name, &[]);
						return;
					}
				}
			}
			self.emit_node_instructions(func, &items[0]);
			return;
		}

		// Check for fetch call: [Symbol("fetch"), url_node], optionally `… timeout SECONDS`
		if self.config.emit_host_imports {
			if let Some((url, timeout)) = crate::host::fetch_call(&Node::List(items.to_vec(), Bracket::None, Separator::Space)) {
				self.emit_fetch_call(func, &url, timeout);
				return;
			}
		}

		// Check for return statement: return value
		if items.len() == 2 {
			if let Node::Symbol(s) = items[0].drop_meta() {
				if s == "return" {
					// Emit the return value and return instruction
					self.emit_node_instructions(func, &items[1]);
					func.instruction(&Instruction::Return);
					// Unreachable after return, push dummy value
					func.instruction(&Instruction::Unreachable);
					return;
				}
			}
		}

		if items.len() == 2 && self.config.emit_wasi_imports && matches!(items[0].drop_meta(), Node::Symbol(name) if name == PRINT) {
			self.emit_print(func, &items[1]);
			return;
		}

		// WASI calls: puts, puti, putl, putf, fd_write — their i64 result is boxed like any value
		if items.len() >= 2 && self.config.emit_wasi_imports {
			if let Node::Symbol(s) = items[0].drop_meta() {
				let emitted = match s.as_str() {
					"puts" => {
						self.emit_wasi_puts(func, &items[1]);
						func.instruction(&Instruction::I64ExtendI32S);
						true
					}
					"puti" | "putl" => {
						self.emit_wasi_puti(func, &items[1]);
						self.emit_numeric_value(func, &items[1]);
						true
					}
					"putf" => {
						self.emit_wasi_putf(func, &items[1]);
						func.instruction(&Instruction::I64ExtendI32S);
						true
					}
					"fd_write" if items.len() >= 5 => {
						self.emit_wasi_fd_write_call(func, &items[1..]);
						true
					}
					_ => false,
				};
				if emitted {
					self.emit_call(func, "new_int");
					return;
				}
			}
		}

		// Check for introspection and math functions
		if items.len() == 2 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.emit_introspection_fn(func, fn_name, &items[1]) {
					return;
				}
			}
		}

		// Check for type constructor calls: int('123'), str(123), char(0x41), etc.
		if items.len() == 2 {
			if let Node::Symbol(type_name) = items[0].drop_meta() {
				let is_typed_decl = matches!(items[1].drop_meta(), Node::Key(_, Op::Assign | Op::Define, _));
				if !is_typed_decl {
					if type_word_kind(&type_name.to_lowercase()).is_some() {
						self.emit_cast(func, &items[1], &items[0]);
						return;
					}
				}
			}
		}

		// Check for range function: range start end
		if items.len() == 3 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if fn_name == "range" {
					self.emit_range(func, &items[1], &items[2], true);
					return;
				}
			}
		}

		// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
		if items.len() >= 2 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.ctx.user_functions.contains_key(fn_name) {
					self.emit_user_function_call(func, fn_name, &items[1..]);
					return;
				}
				// Check for FFI function call
				if self.ctx.ffi_imports.contains_key(fn_name) {
					self.emit_ffi_call(func, fn_name, &items[1..], None);
					return;
				}
				if super::text_builtins::text_builtin_kind(fn_name, items.len() - 1).is_some() {
					self.emit_text_builtin(func, fn_name, &items[1..]);
					return;
				}
			}
		}

		if self.emit_library_word_call(func, items, bracket, separator) || self.emit_type_test(func, items, bracket, separator) {
			return;
		}

		if let [Node::Symbol(call), count, zero] = items {
			if call == crate::analyzer::ZERO_FILL_CALL {
				self.emit_numeric_value(func, count);
				self.emit_node_instructions(func, zero);
				self.emit_call(func, crate::analyzer::ZERO_FILL_CALL);
				return;
			}
		}
		if let [Node::Symbol(call), list, first, second] = items {
			if call == crate::analyzer::INSERT_AT_CALL || call == crate::analyzer::INSERT_EITHER_CALL {
				let Some((position, value)) = self.insert_position_and_value(func, call, first, second) else { return };
				self.emit_node_instructions(func, list);
				self.emit_numeric_value(func, position);
				self.emit_node_instructions(func, value);
				self.emit_call(func, crate::analyzer::INSERT_AT_CALL);
				return;
			}
		}

		if self.reject_unresolved_call(func, items, bracket, separator) {
			return;
		}

		if items.iter().any(|item| matches!(item.drop_meta(), Node::Type { .. })) {
			self.emit_without_type_definitions(func, items, bracket, separator);
			return;
		}

		// Check if this is a statement sequence
		let is_statement_sequence = is_unbracketed_block(items, bracket, separator) || self.is_statement_sequence(items);

		if is_statement_sequence {
			self.emit_statement_sequence(func, items, Self::emit_node_instructions);
		} else {
			// Check for pure numeric expressions; a square list keeps all its items, whatever they compute
			let has_arithmetic = *bracket != Bracket::Square && items.iter().any(|item| {
				matches!(item.drop_meta(), Node::Key(_, op, _) if op.is_arithmetic())
			});
			if has_arithmetic {
				let node = Node::List(items.to_vec(), bracket.clone(), Separator::None);
				if self.get_type(&node).is_float() {
					self.emit_float_value(func, &node);
					self.emit_call(func, "new_float");
				} else {
					self.emit_numeric_value(func, &node);
					self.emit_call(func, "new_int");
				}
			} else {
				// Build linked list: (first, rest, bracket_info)
				self.emit_list_structure(func, items, bracket);
			}
		}
	}

	/// The name `type(x)` reports: `int`, `rational`, `text`, `list of int` …
	fn static_type_name(&self, arg: &Node) -> String {
		let kind = match arg.drop_meta() {
			literal @ Node::Number(_) => literal.kind(),
			_ => self.get_type(arg),
		};
		match (kind, crate::analyzer::literal_number_type_word(arg)) {
			(_, Some(number_word)) => number_word.to_string(),
			(crate::type_kinds::Kind::List, _) => crate::analyzer::list_type_name(arg, &self.scope),
			_ => kind.to_string(),
		}
	}

	/// `is_type(x, "spec")`: 1 when the static type of x is the spec, else 0
	fn emit_type_test(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		if call_name(items, bracket, separator) != Some(crate::type_tests::IS_TYPE) {
			return false;
		}
		let [_, subject, spec] = items else { return false };
		let Node::Text(spec) = spec.drop_meta() else { return false };
		let answer = crate::type_tests::type_matches(&self.static_type_name(subject), spec);
		func.instruction(&Instruction::I64Const(answer as i64));
		self.emit_call(func, "new_int");
		true
	}

	/// Emit introspection functions: type, count, length, size, ceil, floor, round
	/// Returns true if the function was handled
	pub(super) fn emit_introspection_fn(&mut self, func: &mut Function, fn_name: &str, arg: &Node) -> bool {
		if let Some(counter) = crate::analyzer::counting_function(fn_name, &self.ctx) {
			// count, length, size: elements, or graphemes of a text
			self.emit_node_instructions(func, arg);
			self.emit_call(func, counter);
			self.emit_call(func, "new_int");
			return true;
		}
		match fn_name {
			crate::switch::NO_CASE_CALL => {
				let Node::Text(label) = arg.drop_meta() else { return false };
				let error: &'static str = Box::leak(format!("{}{label}", crate::switch::NO_CASE_PREFIX).into_boxed_str());
				self.emit_runtime_error(func, error);
				true
			}
			crate::min_max::EMPTY_EXTREMUM_CALL => {
				let Node::Symbol(extremum) = arg.drop_meta() else { return false };
				let Some((_, error)) = crate::min_max::EMPTY_LIST_ERRORS.iter().find(|(name, _)| name == extremum) else { return false };
				self.emit_runtime_error(func, error);
				true
			}
			"type" => {
				let type_name = self.static_type_name(arg);
				let (ptr, len) = self.allocate_string(&type_name);
				func.instruction(&Instruction::I32Const(ptr as i32));
				func.instruction(&Instruction::I32Const(len as i32));
				self.emit_call(func, "new_symbol");
				true
			}
			"ceil" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, Instruction::F64Ceil);
				true
			}
			"floor" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, Instruction::F64Floor);
				true
			}
			// round half up (JS Math.round, Excel for x ≥ 0): floor(x) + (x - floor(x) ≥ ½), x kept as bits in a scratch local
			"round_half_up" => {
				let bits = self.scratch(0);
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::I64ReinterpretF64);
				func.instruction(&Instruction::LocalTee(bits));
				func.instruction(&Instruction::F64ReinterpretI64);
				func.instruction(&Instruction::F64Floor);
				func.instruction(&Instruction::LocalGet(bits));
				func.instruction(&Instruction::F64ReinterpretI64);
				func.instruction(&Instruction::LocalGet(bits));
				func.instruction(&Instruction::F64ReinterpretI64);
				func.instruction(&Instruction::F64Floor);
				func.instruction(&Instruction::F64Sub);
				func.instruction(&Instruction::F64Const(0.5f64.into()));
				func.instruction(&Instruction::F64Ge);
				func.instruction(&Instruction::F64ConvertI32U);
				func.instruction(&Instruction::F64Add);
				self.emit_integral_float_as_int(func);
				true
			}
			// round = round half even (IEEE 754 default, Python 3, .NET): 2.5 → 2, 3.5 → 4
			"round_half_even" => {
				self.emit_rounded_as_int(func, arg, Instruction::F64Nearest);
				true
			}
			"round" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, Instruction::F64Nearest);
				true
			}
			_ => false,
		}
	}

	/// `arg` as f64, rounded by `rounding`, as an exact Int node
	fn emit_rounded_as_int(&mut self, func: &mut Function, arg: &Node, rounding: Instruction) {
		self.emit_float_value(func, arg);
		func.instruction(&rounding);
		self.emit_integral_float_as_int(func);
	}

	/// An integral f64 on the stack → exact Int node
	fn emit_integral_float_as_int(&mut self, func: &mut Function) {
		self.emit_truncating_cast(func);
		self.emit_call(func, "new_int");
	}

	/// Check if items form a statement sequence
	fn is_statement_sequence(&self, items: &[Node]) -> bool {
		items.iter().any(|item| {
			let item = item.drop_meta();
			match item {
				Node::Key(_, Op::Assign | Op::Define, _) => true,
				Node::Key(_, Op::Hash, _) => true,
				Node::Key(_, op, _) if op.is_compound_assign() => true,
				// control flow: `if c {…}`, `while c {…}`, `i++`
				Node::Key(_, Op::Then | Op::Else | Op::Do | Op::Inc | Op::Dec, _) => true,
				Node::Key(left, Op::Colon, _) => {
					matches!(left.drop_meta(), Node::Symbol(s) if s == "global")
				}
				Node::List(list_items, _, _) if list_items.len() >= 2 => {
					if let Node::Symbol(s) = list_items[0].drop_meta() {
						is_function_keyword(s) || s == "use" || s == "import" || s == "return"
					} else {
						false
					}
				}
				_ => false,
			}
		})
	}

	/// A type definition declares and runs nothing (types are registered beforehand): emit the other items
	fn emit_without_type_definitions(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) {
		let rest: Vec<Node> = items.iter().filter(|item| !matches!(item.drop_meta(), Node::Type { .. })).cloned().collect();
		match rest.as_slice() {
			[] => self.emit_call(func, "new_empty"),
			[only] => self.emit_node_instructions(func, only),
			_ => self.emit_list_node(func, &rest, bracket, separator),
		}
	}

	/// Emit a statement sequence: execute in order, keep the last value.
	/// Function definitions leave no value; they snapshot the variables they capture.
	pub(super) fn emit_statement_sequence(&mut self, func: &mut Function, items: &[Node], emit: fn(&mut Self, &mut Function, &Node)) {
		let last_statement = items.iter().rposition(|item| !self.is_definition(item));
		// `a;b;c` is data; among code (`x=1; foo; x`) a word that names nothing would be silently dropped
		let is_code = items.iter().any(|item| !matches!(item.drop_meta(), Node::Symbol(_) | Node::Number(_) | Node::Text(_) | Node::Char(_)));
		for (i, item) in items.iter().enumerate() {
			if self.is_definition(item) {
				if let Some(name) = self.defined_function_name(item) {
					self.emit_closure_capture(func, &name);
				}
				continue;
			}
			if Some(i) == last_statement {
				emit(self, func, item);
			} else if let Some(name) = self.unknown_word(item).filter(|_| is_code) {
				self.emit_undefined_variable(func, &name);
			} else {
				self.emit_discarded_statement(func, item, emit);
				func.instruction(&Instruction::Drop);
			}
		}
	}

	/// The (position, value) of an insert: given by `at:`, else the one Int among the two arguments is the position;
	/// two Ints are ambiguous (Python `insert(i, x)` vs wasp `insert(x, i)`) and a type error naming both readings
	fn insert_position_and_value<'a>(&mut self, func: &mut Function, call: &str, first: &'a Node, second: &'a Node) -> Option<(&'a Node, &'a Node)> {
		if call == crate::analyzer::INSERT_AT_CALL {
			return Some((first, second));
		}
		let is_int = |kind: crate::type_kinds::Kind| kind == crate::type_kinds::Kind::Int;
		match (is_int(self.get_type(first)), is_int(self.get_type(second))) {
			(true, false) => Some((first, second)),
			(false, true) => Some((second, first)),
			_ => {
				let (a, b) = (crate::normalize::operand_text(first), crate::normalize::operand_text(second));
				self.emit_type_error(func, format!("ambiguous: insert({a}, {b}) inserts {b} at {a} in Python, {a} at {b} in wasp; write insert({b}, at: {a}) or insert({a}, at: {b})"));
				None
			}
		}
	}

	/// A bare word that is no variable, global or function
	fn unknown_word(&self, item: &Node) -> Option<String> {
		let Node::Symbol(name) = item.drop_meta() else { return None };
		let known = self.scope.lookup(name).is_some() || self.ctx.user_globals.contains_key(name) || self.ctx.user_functions.contains_key(name);
		(!known).then(|| name.clone())
	}

	/// A statement whose value is dropped keeps its own representation instead of being forced into an exact Int:
	/// an assignment to a float variable its f64, a text or list update (`s += "a"` in a loop body) its Node
	pub(super) fn emit_discarded_statement(&mut self, func: &mut Function, item: &Node, emit: fn(&mut Self, &mut Function, &Node)) {
		if self.is_float_assignment(item) {
			self.emit_float_value(func, item);
		} else if self.is_ref_update(item) {
			self.emit_node_instructions(func, item);
		} else {
			emit(self, func, item);
		}
	}

	/// `s += "a"`, `xs = xs + [1]`, and an `if` whose branch does such an update
	fn is_ref_update(&self, item: &Node) -> bool {
		match item.drop_meta() {
			Node::Key(left, op, _) if *op == Op::Assign || op.is_compound_assign() => {
				matches!(left.drop_meta(), Node::Symbol(name) if self.scope.lookup(name).is_some_and(|local| local.kind.is_ref()))
			}
			Node::Key(_, Op::Then | Op::Else, _) => self.get_type(item).is_ref(),
			_ => false,
		}
	}

	fn is_float_assignment(&self, item: &Node) -> bool {
		match item.drop_meta() {
			Node::Key(left, Op::Define | Op::Assign, _) => {
				matches!(left.drop_meta(), Node::Symbol(name) if self.scope.lookup(name).is_some_and(|local| local.kind.is_float()))
			}
			_ => false,
		}
	}

	/// Function definitions and imports: compiled ahead, no runtime value
	fn is_definition(&self, item: &Node) -> bool {
		self.defined_function_name(item).is_some() || match item.drop_meta() {
			Node::List(list_items, _, _) if list_items.len() >= 2 => {
				matches!(list_items[0].drop_meta(), Node::Symbol(s) if is_function_keyword(s) || s == "use" || s == "import")
			}
			_ => false,
		}
	}

	/// Name of the user function this item defines: `f := …`, `f(x…) := …`, `f(x…) = …`, `def f(x…): …`
	pub(super) fn defined_function_name(&self, item: &Node) -> Option<String> {
		let name = match item.drop_meta() {
			Node::Key(left, op @ (Op::Define | Op::Assign), _) => match left.drop_meta() {
				Node::Symbol(name) if *op == Op::Define => Some(name.clone()),
				Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
					Some(Node::Symbol(name)) => Some(name.clone()),
					_ => None,
				},
				_ => None,
			},
			Node::List(items, _, _) if items.len() >= 2 => match items[0].drop_meta() {
				Node::Symbol(keyword) if is_function_keyword(keyword) => {
					crate::analyzer::extract_def_function(&items[1..]).map(|function| function.name)
				}
				_ => None,
			},
			_ => None,
		};
		name.filter(|name| self.ctx.user_functions.contains_key(name))
	}
}
