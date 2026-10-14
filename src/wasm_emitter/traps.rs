//! Traps: the detail and position lines of a trapped run, and the runtime error a trap means
use super::*;

/// The `trap detail: <value>` line of a trapped run; an Error value raised as it is (Node arithmetic on an Error, a text
/// builtin of an Error) names its message
pub fn trap_detail_line(detail: &Node) -> String {
	let shown = match detail {
		Node::Error(reason) => reason.serialize(),
		value => value.serialize(),
	};
	// one line in the trace, whatever lines the value has: `\` and line breaks escaped, trap_detail reads them back
	format!(
		"{TRAP_DETAIL_PREFIX}{}",
		shown.replace('\\', "\\\\").replace('\n', "\\n")
	)
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
pub(super) fn trap_detail(trace: &str) -> Option<String> {
	let line = trace.split_once(TRAP_DETAIL_PREFIX)?.1.lines().next()?;
	let mut detail = String::with_capacity(line.len());
	let mut characters = line.chars();
	while let Some(character) = characters.next() {
		detail.push(
			match (character, (character == '\\').then(|| characters.next()).flatten()) {
				(_, Some('n')) => '\n',
				(_, Some(escaped)) => escaped,
				(plain, None) => plain,
			},
		);
	}
	Some(detail)
}

/// Why a unit word is an undefined variable: the static units pass left this quantity uncomputed
pub(super) const UNIT_AT_RUN_TIME: &str =
	" (a unit: quantities compute in functions, loops, lists, branches and print, but not in this use yet; notes/units_runtime.md)";
/// The global behind the export `trap_detail`: not a warp name, so no user global meets it
pub(super) const TRAP_DETAIL_GLOBAL: &str = "trap·detail";
/// The exported global where a program leaves the source position of the statement it runs, for its runtime errors
pub const TRAP_POSITION: &str = "trap_position";
/// How a trapped run carries that position into its error: `trap position: <line << 12 | column>`
pub const TRAP_POSITION_PREFIX: &str = "trap position: ";
/// The global behind the export `trap_position`
pub(super) const TRAP_POSITION_GLOBAL: &str = "trap·position";

/// The bits of the column in `trap_position`, an i32 (smaller code than an i64 per statement): columns to 4095
pub(super) const COLUMN_BITS: u32 = 12;

/// A source position of the program as the i32 `trap_position` holds: the line above, the column below
pub(super) fn encoded_position(position: &crate::compiler::meta::LineInfo) -> i32 {
	((position.line_nr as i32) << COLUMN_BITS) | position.column.min((1 << COLUMN_BITS) - 1) as i32
}

/// The `trap position: …` line of a trapped run, from the value of `trap_position`; none before the first statement
pub fn trap_position_line(encoded: i64) -> Option<String> {
	(encoded != 0).then(|| format!("{TRAP_POSITION_PREFIX}{encoded}"))
}

/// The position a trace's `trap position: …` line carries
pub(super) fn trap_position(trace: &str) -> Option<crate::compiler::meta::LineInfo> {
	let encoded: i64 = trace
		.split_once(TRAP_POSITION_PREFIX)?
		.1
		.lines()
		.next()?
		.trim()
		.parse()
		.ok()?;
	Some(crate::compiler::meta::LineInfo {
		line_nr: (encoded >> COLUMN_BITS) as usize,
		column: (encoded & ((1 << COLUMN_BITS) - 1)) as usize,
		..Default::default()
	})
}

pub(super) const TERNARY_BRANCHES: &str = "`condition ? then : else`";
pub(super) const IF_THEN: &str = "`if condition then ...`";

/// A trap is a runtime error of the program; any other failure (link, instantiation, validation) is an error
/// of the compiler or the environment. Both become error values, never the unevaluated program.
#[cfg(feature = "native")]
pub(crate) fn failed_run(failure: anyhow::Error) -> Node {
	if let Some(task) = failure
		.chain()
		.find_map(|cause| cause.downcast_ref::<crate::host::tasks::TaskFailure>())
	{
		return crate::node::error(&task.0);
	}
	let trap = match failure.downcast_ref::<wasmtime::Trap>() {
		None => return crate::node::error(&format!("could not run the program: {failure:#}")),
		Some(wasmtime::Trap::OutOfFuel) => return out_of_fuel(crate::util::fuel_budget()),
		Some(trap) => trap,
	};
	trap_error(&format!("{:?}", failure), trap.to_string())
}

/// How an index error of an empty list ends: a lookup that found nothing (a served route answers it 404)
pub const EMPTY_RANGE: &str = "an empty list";

/// "index out of range: 7 not in 1…3" from index_out_of_range_of's detail `7:3` (the index asked for, the list's count)
pub(super) fn index_range_message(detail: &str) -> Option<String> {
	let numbers: Vec<i64> = detail
		.split(|c: char| !(c.is_ascii_digit() || c == '-'))
		.filter_map(|part| part.parse().ok())
		.collect();
	let [index, count] = numbers.as_slice() else {
		return None;
	};
	let range = if *count == 0 {
		EMPTY_RANGE.to_string()
	} else {
		format!("1…{count}")
	};
	Some(format!(
		"{}: {index} not in {range}",
		list_ops::runtime_error_message("index_out_of_range")
	))
}

/// The runtime error a trap means, read from its trace: the function names on the stack and the `trap detail: …`
/// line; `trap` (the engine's own words) when the trace names none. Shared by wasmtime and the browser (web.rs).
pub fn trap_error(trace: &str, trap: String) -> Node {
	let missing_field = trace.split_once(list_ops::NO_FIELD_PREFIX).map(|(_, rest)| {
		// a meta key starts with its mark (`no_field_@source`); a later `@` begins the url of a Firefox or Safari frame
		let mark = rest.strip_prefix(crate::node::ATTRIBUTE_MARK).map_or("", |_| "@");
		let name: String = rest[mark.len()..]
			.chars()
			.take_while(|c| c.is_alphanumeric() || *c == '_')
			.collect();
		format!("{}{mark}{name}", list_ops::NO_FIELD_MESSAGE)
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
		.contains(list_ops::RETURNED_ERROR)
		.then(|| trap_detail(trace))
		.flatten()
		.map(|detail| unquoted(&detail));
	let index_range = trace
		.contains(list_ops::INDEX_OUT_OF_RANGE_OF)
		.then(|| index_range_message(&trap_detail(trace)?))
		.flatten();
	let runtime_error = returned_error
		.or(index_range)
		.or(missing_field)
		.or(no_case)
		.or(overflow)
		.or_else(|| {
			list_ops::RUNTIME_ERRORS
				.iter()
				.find(|name| trace.contains(*name))
				.map(|name| list_ops::runtime_error_message(name))
		});
	let exact_trap = EXACT_TRAP_MESSAGES
		.iter()
		.find(|(function, _)| trace.contains(function))
		.map(|(_, message)| message.to_string());
	// the engine's own integer divide trap (a division the emitter did not guard) reads as the guarded one
	let divide_trap = trap
		.ends_with(ENGINE_DIVIDE_BY_ZERO)
		.then(|| list_ops::runtime_error_message(list_ops::DIVIDE_BY_ZERO));
	let message = runtime_error.or(exact_trap).or(divide_trap).unwrap_or(trap);
	match trap_position(trace) {
		Some(position) => crate::node::error(&crate::diagnostic::located_at(message, &position)),
		None => crate::node::error(&message),
	}
}
