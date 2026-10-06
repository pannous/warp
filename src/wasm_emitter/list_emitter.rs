//! List node emission - handles all List(items, bracket, separator) patterns

use crate::analyzer::{call_name, is_statement, is_unbracketed_block, type_word_kind};
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use wasm_encoder::*;
use Instruction as I;

use super::{WasmGcEmitter, ROUNDING_FUNCTIONS};

/// Names the emitter resolves itself, besides user functions, imports, type words and counting functions
const BUILTIN_CALLS: [&str; 17] = [
	"return", "fetch", "puts", "puti", "putl", "putf", "fd_write", "range", "type", "use",
	crate::min_max::EMPTY_EXTREMUM_CALL, crate::switch::NO_CASE_CALL, crate::analyzer::ZERO_FILL_CALL, crate::analyzer::INSERT_AT_CALL,
	crate::analyzer::INSERT_EITHER_CALL, crate::library_words::LIST_SUM, crate::traits::INSTANCE_OF,
];

const PRINT: &str = "print";
/// `print a, b` writes "a b", like Python
const PRINT_ARGUMENT_SEPARATOR: &str = " ";

/// The value `print` writes: its one argument, or several joined by a space
fn printed_value(call: &[Node], bracket: &Bracket) -> Node {
	match crate::wasp_parser::print_arguments_of(call, bracket).as_slice() {
		[single] => juxtaposed_text(single).unwrap_or_else(|| single.clone()),
		several => super::joined_text(several, PRINT_ARGUMENT_SEPARATOR),
	}
}

/// `print "x changed to " value` (wiki/signal.md): a text followed by values prints them joined, without separator
fn juxtaposed_text(argument: &Node) -> Option<Node> {
	match argument.drop_meta() {
		Node::List(parts, Bracket::None, Separator::Space) if matches!(parts.first().map(Node::drop_meta), Some(Node::Text(_))) => {
			Some(super::joined_text(parts, ""))
		}
		_ => None,
	}
}
/// The Ask topic of `xs.insert(i, x)` with two Ints: answers are remembered per topic
const INSERT_ORDER_TOPIC: &str = "insert-order";
/// The got-it topic of an unknown word applied to a value (`cube 3`)
/// `data cube 3`: the rest is data (P62, P63: `data` is the word for what never runs)
const QUOTE_WORDS: [&str; 3] = ["data", "code", "block"];
/// The got-it topic of a lone word close to a defined name
const NEAR_MISS_TOPIC: &str = "near-miss";
/// Words the near-miss warning compares with besides the program's own names
const KNOWN_WORDS: [&str; 6] = ["print", "count", "sum", "first", "last", "reverse"];

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
			|| name == crate::wasp_parser::TEXT_TIMES
			|| BUILTIN_CALLS.contains(&name)
			|| super::cells::CELL_WORDS.contains(&name)
			|| crate::library_words::is_runtime_word(name)
			|| name == crate::type_tests::IS_TYPE
			|| ROUNDING_FUNCTIONS.contains(&name)
			|| crate::analyzer::counting_function(name, &self.ctx).is_some()
			|| super::text_builtins::is_text_builtin(name)
			// a C function resolves once imported (`use c`, `import f from "c"`, libm's implicit imports): known but not
			// imported it is an error that says so (ffi::undefined_function_message), never data
			|| crate::ffi::get_ffi_signature(name).is_some_and(|signature| signature.library != "c")
	}

	/// `reverse(xs)`, `split(text, separator)` …: the library words with a runtime function; returns whether it was one
	fn emit_library_word_call(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let Some(name) = call_name(items, bracket, separator) else { return false };
		let Some((_, function)) = super::library_ops::LIBRARY_FUNCTIONS.iter().find(|(word, _)| *word == name) else { return false };
		if name == crate::library_words::SLICE {
			for bound in items.iter().skip(2) {
				self.emit_integral_index_check(func, bound); // `a[0:n/2]` like `a[n/2]`: an integer, with the `n//2` hint
			}
		}
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
		// `inc()` of a bound name reads it (`inc:={x+1}; inc()`); with arguments a variable is no function (`x=2; x(3)`)
		if self.resolves_call(name) || (items.len() == 1 && !self.is_unbound(name)) {
			return false;
		}
		let call = Node::List(items.to_vec(), bracket.clone(), separator.clone());
		let diagnostic = crate::diagnostic::Diagnostic::at(&call, crate::ffi::undefined_function_message(name));
		self.emit_type_error(func, diagnostic.to_string());
		true
	}

	/// `f(a, b…)` or `(f)` of a user function, an FFI import or a text builtin
	fn emit_function_call(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) -> bool {
		let Some(Node::Symbol(fn_name)) = items.first().map(Node::drop_meta) else { return false };
		if items.len() < 2 && *bracket != Bracket::Round {
			return false;
		}
		if self.ctx.user_functions.contains_key(fn_name) {
			self.emit_user_function_call(func, fn_name, &items[1..]);
		} else if self.ctx.ffi_imports.contains_key(fn_name) {
			self.emit_ffi_call(func, fn_name, &items[1..], None);
		} else if items.len() >= 2 && super::text_builtins::text_builtin_kind(fn_name, items.len() - 1).is_some() {
			self.emit_text_builtin(func, fn_name, &items[1..]);
		} else {
			return false;
		}
		true
	}

	/// `return value`: leaves the open tries and the function
	fn emit_return_statement(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let [word, value] = items else { return false };
		if !matches!(word.drop_meta(), Node::Symbol(s) if s == "return") {
			return false;
		}
		self.emit_returned_value(func, value);
		self.emit_leave_tries(func, 0);
		func.instruction(&I::Return);
		func.instruction(&I::Unreachable); // nothing follows a return, the block type still wants a value
		true
	}

	/// `n times "ab"`: the text repeated; anything but a text is refused
	fn emit_text_times(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let [word, count, repeated] = items else { return false };
		if !matches!(word.drop_meta(), Node::Symbol(name) if name == crate::wasp_parser::TEXT_TIMES) {
			return false;
		}
		match self.get_type(repeated) {
			crate::Kind::Text | crate::Kind::Codepoint => self.emit_text_repeat(func, repeated, count),
			kind => {
				let reason = format!("`n times x` repeats a text (or a list: `n times [x]`), {} is {}", repeated.serialize(), crate::analyzer::kind_with_article(kind));
				self.emit_type_error(func, crate::diagnostic::Diagnostic::at(repeated, reason).to_string());
			}
		}
		true
	}

	/// `print x` and the WASI calls puts, puti, putl, putf, fd_write; their i64 result is boxed like any value
	fn emit_wasi_call(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) -> bool {
		if items.len() < 2 || !self.config.emit_wasi_imports {
			return false;
		}
		let Node::Symbol(word) = items[0].drop_meta() else { return false };
		match word.as_str() {
			PRINT => {
				self.emit_print(func, &printed_value(items, bracket));
				return true;
			}
			"puts" => {
				self.emit_wasi_puts(func, &items[1]);
				func.instruction(&I::I64ExtendI32S);
			}
			"puti" | "putl" => {
				self.emit_wasi_puti(func, &items[1]);
				self.emit_numeric_value(func, &items[1]);
			}
			"putf" => {
				self.emit_wasi_putf(func, &items[1]);
				func.instruction(&I::I64ExtendI32S);
			}
			"fd_write" if items.len() >= 5 => self.emit_wasi_fd_write_call(func, &items[1..]),
			_ => return false,
		}
		self.emit_call(func, "new_int");
		true
	}

	/// `print()`: an empty line, like Python; worth the empty text, as `print ""`
	fn emit_empty_print(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let is_print = matches!(items, [word] if matches!(word.drop_meta(), Node::Symbol(name) if name == PRINT));
		if is_print && self.config.emit_wasi_imports {
			self.emit_print(func, &Node::Text(String::new()));
		}
		is_print && self.config.emit_wasi_imports
	}

	/// Emit instructions for List(items, bracket, separator) nodes
	/// Dispatches based on list contents and bracket type
	pub(super) fn emit_list_node(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) {
		if items.is_empty() || self.emit_task_check(func, items) {
			self.emit_call(func, "new_empty");
			return;
		}
		if self.emit_quoted(func, items, bracket, separator) {
			return;
		}

		if items.len() == 1 && *bracket != Bracket::Square {
			// `f()` of a name bound to nothing is an undefined function (P92); the group `(f)` is its item
			if !self.emit_function_call(func, items, bracket) && !self.emit_empty_print(func, items) && !self.reject_unresolved_call(func, items, bracket, separator) {
				self.emit_node_instructions(func, &items[0]);
			}
			return;
		}

		// a lone import or use statement (the whole program) is worth ø
		if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == "import" || word == "use") && items.len() >= 2 {
			self.emit_call(func, "new_empty");
			return;
		}

		// Check for fetch call: [Symbol("fetch"), url_node], optionally `… timeout SECONDS`
		if self.config.emit_host_imports {
			if let Some((url, timeout)) = crate::host::fetch_call(&Node::List(items.to_vec(), Bracket::None, Separator::Space)) {
				self.emit_fetch_call(func, &url, timeout);
				return;
			}
		}

		if self.emit_return_statement(func, items) || self.emit_text_times(func, items) || self.emit_wasi_call(func, items, bracket) {
			return;
		}

		// Check for introspection and math functions; `[count, a]` lists two items, and a variable `count` heads no call
		if items.len() == 2 && !(*bracket == Bracket::Square && *separator == Separator::Colon) {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.is_unbound(fn_name) && self.emit_introspection_fn(func, fn_name, &items[1]) {
					return;
				}
			}
		}

		// Check for type constructor calls: int('123'), str(123), char(0x41), etc.
		if items.len() == 2 {
			if let Node::Symbol(type_name) = items[0].drop_meta() {
				let is_typed_decl = matches!(items[1].drop_meta(), Node::Key(_, Op::Assign | Op::Define, _));
				if !is_typed_decl
					&& type_word_kind(&type_name.to_lowercase()).is_some() {
						self.emit_cast(func, &items[1], &items[0]);
						return;
					}
			}
		}

		if let [word, instance, type_name] = items {
			if let (Node::Symbol(name), Node::Text(type_name)) = (word.drop_meta(), type_name.drop_meta()) {
				if name == crate::traits::INSTANCE_OF {
					self.emit_instance_of(func, instance, type_name);
					self.emit_call(func, "new_int");
					return;
				}
			}
		}
		if let [word, dividend, divisor] = items {
			if matches!(word.drop_meta(), Node::Symbol(name) if name == crate::wasp_parser::FLOOR_QUOTIENT) {
				self.emit_floor_quotient(func, dividend, divisor);
				self.emit_call(func, "new_int");
				return;
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

		if self.emit_function_call(func, items, bracket) {
			return;
		}

		if self.emit_library_word_call(func, items, bracket, separator) || self.emit_type_test(func, items, bracket, separator) {
			return;
		}

		if let Some((list, sum_loop)) = super::list_dispatch::list_sum_call(items) {
			return self.emit_list_sum(func, list, sum_loop, super::list_dispatch::Wanted::Node);
		}
		if self.emit_cell_call(func, items) {
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
		if let Some(message) = self.unknown_word_error(items, bracket, separator) {
			self.emit_type_error(func, message);
			return;
		}

		if items.iter().any(|item| matches!(item.drop_meta(), Node::Type { .. })) {
			self.emit_without_type_definitions(func, items, bracket, separator);
			return;
		}

		// Check if this is a statement sequence
		let is_statement_sequence = is_unbracketed_block(items, bracket, separator) || items.iter().any(|item| is_statement(item, bracket));

		if is_statement_sequence {
			self.emit_statement_sequence(func, items, Self::emit_node_instructions);
		} else {
			// Check for pure numeric expressions; a square list and a comma tuple `(h + 1, 2)` keep all their items,
			// whatever they compute
			let is_tuple = *separator == Separator::Colon && items.len() > 1;
			let has_arithmetic = *bracket != Bracket::Square && !is_tuple && items.iter().any(|item| {
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
			(crate::type_kinds::Kind::List, _) => crate::analyzer::shown_list_type_name(arg, &self.scope),
			_ => kind.to_string(),
		}
	}

	/// `is_type(x, "spec")` as a Node: its 1 or 0
	fn emit_type_test(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let call = Node::List(items.to_vec(), bracket.clone(), separator.clone());
		if !self.emit_type_test_value(func, &call) {
			return false;
		}
		self.emit_call(func, "new_int");
		true
	}

	/// `is_type(x, "spec")` as an i64 1 or 0: from the static type of x when it is known, else from the run-time kind of the
	/// value (node_kind_in) or, for a declared type, the instance's type (instance_of); true when it was one
	pub(super) fn emit_type_test_value(&mut self, func: &mut Function, node: &Node) -> bool {
		let Node::List(items, bracket, separator) = node.drop_meta() else { return false };
		if call_name(items, bracket, separator) != Some(crate::type_tests::IS_TYPE) {
			return false;
		}
		let [_, subject, spec] = items.as_slice() else { return false };
		let Node::Text(spec) = spec.drop_meta() else { return false };
		let unknown = !matches!(subject.drop_meta(), Node::Number(_)) && matches!(self.get_type(subject), crate::Kind::Empty | crate::Kind::Data);
		let declared_type = self.ctx.type_registry.get_by_name(spec).is_some();
		match crate::type_tests::runtime_kinds(spec).filter(|_| unknown && !declared_type) {
			Some(kinds) => {
				let mask = kinds.iter().fold(0i64, |mask, kind| mask | 1 << (*kind as i64));
				self.emit_node_instructions(func, subject);
				func.instruction(&I::RefAsNonNull);
				func.instruction(&I::I64Const(mask));
				self.emit_call(func, crate::type_tests::NODE_KIND_IN);
			}
			// an instance's static kind is no type name: its declared type is read at run time
			None if declared_type => {
				let instance_test = Node::List(vec![Node::Symbol(crate::traits::INSTANCE_OF.to_string()), subject.clone(), Node::Text(spec.clone())], Bracket::Round, Separator::None);
				self.emit_numeric_value(func, &instance_test);
			}
			None => {
				let answer = crate::type_tests::type_matches(&self.static_type_name(subject), spec);
				func.instruction(&I::I64Const(answer as i64));
			}
		}
		true
	}

	/// Emit introspection functions: type, count, length, size, ceil, floor, round
	/// Returns true if the function was handled
	pub(super) fn emit_introspection_fn(&mut self, func: &mut Function, fn_name: &str, arg: &Node) -> bool {
		if let Some(counter) = crate::analyzer::counting_function(fn_name, &self.ctx) {
			// count, length, size: elements, or graphemes of a text
			self.emit_list_count(func, arg, counter);
			self.emit_call(func, "new_int");
			return true;
		}
		match fn_name {
			crate::switch::NO_CASE_CALL => {
				let Node::Key(label, Op::Colon, value) = arg.drop_meta() else { return false };
				let Node::Text(label) = label.drop_meta() else { return false };
				self.emit_trap_detail(func, value);
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
				func.instruction(&I::I32Const(ptr as i32));
				func.instruction(&I::I32Const(len as i32));
				self.emit_call(func, "new_symbol");
				true
			}
			"ceil" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, I::F64Ceil);
				true
			}
			// an exact number floors exactly, also beyond the f64 range
			"floor" if !self.ctx.ffi_imports.contains_key(fn_name) && self.get_type(arg) == crate::Kind::Int => {
				self.emit_numeric_value(func, arg);
				if self.int_runtime() {
					self.emit_call(func, "exact_floor");
				}
				self.emit_call(func, "new_int");
				true
			}
			"floor" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, I::F64Floor);
				true
			}
			// round half up (JS Math.round, Excel for x ≥ 0): floor(x) + (x - floor(x) ≥ ½), x kept as bits in a scratch local
			"round_half_up" => {
				let bits = self.scratch(0);
				self.emit_float_value(func, arg);
				func.instruction(&I::I64ReinterpretF64);
				func.instruction(&I::LocalTee(bits));
				func.instruction(&I::F64ReinterpretI64);
				func.instruction(&I::F64Floor);
				func.instruction(&I::LocalGet(bits));
				func.instruction(&I::F64ReinterpretI64);
				func.instruction(&I::LocalGet(bits));
				func.instruction(&I::F64ReinterpretI64);
				func.instruction(&I::F64Floor);
				func.instruction(&I::F64Sub);
				func.instruction(&I::F64Const(0.5f64.into()));
				func.instruction(&I::F64Ge);
				func.instruction(&I::F64ConvertI32U);
				func.instruction(&I::F64Add);
				self.emit_integral_float_as_int(func);
				true
			}
			// round = round half even (IEEE 754 default, Python 3, .NET): 2.5 → 2, 3.5 → 4
			"round_half_even" => {
				self.emit_rounded_as_int(func, arg, I::F64Nearest);
				true
			}
			"round" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_rounded_as_int(func, arg, I::F64Nearest);
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

	/// `instance_of(x, "T")` as i64: 1 when x is an instance (a Key node) whose type name, its data, is T
	pub(super) fn emit_instance_of(&mut self, func: &mut Function, instance: &Node, type_name: &str) {
		let node_type = self.type_manager.node_type;
		self.emit_node_instructions(func, instance);
		let instance_local = self.node_scratch();
		func.instruction(&I::LocalSet(instance_local));
		self.emit_field(func, instance_local, 0);
		Self::emit_list(func, &[
			I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::I64Const(crate::Kind::Key as i64), I::I64Eq,
			I::If(BlockType::Result(ValType::I64)),
		]);
		self.emit_field(func, instance_local, 1);
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(node_type)));
		let (pointer, length) = self.allocate_string(type_name);
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.emit_call(func, "new_symbol");
		self.emit_call(func, super::VALUES_EQUAL);
		func.instruction(&I::I64ExtendI32U);
		Self::emit_list(func, &[I::Else, I::I64Const(0), I::End]);
	}

	/// `a // b` as i64: the Euclidean quotient, exact for exact operands (exact_euclid_div), else of the f64 quotient
	pub(super) fn emit_floor_quotient(&mut self, func: &mut Function, dividend: &Node, divisor: &Node) {
		let floats = self.get_type(dividend).is_float() || self.get_type(divisor).is_float();
		if !floats {
			self.emit_numeric_value(func, dividend);
			self.emit_numeric_value(func, divisor);
			self.emit_call(func, "exact_euclid_div");
			return;
		}
		let (quotient, divisor_bits) = (self.scratch(0), self.scratch(1));
		self.emit_float_value(func, dividend);
		self.emit_float_value(func, divisor);
		func.instruction(&I::I64ReinterpretF64);
		func.instruction(&I::LocalTee(divisor_bits));
		func.instruction(&I::F64ReinterpretI64);
		func.instruction(&I::F64Div);
		func.instruction(&I::I64ReinterpretF64);
		func.instruction(&I::LocalSet(quotient));
		// a negative divisor rounds the quotient up, a positive one down
		Self::emit_list(func, &[
			I::LocalGet(quotient), I::F64ReinterpretI64, I::F64Ceil,
			I::LocalGet(quotient), I::F64ReinterpretI64, I::F64Floor,
			I::LocalGet(divisor_bits), I::I64Const(0), I::I64LtS, I::Select,
		]);
		self.emit_truncating_cast(func);
	}

	/// An integral f64 on the stack → exact Int node
	fn emit_integral_float_as_int(&mut self, func: &mut Function) {
		self.emit_truncating_cast(func);
		self.emit_call(func, "new_int");
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
			} else if self.emit_loop_jump(func, item) { // `break`, `continue`: a jump, or its own error outside a loop
			} else if let Some(name) = self.unknown_word(item).filter(|_| is_code) {
				self.emit_undefined_variable(func, &name);
			} else {
				self.emit_discarded_statement(func, item, emit);
				func.instruction(&I::Drop);
			}
		}
		// only definitions and imports: the sequence is worth ø
		if last_statement.is_none() {
			emit(self, func, &Node::Empty);
		}
	}

	/// The (position, value) of an insert: given by `at:`, else the one Int among the two arguments is the position;
	/// two Ints are ambiguous (Python `insert(i, x)` vs wasp `insert(x, i)`): the user is asked, unanswered it is an error
	fn insert_position_and_value<'a>(&mut self, func: &mut Function, call: &str, first: &'a Node, second: &'a Node) -> Option<(&'a Node, &'a Node)> {
		use crate::diagnostic::{ask, reading, Ask, Fallback};
		if call == crate::analyzer::INSERT_AT_CALL {
			return Some((first, second));
		}
		let is_int = |kind: crate::type_kinds::Kind| kind == crate::type_kinds::Kind::Int;
		let (a, b) = (crate::normalize::operand_text(first), crate::normalize::operand_text(second));
		match (is_int(self.get_type(first)), is_int(self.get_type(second))) {
			(true, false) => Some((first, second)),
			(false, true) => Some((second, first)),
			(false, false) => {
				self.emit_type_error(func, format!("insert({a}, {b}) needs an integer position: insert(value, at: position)"));
				None
			}
			(true, true) => {
				let question = Ask::new(INSERT_ORDER_TOPIC, format!("does insert({a}, {b}) put {b} at {a} (Python) or {a} at {b} (wasp)?"),
					vec![reading("position first, as Python", &format!("insert({b}, at: {a})")), reading("value first, as wasp", &format!("insert({a}, at: {b})"))],
					Fallback::Error).written(&format!("insert({a}, {b})")).at_node(first);
				match ask(&question) {
					Ok(0) => Some((first, second)),
					Ok(_) => Some((second, first)),
					Err(error) => {
						let message = match error { Node::Error(reason) => reason.drop_meta().name(), other => other.serialize() };
						self.emit_type_error(func, message);
						None
					}
				}
			}
		}
	}

	/// P51/P62 (user): an unknown word next to a value in code (`cube 3`, `(cube 3)`, `[cube 3]`) is an error, never silent
	/// data. Data contexts keep it: a `quote`/`data` prefix and the values of an object literal (`data_context`); all-word
	/// lists (`hello world`) are not judged here
	pub(super) fn unknown_word_error(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<String> {
		let in_code = matches!((bracket, separator), (Bracket::None | Bracket::Round, Separator::Space) | (Bracket::Square, _));
		// an argument: a value or a name the program defines (`cube x`); unknown words alone are symbols (`[red green]`)
		let is_value = |item: &Node| match item.drop_meta() {
			Node::Symbol(name) => self.unknown_word(item).is_none() && !name.is_empty(),
			Node::List(_, Bracket::Curly, _) => false,
			_ => true,
		};
		// only lists of atoms: a list with an operator (`foo x = 3`) names its undefined variable on its own
		// a cast value counts too: `cube 3 as int` parses as `cube (3 as int)`
		let is_atom = |item: &Node| matches!(item.drop_meta(), Node::Symbol(_) | Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(..) | Node::Key(_, Op::As, _));
		if self.data_context || !in_code || !items.iter().any(is_value) || !items.iter().all(is_atom) {
			return None;
		}
		let name = items.iter().find_map(|item| self.unknown_word(item).filter(|name| self.is_undefined_word(name)))?;
		let written = Node::List(items.to_vec(), bracket.clone(), separator.clone());
		let text = crate::diagnostic::written_text(&written);
		let message = format!("undefined: {name} in `{text}`; define {name}, or write `data {text}` for data");
		Some(crate::diagnostic::Diagnostic::at(&written, message).to_string())
	}

	/// A word that names nothing the program or the language knows: no function, type, unit or keyword
	fn is_undefined_word(&self, name: &str) -> bool {
		!self.resolves_call(name) && !crate::units::is_unit(name) && self.ctx.type_registry.get_by_name(name).is_none()
	}

	/// `pirnt`: a lone word that names nothing but is one or two letters away from a defined name; a got-it warning
	pub(super) fn warn_near_miss(&self, word: &str) -> Result<(), Node> {
		use crate::diagnostic::{ask, reading, Ask, Fallback};
		if word.chars().count() < 3 || self.is_known_word(word) {
			return Ok(());
		}
		let allowed = if word.chars().count() < 6 { 1 } else { 2 };
		let mut names: Vec<String> = self.ctx.user_functions.keys().chain(self.ctx.user_globals.keys()).cloned().collect();
		names.extend(self.scope.local_names());
		names.extend(KNOWN_WORDS.iter().map(|word| word.to_string()));
		let Some(near) = names.into_iter().filter(|name| name != word).find(|name| edit_distance(word, name) <= allowed) else { return Ok(()) };
		let question = Ask::new(NEAR_MISS_TOPIC, format!("`{word}` names nothing: a symbol, or did you mean {near}?"),
			vec![reading("the symbol", &format!("data {word}")), reading(&format!("the name {near}"), &near)], Fallback::Warning).written(word);
		let (line, column) = self.source_position.unwrap_or((0, 0));
		let question = question.at(line, column);
		ask(&question).map(|_| ())
	}

	fn is_known_word(&self, word: &str) -> bool {
		self.resolves_call(word) || crate::units::is_unit(word) || crate::analyzer::type_word_kind(word).is_some()
	}

	/// `data cube 3`: the words after the prefix as data (P62); true when it was one
	fn emit_quoted(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let [prefix, rest @ ..] = items else { return false };
		if *bracket != Bracket::None || *separator != Separator::Space || rest.is_empty() || !matches!(prefix.drop_meta(), Node::Symbol(word) if QUOTE_WORDS.contains(&word.as_str())) {
			return false;
		}
		// data never runs: the expression as it is written (`data 1+2` is 1+2)
		match rest {
			[only] => self.emit_literal(func, only),
			many => self.emit_literal(func, &Node::List(many.to_vec(), Bracket::None, Separator::Space)),
		}
		true
	}

	/// A bare word that is no variable, global or function
	fn unknown_word(&self, item: &Node) -> Option<String> {
		let Node::Symbol(name) = item.drop_meta() else { return None };
		let known = self.scope.lookup(name).is_some() || self.ctx.user_globals.contains_key(name) || self.ctx.user_functions.contains_key(name);
		(!known).then(|| name.clone())
	}

	/// A statement whose value is dropped keeps its own representation instead of being forced into an exact Int:
	/// an assignment to a float variable its f64, a text or list update (`s += "a"` in a loop body) its Node
	/// Statements run for their effects (a loop jump among them as the jump), their values dropped
	pub(super) fn emit_discarded_statements(&mut self, func: &mut Function, statements: &[Node]) {
		for statement in statements {
			if !self.emit_loop_jump(func, statement) {
				self.emit_discarded_statement(func, statement, Self::emit_node_instructions);
				func.instruction(&I::Drop);
			}
		}
	}

	pub(super) fn emit_discarded_statement(&mut self, func: &mut Function, item: &Node, emit: fn(&mut Self, &mut Function, &Node)) {
		if self.emit_discarded_branches(func, item) {
			return;
		}
		if let Some((name, value)) = self.typed_list_store(item) {
			self.emit_typed_list_store(func, &name, &value); // the array itself is dropped, it needs no Node
		} else if self.is_float_assignment(item) || self.is_float_call(item) {
			self.emit_float_value(func, item); // a float update or a call giving a float (shared_addf), dropped as an f64
		} else if self.is_ref_update(item) || self.is_output_call(item) || self.is_ref_value(item) || matches!(item.drop_meta(), Node::Empty) {
			self.emit_node_instructions(func, item);
		} else {
			emit(self, func, item);
		}
	}

	/// A call giving a float (`shared_addf(xs, i, v)`)
	fn is_float_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))))
			&& self.get_type(node).is_float()
	}

	/// Statements whose last value is an exact number (`{i*10}`, `{puti(i)}`): none that `emit_discarded_statement`
	/// keeps in another representation (a float, text or list update, a typed list store), no float
	pub(super) fn ends_in_number(&self, statements: &[Node]) -> bool {
		statements.last().is_some_and(|last| {
			!self.is_float_assignment(last) && !self.is_ref_update(last) && !self.is_ref_value(last)
				&& self.typed_list_store(last).is_none() && !self.get_type(last).is_ref() && !self.is_float_call(last)
		})
	}

	/// An `if` statement whose branches store a typed list (`if x > 10 then {out = out + [x]}`, what filter lowers to): its
	/// branches run as statements, so the stored list never becomes a Node; leaves a value for the caller to drop
	fn emit_discarded_branches(&mut self, func: &mut Function, item: &Node) -> bool {
		let (if_then, otherwise) = match item.drop_meta() {
			Node::Key(if_then, Op::Else, otherwise) => (if_then.as_ref(), Some(otherwise.as_ref())),
			other => (other, None),
		};
		let Node::Key(condition, Op::Then, then) = if_then.drop_meta() else { return false };
		let Node::Key(_, Op::If, condition) = condition.drop_meta() else { return false };
		let stores_typed_list = |branch: &Node| {
			let mut found = false;
			branch.visit(&mut |part| found |= self.typed_list_store(part).is_some());
			found
		};
		if !stores_typed_list(then) && !otherwise.is_some_and(stores_typed_list) {
			return false;
		}
		self.emit_condition(func, condition, Self::emit_block_value);
		func.instruction(&I::If(BlockType::Empty));
		self.emit_branch_statements(func, then);
		if let Some(otherwise) = otherwise {
			func.instruction(&I::Else);
			self.emit_branch_statements(func, otherwise);
		}
		func.instruction(&I::End);
		func.instruction(&I::I64Const(0)); // the statement's value, dropped
		true
	}

	fn emit_branch_statements(&mut self, func: &mut Function, branch: &Node) {
		let statements = match branch.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.clone(),
			other => vec![other.clone()],
		};
		self.emit_discarded_statements(func, &statements);
	}

	/// `s += "a"`, `xs = xs + [1]`, and an `if` whose branch does such an update
	fn is_ref_update(&self, item: &Node) -> bool {
		match item.drop_meta() {
			Node::Key(left, op, _) if *op == Op::Assign || op.is_compound_assign() => {
				matches!(left.drop_meta(), Node::Symbol(name) if self.scope.lookup(name).map(|local| local.kind)
					.or_else(|| self.ctx.user_globals.get(name).map(|(_, kind)| *kind)).is_some_and(|kind| kind.is_ref()))
			}
			Node::Key(_, Op::Then | Op::Else, _) => self.get_type(item).is_ref(),
			_ => false,
		}
	}

	/// A list, text or other Node-valued expression statement (`xs`, `f()` returning a list): its value is no number
	fn is_ref_value(&self, item: &Node) -> bool {
		match item.drop_meta() {
			Node::Key(_, op, _) if *op == Op::Assign || *op == Op::Define || op.is_compound_assign() => false,
			Node::Symbol(_) | Node::List(_, Bracket::Round, _) => self.get_type(item).is_ref(),
			_ => false,
		}
	}

	/// `x = v`, `x += v` to a float variable: float arithmetic also in a loop body
	/// `x = …` of a float variable, or an `if` with a branch ending in one (`if (x > t) x = x - t`,
	/// `if op == "+" { advance(); left = left + term() } else { return left }`)
	pub(super) fn is_float_assignment(&self, item: &Node) -> bool {
		let branch_assigns_float = |branch: &Node| match branch.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.last().is_some_and(|last| self.is_float_assignment(last)),
			other => self.is_float_assignment(other),
		};
		match item.drop_meta() {
			Node::Key(left, op, _) if matches!(op, Op::Define | Op::Assign) || op.is_compound_assign() => self.is_float_variable(left),
			Node::Key(_, Op::Then, then) => branch_assigns_float(then),
			Node::Key(if_then, Op::Else, otherwise) => {
				matches!(if_then.drop_meta(), Node::Key(_, Op::Then, then) if branch_assigns_float(then)) || branch_assigns_float(otherwise)
			}
			_ => false,
		}
	}

	/// Function definitions and imports: compiled ahead, no runtime value
	pub(super) fn is_definition(&self, item: &Node) -> bool {
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

/// Edit distance (insertions, deletions, substitutions, adjacent swaps) between two words
fn edit_distance(a: &str, b: &str) -> usize {
	let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
	let mut rows = vec![vec![0usize; b.len() + 1]; a.len() + 1];
	for (i, row) in rows.iter_mut().enumerate() {
		row[0] = i;
	}
	for (j, cell) in rows[0].iter_mut().enumerate() {
		*cell = j;
	}
	for i in 1..=a.len() {
		for j in 1..=b.len() {
			let cost = usize::from(a[i - 1] != b[j - 1]);
			rows[i][j] = (rows[i - 1][j] + 1).min(rows[i][j - 1] + 1).min(rows[i - 1][j - 1] + cost);
			if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
				rows[i][j] = rows[i][j].min(rows[i - 2][j - 2] + 1);
			}
		}
	}
	rows[a.len()][b.len()]
}
