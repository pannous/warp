//! The runtime errors a trap means: the names of the functions a compiled module traps in, their messages, fixes and
//! Python classes, and the error value a trapped run's trace reads as. The ABI of failure between whoever emits modules
//! (the Rust emitter, compiler.warp's runtime) and whoever runs them (run/, host/, the browser's web.rs) (card
//! rust-bootstrap: the harness keeps it when the Rust compiler goes).
use crate::abi::{RETURNED_ERROR, TRAP_DETAIL_PREFIX, TRAP_POSITION_PREFIX};
use crate::node::Node;

/// Runtime errors trap inside a function of that name; eval reports the name as an error value
pub const INDEX_NOT_INTEGRAL: &str = "index_must_be_an_integer";

/// `y times "ab"` with y = 2.5 held in a variable (a literal fraction is a compile-time type error)
pub const COUNT_NOT_INTEGRAL: &str = "count_must_be_an_integer";

/// `x:int = 3; x += 0.5`: a declared int (variable or field) given a fraction at run time; a literal one is a type error
pub const INT_NOT_WHOLE: &str = "an_int_must_be_a_whole_number";

/// `m.a * 2` with a text in m.a, where the kinds are known only at run time (NODE_ARITHMETIC)
pub const NOT_A_NUMBER: &str = "not_a_number";

/// `t = ø; t += "a"` with ø held at run time: ø + text fails as ø + 1 does (decision empty-plus-text)
pub const EMPTY_PLUS_TEXT: &str = "ø_plus_a_text";

/// How to fix a runtime error, appended to its message: the trap knows no source position, so the fix is generic
pub const WHOLE_NUMBER_FIX: &str = "fix: compute it with /! (floor division) or `… as int`";

pub const RUNTIME_ERROR_FIXES: [(&str, &str); 9] = [
	(INVALID_CODE_POINT, "a code point is 0…0x10FFFF, outside the surrogates 0xD800…0xDFFF"),
	(NOT_A_MAP, "keys, values and entries are a map's; a list's positions are 1..count(xs)"),
	(INDEX_NOT_INTEGRAL, WHOLE_NUMBER_FIX),
	(COUNT_NOT_INTEGRAL, WHOLE_NUMBER_FIX),
	(INT_NOT_WHOLE, WHOLE_NUMBER_FIX),
	(LIST_CANNOT_HOLD, "its declared element type does not admit it (P215)"),
	(ERROR_IN_DATA, ERROR_IN_DATA_FIX),
	(ERROR_IN_FIELD, ERROR_IN_DATA_FIX),
	(EMPTY_PLUS_TEXT, "fix: start a text with `t = \"\"`, a list with `t = []`"),
];

/// The class of a caught runtime error, Python's name for it (decision error-classes): each a subclass of Error in
/// lib/prelude.warp, which `except IndexError` tests
pub fn runtime_error_class(name: &str) -> &'static str {
	match name {
		"index_out_of_range" => "IndexError",
		"key_not_found" => "KeyError",
		DIVIDE_BY_ZERO => "ZeroDivisionError",
		"out_of_memory" => "MemoryError",
		"invalid_number"
		| "min_of_an_empty_list"
		| "max_of_an_empty_list"
		| "reduce_of_an_empty_list"
		| "empty_separator"
		| "non_ascii_text"
		| NOT_A_NUMBER
		| INVALID_CODE_POINT
		| NOT_FINITE
		| NEGATIVE_UNCERTAINTY
		| INTERVAL_AND_GAUSSIAN => "ValueError",
		"float_out_of_int_range" => "OverflowError",
		name if crate::numbers::fixed_width::FIXED_WIDTHS.iter().any(|width| width.trap == name) => "OverflowError",
		name if name.starts_with(NO_CASE_PREFIX) => "ValueError",
		_ => "TypeError",
	}
}

/// The message of the runtime error trapped in the function `name`: its words, then its fix if it has one
pub fn runtime_error_message(name: &str) -> String {
	let words = name.replace('_', " ");
	match RUNTIME_ERROR_FIXES.iter().find(|(error, _)| *error == name) {
		Some((_, fix)) => format!("{words}; {fix}"),
		None => words,
	}
}

/// index_out_of_range_of(index, count): index_out_of_range with the index and the list's count as the trap detail, which
/// trap_error turns into "index out of range: 7 not in 1…3" (card index-range)
pub const INDEX_OUT_OF_RANGE_OF: &str = "index_out_of_range_of";

/// `a % 0`, `a rem 0`: an integer divide by zero (big_int::emit_nonzero_divisor)
pub const DIVIDE_BY_ZERO: &str = "divide_by_zero";

/// `print [f]`, `join`: an item that is no text, number or other value with a text form; the item is the trap detail
pub const NOT_JOINABLE: &str = "not_a_joinable_item";

/// `[e 2]` of an Error e: data holds no error (decision error-optional); the Error is the trap detail
pub const ERROR_IN_DATA: &str = "a_list_item_holds_no_error";

/// `q.k = e`, `{k: e}`, `pt(e)` of an Error e
pub const ERROR_IN_FIELD: &str = "a_field_holds_no_error";

pub const ERROR_IN_DATA_FIX: &str = "handle it with try … else, or declare `or error`";

/// `x.keys`, `x.values`, `x.entries` of a value that is no map (card keys-value)
pub const NOT_A_MAP: &str = "not_a_map";

pub const RUNTIME_ERRORS: [&str; 35] = [
	"index_out_of_range",
	INDEX_NOT_INTEGRAL,
	"invalid_number",
	"out_of_memory",
	"key_not_found",
	"float_out_of_int_range",
	"min_of_an_empty_list",
	"max_of_an_empty_list",
	"reduce_of_an_empty_list",
	"not_a_list",
	"not_a_text",
	"not_an_int",
	"non_ascii_text",
	NOT_JOINABLE,
	"empty_separator",
	"not_an_object",
	"not_comparable",
	NOT_A_FUNCTION,
	WRONG_ARGUMENT_COUNT,
	WRONG_NUMBER_OF_VALUES,
	RETURNED_ERROR,
	"not_a_character",
	COUNT_NOT_INTEGRAL,
	NOT_A_NUMBER,
	DIVIDE_BY_ZERO,
	INT_NOT_WHOLE,
	LIST_CANNOT_HOLD,
	NEGATIVE_UNCERTAINTY,
	INTERVAL_AND_GAUSSIAN,
	NOT_A_MAP,
	INVALID_CODE_POINT,
	NOT_FINITE,
	ERROR_IN_DATA,
	ERROR_IN_FIELD,
	EMPTY_PLUS_TEXT,
];

/// `n as codepoint` of a number no character has: below 0, above U+10FFFF or a UTF-16 surrogate
pub const INVALID_CODE_POINT: &str = "invalid_code_point";

pub const LIST_CANNOT_HOLD: &str = "list_cannot_hold_this_item";

/// The runtime error of float_as_exact for ∞ and NaN
pub const NOT_FINITE: &str = "not_a_finite_number";

/// `5 ± -1`
pub const NEGATIVE_UNCERTAINTY: &str = "an_uncertainty_must_not_be_negative";

/// `(5 ± 1σ) + (2 ± 1)`: which of the two the result would be is unclear, so it fails
pub const INTERVAL_AND_GAUSSIAN: &str = "an_interval_and_a_gaussian_do_not_mix";

/// Runtime errors of a closure call: the value called is no closure, or a closure of another arity
pub const NOT_A_FUNCTION: &str = "not_a_function";

pub const WRONG_ARGUMENT_COUNT: &str = "wrong_number_of_arguments";

/// `a, b = [1, 2, 3]`: the runtime error of unpacking another number of items than names
pub const WRONG_NUMBER_OF_VALUES: &str = "wrong_number_of_values";

/// The runtime error of a missing constant field is a function `no_field_<name>`; eval reports it as `no field <name>`
pub const NO_FIELD_PREFIX: &str = "no_field_";

/// How a missing field's error begins: `no field title`
pub const NO_FIELD_MESSAGE: &str = "no field ";

/// The runtime error of a missing field whose name is known only at run time (lib/compiler/runtime.warp map_get):
/// the trap detail holds the name
pub const MISSING_FIELD: &str = "missing_field";

pub const NO_CASE_PREFIX: &str = "no_case_";

/// wasmtime's words for an integer division or remainder by zero
pub const ENGINE_DIVIDE_BY_ZERO: &str = "integer divide by zero";

/// Traps of the exact runtime with the message a user should read instead of a wasm backtrace
pub const EXACT_TRAP_MESSAGES: [(&str, &str); 2] = [
	("int_shift_", "a shift needs an integer and a count from 0 to 65536"),
	("exact_pow", "an exact power needs an integer exponent"),
];

/// A detail written as a quoted text is the message it quotes, the `\` before a quote or `\` undone (`"a \"b\""` is
/// a "b"; node/serialization.rs quoted writes them)
pub fn unquoted(detail: &str) -> String {
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
pub fn trap_detail(trace: &str) -> Option<String> {
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

/// The bits of the column in `trap_position`, an i32 (smaller code than an i64 per statement): columns to 4095
pub const COLUMN_BITS: u32 = 12;

/// The position a trace's `trap position: …` line carries
pub fn trap_position(trace: &str) -> Option<crate::compiler::meta::LineInfo> {
	let encoded: i64 = trace.split_once(TRAP_POSITION_PREFIX)?.1.lines().next()?.trim().parse().ok()?;
	Some(crate::compiler::meta::LineInfo {
		line_nr: (encoded >> COLUMN_BITS) as usize,
		column: (encoded & ((1 << COLUMN_BITS) - 1)) as usize,
		..Default::default()
	})
}

/// The characters of an item an unjoinable-item error shows: enough to know it, short enough for one line
pub const SHOWN_ITEM_LENGTH: usize = 60;

pub const JOINABLE_HINT: &str = "join takes texts and numbers: write the item as text first, \"\\(x)\" or x as text";

/// "not a joinable item: a key ('a'.dump), …" from not_a_joinable_item's detail, the item serialized
pub fn unjoinable_message(item: &str) -> String {
	let kind = crate::abi::value_of_text(item).kind();
	let shown: String = item.chars().take(SHOWN_ITEM_LENGTH).collect();
	let ellipsis = if shown.len() < item.len() { "…" } else { "" };
	let words = runtime_error_message(NOT_JOINABLE);
	format!("{words}: a {kind} ({shown}{ellipsis}); {JOINABLE_HINT}")
}

/// How an index error of an empty list ends: a lookup that found nothing (a served route answers it 404)
pub const EMPTY_RANGE: &str = "an empty list";

/// "index out of range: 7 not in 1…3" from index_out_of_range_of's detail `7:3` (the index asked for, the list's count)
pub fn index_range_message(detail: &str) -> Option<String> {
	let numbers: Vec<i64> =
		detail.split(|c: char| !(c.is_ascii_digit() || c == '-')).filter_map(|part| part.parse().ok()).collect();
	let [index, count] = numbers.as_slice() else {
		return None;
	};
	let range = if *count == 0 { EMPTY_RANGE.to_string() } else { format!("1…{count}") };
	Some(format!("{}: {index} not in {range}", runtime_error_message("index_out_of_range")))
}

/// The runtime error a trap means, read from its trace: the function names on the stack and the `trap detail: …`
/// line; `trap` (the engine's own words) when the trace names none. Shared by wasmtime and the browser (web.rs).
pub fn trap_error(trace: &str, trap: String) -> Node {
	let missing_field = trace.split_once(NO_FIELD_PREFIX).map(|(_, rest)| {
		// a meta key starts with its mark (`no_field_@source`); a later `@` begins the url of a Firefox or Safari frame
		let mark = rest.strip_prefix(crate::node::ATTRIBUTE_MARK).map_or("", |_| "@");
		let name: String = rest[mark.len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
		format!("{}{mark}{name}", NO_FIELD_MESSAGE)
	});
	let missing_field = missing_field.or_else(|| {
		let name = trace.contains(MISSING_FIELD).then(|| trap_detail(trace)).flatten()?;
		Some(format!("{}{}", NO_FIELD_MESSAGE, unquoted(&name)))
	});
	let no_case = trace.split_once(NO_CASE_PREFIX).map(|(_, rest)| {
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
	let returned_error =
		trace.contains(RETURNED_ERROR).then(|| trap_detail(trace)).flatten().map(|detail| unquoted(&detail));
	let index_range =
		trace.contains(INDEX_OUT_OF_RANGE_OF).then(|| index_range_message(&trap_detail(trace)?)).flatten();
	// `[e 2]`, `q.k = e` of an Error e: its own message, then why it stops here
	let error_in_data = [ERROR_IN_DATA, ERROR_IN_FIELD]
		.into_iter()
		.find(|error| trace.contains(error))
		.and_then(|error| Some(format!("{}: {}", unquoted(&trap_detail(trace)?), runtime_error_message(error))));
	let unjoinable =
		trace.contains(NOT_JOINABLE).then(|| trap_detail(trace)).flatten().map(|item| unjoinable_message(&item));
	let runtime_error = error_in_data
		.or(returned_error)
		.or(index_range)
		.or(unjoinable)
		.or(missing_field)
		.or(no_case)
		.or(overflow)
		.or_else(|| RUNTIME_ERRORS.iter().find(|name| trace.contains(*name)).map(|name| runtime_error_message(name)));
	let exact_trap = EXACT_TRAP_MESSAGES
		.iter()
		.find(|(function, _)| trace.contains(function))
		.map(|(_, message)| message.to_string());
	// the engine's own integer divide trap (a division the emitter did not guard) reads as the guarded one
	let divide_trap = trap.ends_with(ENGINE_DIVIDE_BY_ZERO).then(|| runtime_error_message(DIVIDE_BY_ZERO));
	let message = runtime_error.or(exact_trap).or(divide_trap).unwrap_or(trap);
	match trap_position(trace) {
		Some(position) => crate::node::error(&crate::errors::diagnostic::located_at(message, &position)),
		None => crate::node::error(&message),
	}
}
