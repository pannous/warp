//! Traps: the detail and position lines of a trapped run, and the runtime error a trap means
use super::super::*;

/// Traps of the exact runtime with the message a user should read instead of a wasm backtrace
pub(crate) const EXACT_TRAP_MESSAGES: [(&str, &str); 2] = [
	("int_shift_", "a shift needs an integer and a count from 0 to 65536"),
	("exact_pow", "an exact power needs an integer exponent"),
];
/// The exported global where a program leaves the value a runtime error is about (the subject of a missed switch)
pub const TRAP_DETAIL: &str = "trap_detail";
/// How a trapped run carries that value into its error: `trap detail: <value>`
pub const TRAP_DETAIL_PREFIX: &str = "trap detail: ";

/// The `trap detail: <value>` line of a trapped run; an Error value raised as it is (Node arithmetic on an Error, a text
/// builtin of an Error) names its message
pub fn trap_detail_line(detail: &Node) -> String {
	let shown = match detail {
		Node::Error(reason) => reason.serialize(),
		value => value.serialize(),
	};
	// one line in the trace, whatever lines the value has: `\` and line breaks escaped, trap_detail reads them back
	format!("{TRAP_DETAIL_PREFIX}{}", shown.replace('\\', "\\\\").replace('\n', "\\n"))
}

/// A detail written as a quoted text is the message it quotes, the `\` before a quote or `\` undone (`"a \"b\""` is
/// a "b"; node/serialization.rs quoted writes them)
fn unquoted(detail: &str) -> String {
	let Some(message) = detail.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) else {
		return detail.to_string();
	};
	let mut characters = message.chars();
	let mut unescaped = String::with_capacity(message.len());
	while let Some(character) = characters.next() {
		unescaped.push(match character {
			'\\' => characters.next().unwrap_or(character),
			plain => plain,
		});
	}
	unescaped
}

/// The value a trace's `trap detail: …` line carries, its line breaks back
pub(in crate::wasm_emitter) fn trap_detail(trace: &str) -> Option<String> {
	let line = trace.split_once(TRAP_DETAIL_PREFIX)?.1.lines().next()?;
	let mut detail = String::with_capacity(line.len());
	let mut characters = line.chars();
	while let Some(character) = characters.next() {
		detail.push(match (character, (character == '\\').then(|| characters.next()).flatten()) {
			(_, Some('n')) => '\n',
			(_, Some(escaped)) => escaped,
			(plain, None) => plain,
		});
	}
	Some(detail)
}

/// Why a unit word is an undefined variable: the static units pass left this quantity uncomputed
pub(in crate::wasm_emitter) const UNIT_AT_RUN_TIME: &str =
	" (a unit: quantities compute in functions, loops, lists, branches and print, but not in this use yet; notes/units_runtime.md)";
/// The global behind the export `trap_detail`: not a warp name, so no user global meets it
pub(in crate::wasm_emitter) const TRAP_DETAIL_GLOBAL: &str = "trap·detail";
/// The exported global where a program leaves the source position of the statement it runs, for its runtime errors
pub const TRAP_POSITION: &str = "trap_position";
/// How a trapped run carries that position into its error: `trap position: <line << 12 | column>`
pub const TRAP_POSITION_PREFIX: &str = "trap position: ";
/// The global behind the export `trap_position`
pub(in crate::wasm_emitter) const TRAP_POSITION_GLOBAL: &str = "trap·position";

/// The bits of the column in `trap_position`, an i32 (smaller code than an i64 per statement): columns to 4095
pub(in crate::wasm_emitter) const COLUMN_BITS: u32 = 12;

/// A source position of the program as the i32 `trap_position` holds: the line above, the column below
pub(in crate::wasm_emitter) fn encoded_position(position: &crate::compiler::meta::LineInfo) -> i32 {
	((position.line_nr as i32) << COLUMN_BITS) | position.column.min((1 << COLUMN_BITS) - 1) as i32
}

/// The `trap position: …` line of a trapped run, from the value of `trap_position`; none before the first statement
pub fn trap_position_line(encoded: i64) -> Option<String> {
	(encoded != 0).then(|| format!("{TRAP_POSITION_PREFIX}{encoded}"))
}

/// The position a trace's `trap position: …` line carries
pub(in crate::wasm_emitter) fn trap_position(trace: &str) -> Option<crate::compiler::meta::LineInfo> {
	let encoded: i64 = trace.split_once(TRAP_POSITION_PREFIX)?.1.lines().next()?.trim().parse().ok()?;
	Some(crate::compiler::meta::LineInfo {
		line_nr: (encoded >> COLUMN_BITS) as usize,
		column: (encoded & ((1 << COLUMN_BITS) - 1)) as usize,
		..Default::default()
	})
}

pub(in crate::wasm_emitter) const TERNARY_BRANCHES: &str = "`condition ? then : else`";
pub(in crate::wasm_emitter) const IF_THEN: &str = "`if condition then ...`";

/// A trap is a runtime error of the program; any other failure (link, instantiation, validation) is an error
/// of the compiler or the environment. Both become error values, never the unevaluated program.
#[cfg(feature = "native")]
pub(crate) fn failed_run(failure: anyhow::Error) -> Node {
	if let Some(task) = failure.chain().find_map(|cause| cause.downcast_ref::<crate::host::tasks::TaskFailure>()) {
		return crate::node::error(&task.0);
	}
	if let Some(out_of_time) = failure.chain().find_map(|cause| cause.downcast_ref::<warp_runtime::engine::OutOfTime>())
	{
		return crate::errors::run_time::out_of_time(out_of_time.0);
	}
	let trap = match failure.downcast_ref::<wasmtime::Trap>() {
		None => return crate::node::error(&format!("could not run the program: {failure:#}")),
		Some(wasmtime::Trap::OutOfFuel) => return out_of_fuel(crate::util::fuel_budget()),
		Some(trap) => trap,
	};
	trap_error(&format!("{:?}", failure), trap.to_string())
}

/// The characters of an item an unjoinable-item error shows: enough to know it, short enough for one line
const SHOWN_ITEM_LENGTH: usize = 60;
const JOINABLE_HINT: &str = "join takes texts and numbers: write the item as text first, \"\\(x)\" or x as text";

/// "not a joinable item: a key ('a'.dump), …" from not_a_joinable_item's detail, the item serialized
pub(in crate::wasm_emitter) fn unjoinable_message(item: &str) -> String {
	let kind = crate::normalize::without_hints(|| crate::warp_parser::parse_data(item)).kind();
	let shown: String = item.chars().take(SHOWN_ITEM_LENGTH).collect();
	let ellipsis = if shown.len() < item.len() { "…" } else { "" };
	let words = lists::list_ops::runtime_error_message(lists::list_ops::NOT_JOINABLE);
	format!("{words}: a {kind} ({shown}{ellipsis}); {JOINABLE_HINT}")
}

/// How an index error of an empty list ends: a lookup that found nothing (a served route answers it 404)
pub const EMPTY_RANGE: &str = "an empty list";

/// "index out of range: 7 not in 1…3" from index_out_of_range_of's detail `7:3` (the index asked for, the list's count)
pub(in crate::wasm_emitter) fn index_range_message(detail: &str) -> Option<String> {
	let numbers: Vec<i64> =
		detail.split(|c: char| !(c.is_ascii_digit() || c == '-')).filter_map(|part| part.parse().ok()).collect();
	let [index, count] = numbers.as_slice() else {
		return None;
	};
	let range = if *count == 0 { EMPTY_RANGE.to_string() } else { format!("1…{count}") };
	Some(format!("{}: {index} not in {range}", lists::list_ops::runtime_error_message("index_out_of_range")))
}

/// The runtime error a trap means, read from its trace: the function names on the stack and the `trap detail: …`
/// line; `trap` (the engine's own words) when the trace names none. Shared by wasmtime and the browser (web.rs).
pub fn trap_error(trace: &str, trap: String) -> Node {
	let missing_field = trace.split_once(lists::list_ops::NO_FIELD_PREFIX).map(|(_, rest)| {
		// a meta key starts with its mark (`no_field_@source`); a later `@` begins the url of a Firefox or Safari frame
		let mark = rest.strip_prefix(crate::node::ATTRIBUTE_MARK).map_or("", |_| "@");
		let name: String = rest[mark.len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
		format!("{}{mark}{name}", lists::list_ops::NO_FIELD_MESSAGE)
	});
	let no_case = trace.split_once(crate::switch::NO_CASE_PREFIX).map(|(_, rest)| {
		let label: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
		match trap_detail(trace) {
			Some(value) => format!("no case for {label} = {value}"),
			None => format!("no case for {label}"),
		}
	});
	let overflow = trace.split_once(crate::numbers::fixed_width::OVERFLOW_PREFIX).map(|(_, rest)| {
		let type_name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
		crate::numbers::fixed_width::overflow_message(&type_name)
	});
	// `return error "…"` from a number function: the message is the trap detail
	let returned_error = trace
		.contains(lists::list_ops::RETURNED_ERROR)
		.then(|| trap_detail(trace))
		.flatten()
		.map(|detail| unquoted(&detail));
	let index_range = trace
		.contains(lists::list_ops::INDEX_OUT_OF_RANGE_OF)
		.then(|| index_range_message(&trap_detail(trace)?))
		.flatten();
	// `[e 2]`, `q.k = e` of an Error e: its own message, then why it stops here
	let error_in_data = [lists::list_ops::ERROR_IN_DATA, lists::list_ops::ERROR_IN_FIELD]
		.into_iter()
		.find(|error| trace.contains(error))
		.and_then(|error| {
			Some(format!("{}: {}", unquoted(&trap_detail(trace)?), lists::list_ops::runtime_error_message(error)))
		});
	let unjoinable = trace
		.contains(lists::list_ops::NOT_JOINABLE)
		.then(|| trap_detail(trace))
		.flatten()
		.map(|item| unjoinable_message(&item));
	let runtime_error = error_in_data
		.or(returned_error)
		.or(index_range)
		.or(unjoinable)
		.or(missing_field)
		.or(no_case)
		.or(overflow)
		.or_else(|| {
			lists::list_ops::RUNTIME_ERRORS
				.iter()
				.find(|name| trace.contains(*name))
				.map(|name| lists::list_ops::runtime_error_message(name))
		});
	let exact_trap = EXACT_TRAP_MESSAGES
		.iter()
		.find(|(function, _)| trace.contains(function))
		.map(|(_, message)| message.to_string());
	// the engine's own integer divide trap (a division the emitter did not guard) reads as the guarded one
	let divide_trap = trap
		.ends_with(ENGINE_DIVIDE_BY_ZERO)
		.then(|| lists::list_ops::runtime_error_message(lists::list_ops::DIVIDE_BY_ZERO));
	let message = runtime_error.or(exact_trap).or(divide_trap).unwrap_or(trap);
	match trap_position(trace) {
		Some(position) => crate::node::error(&crate::errors::diagnostic::located_at(message, &position)),
		None => crate::node::error(&message),
	}
}

impl WasmGcEmitter {
	/// `error "…"` where a number is wanted (the branch of an Int if, analyzer::raises_error): it cannot be an Error value,
	/// so it fails the run with its message through `returned_error`, which `try` catches; true when it was one
	pub(crate) fn emit_raised_error(&mut self, func: &mut Function, node: &Node) -> bool {
		let Some(message) = returned_error_message(node) else {
			return false;
		};
		self.emit_trap_detail(func, message);
		self.emit_runtime_error(func, lists::list_ops::RETURNED_ERROR);
		true
	}

	/// The value of `return x` as the function gives it back: a Node, an f64 or an i64; `return error "…"` from a
	/// function of numbers fails the run with the message, it cannot give back an Error value
	pub(crate) fn emit_returned_value(&mut self, func: &mut Function, value: &Node) {
		if self.returns_list {
			self.emit_list_abi_value(func, value);
		} else if let Some(message) = returned_error_message(value).filter(|_| self.returns_number) {
			self.emit_trap_detail(func, message);
			self.emit_runtime_error(func, lists::list_ops::RETURNED_ERROR);
		} else if !self.returns_number {
			self.emit_node_instructions(func, value);
		} else if self.returns_float {
			self.emit_float_value(func, value);
		} else {
			self.emit_numeric_value(func, value);
		}
	}

	/// Leave `value` in the exported global `trap_detail` (declared on first use), for the runtime error that follows
	/// to name it: the runner reads it after the trap (wasm_reader::with_trap_detail)
	pub(crate) fn emit_trap_detail(&mut self, func: &mut Function, value: &Node) {
		self.trap_detail_global();
		self.emit_global_store(func, TRAP_DETAIL_GLOBAL, value);
		func.instruction(&I::Drop);
	}

	/// Leave the position of the statement being emitted in the exported global `trap_position`, for a runtime error of
	/// it to name: the runner reads it after the trap (wasm_reader::with_trap_detail). Again after a call, whose
	/// statements left theirs. A statement of a module or of generated code (meta::GENERATED_FILE) clears it: a position
	/// in another file would be read as the program's
	pub(crate) fn emit_trap_position(&mut self, func: &mut Function) {
		let Some(position) = self.statement_position.as_ref().filter(|position| position.line_nr > 0) else {
			return;
		};
		let encoded = match position.file {
			None => encoded_position(position),
			Some(_) => 0,
		};
		let global = self.trap_position_global();
		Self::emit_list(func, &[I32Const(encoded), I::GlobalSet(global)]);
	}

	/// The index of the exported global `trap_position`, declared on first use
	fn trap_position_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_POSITION_GLOBAL) {
			return *index;
		}
		let index = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
		self.exports.export(TRAP_POSITION, ExportKind::Global, index);
		self.ctx.user_globals.insert(TRAP_POSITION_GLOBAL.to_string(), (index, Kind::Int));
		index
	}

	/// The index of the exported global `trap_detail`, declared on first use
	pub(crate) fn trap_detail_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_DETAIL_GLOBAL) {
			return *index;
		}
		let index = self.declare_mutable_global(Kind::Empty);
		self.exports.export(TRAP_DETAIL, ExportKind::Global, index);
		self.ctx.user_globals.insert(TRAP_DETAIL_GLOBAL.to_string(), (index, Kind::Empty));
		index
	}
}
