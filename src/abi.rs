//! The ABI between a compiled warp module and the host: the export names the host calls (a list, a closure, an error, a
//! node), the trap lines a run reports, and how a list's kind field carries its bracket and separator. Whoever emits
//! modules (the Rust emitter, compiler.warp) and whoever runs them (host/, run/) agree on these (card rust-bootstrap:
//! the harness keeps them when the Rust compiler goes).
use crate::node::{Bracket, Node, Separator};
use wasm_encoder::ValType;

pub mod traps;

/// A value written as warp text (a map, a list, a serialized value) read back as a Node: by compiler.wasm's view mode in
/// the native host (card rust-bootstrap), by the data parser in the browser build, which has no compiler.wasm
#[cfg(feature = "native")]
pub fn value_of_text(text: &str) -> Node {
	crate::compiler::standalone::parsed(text)
}

#[cfg(not(feature = "native"))]
pub fn value_of_text(text: &str) -> Node {
	crate::normalize::without_hints(|| crate::warp_parser::parse_data(text))
}

pub const NEW_BOOL: &str = "new_bool";

/// new_type(name, body): a type declaration in code, `class P {x: int}` (Kind::TypeDef)
pub const NEW_TYPE: &str = "new_type";

/// The export name prefix of a closure's captured variable, `capture·add·k` (tasks copy them to their instance)
pub const CAPTURE_EXPORT_PREFIX: &str = "capture·";

/// closure_rebuild(name, captured) and closure_captured(f): what a host needs to carry a closure to another instance of
/// the module (a task, src/host/tasks.rs): its target's name and its captured values; the entry is found again by name
pub const CLOSURE_REBUILD: &str = "closure_rebuild";

/// closure_apply(f, arguments): f called with the items of the list `arguments`, as many as f takes (missing ones ø,
/// extra ones dropped, as JavaScript calls a function): how a foreign runtime calls back a warp function it was given
pub const CLOSURE_APPLY: &str = "closure_apply";

/// node_at(node, line, column, comment) -> the node with its source position and comment (or null), a $MetaNode
pub const NODE_AT: &str = "node_at";

/// The exported Int operations a host builds an exact number beyond the fixnums with (tasks.rs Builders, host.js
/// exactHandle): shift, add, subtract, divide
pub const EXACT_BUILDERS: [&str; 4] = ["int_shift_left", "exact_add", "exact_sub", "exact_div"];

pub const FIXNUM_MIN: i128 = -((1 << 62) - 1);

pub const FIXNUM_MAX: i128 = 1 << 62;

/// What `use { memory, table } from "env"` imports instead of defining: the linear memory and a function table
/// The store of the values a `warp dev` page keeps across reloads (in memory natively; dev.js, site.js)
pub const DEV_STORE: &str = "warp-dev";
/// The store of `session[k]` (in memory natively, std_adapters.rs; sessionStorage in the browser)
pub const SESSION_STORE: &str = "warp-session";
/// `database[k]`: the store whose file ends in this, `app.warp`'s is `app.database.json`
pub const DATABASE_STORE: &str = "database.json";

/// The keeping bits of a GPU map (gpu_maps): keep the result on the GPU, the source is kept there, the compiler chose it
pub const GPU_KEEP_RESULT: i64 = 1;
pub const GPU_SOURCE_KEPT: i64 = 2;
pub const GPU_AUTOMATIC: i64 = 4;
pub const GPU_AUTOMATIC_NOTICE: &str = "ran on the GPU by itself (f32, imprecise by design; write @cpu for exact f64)";
/// A channel's event names its channel and event: `channel/event`
pub const EVENT_SEPARATOR: char = '/';
/// The prefix of a closure's function standing for a builtin word
pub const CLOSURE_NEW: &str = "closure_new";
/// A task wrapper's suffix: `total·node` takes the argument list
pub const NODE_WRAPPER_SUFFIX: &str = "·node";
/// Without a server, a form of the page asks the program's own routes
pub const PAGE_SUBMITTED: &str = "page·submitted";

pub const MEMORY_ENTITY: &str = "memory";

/// The exported linear_new and kernel prefix the host calls to split a map (tasks.rs split_map)
pub const SPLIT_BLOCK_NEW: &str = "ffi_linear_new";

/// ints_to_list(address, count) -> list: the `count` u32 at `address` as a square list of Ints (gpu_render's pixels)
pub const INTS_TO_LIST: &str = "ints_to_list";

/// list_to_ints(list, address, room) -> count: the list's first `room` Ints as i64 at `address` (paint's pixels, negatives
/// as their magnitude), -1 when an item is no fixnum Int and the host reads the list itself
pub const LIST_TO_INTS: &str = "list_to_ints";

/// `return error "…"` from a function that returns numbers: the run fails, the message is the trap detail
pub const RETURNED_ERROR: &str = "returned_error";

/// error_of(text) → Error: exported, the host builds the Error of a caught stack overflow with it (guarded_call)
pub const ERROR_OF: &str = "error_of";

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

/// The exported global where a program leaves the name of the runtime word a runtime error fails in (`index_out_of_range`,
/// `returned_error` …): trap_error reads the error by that name, which a trace no longer shows once Cranelift has inlined
/// the word into its caller
pub const TRAP_NAME: &str = "trap_name";

/// How a trapped run carries that name into its error: `trap name: <word>`
pub const TRAP_NAME_PREFIX: &str = "trap name: ";

/// The exported global where a program leaves the source position of the statement it runs, for its runtime errors
pub const TRAP_POSITION: &str = "trap_position";

/// How a trapped run carries that position into its error: `trap position: <line << 12 | column>`
pub const TRAP_POSITION_PREFIX: &str = "trap position: ";

/// The `trap position: …` line of a trapped run, from the value of `trap_position`; none before the first statement
pub fn trap_position_line(encoded: i64) -> Option<String> {
	(encoded != 0).then(|| format!("{TRAP_POSITION_PREFIX}{encoded}"))
}

/// The bracket of a list as the high bits of its kind (`new_list(first, rest, bracket_info)`); node_in reads it back
pub fn bracket_info(bracket: &Bracket) -> i64 {
	match bracket {
		Bracket::Curly => 0,
		Bracket::Square => 1,
		Bracket::Round => 2,
		Bracket::Less => 3,
		Bracket::Other(_, _) => 4,
		Bracket::None => 5,
	}
}

/// The separators of a list by their code above SEPARATOR_SHIFT (None is 0: a list made at run time names none)
pub const SEPARATORS: [Separator; 6] =
	[Separator::None, Separator::Space, Separator::Comma, Separator::Semicolon, Separator::Newline, Separator::Tab];

/// The code of a list's separator, as `separated` and `n.separator` know it
pub fn separator_code(separator: &Separator) -> i64 {
	SEPARATORS.iter().position(|known| known == separator).unwrap_or(0) as i64
}

/// The kind field's info bits (above KIND_BITS) of a list: its bracket and its separator (card keep-list)
pub fn list_info(bracket: &Bracket, separator: &Separator) -> i64 {
	bracket_info(bracket)
		| separator_code(separator) << (crate::type_kinds::SEPARATOR_SHIFT - crate::type_kinds::KIND_BITS)
}

/// The separator of a list's kind field, separator_code read back
pub fn separator_of_kind(kind: i64) -> Separator {
	let code = (kind & crate::type_kinds::SEPARATOR_MASK) >> crate::type_kinds::SEPARATOR_SHIFT;
	SEPARATORS.get(code as usize).cloned().unwrap_or(Separator::None)
}

/// The bracket of a list's kind high bits, bracket_info read back (Other as None)
pub fn bracket_of_info(info: i64) -> Bracket {
	match info {
		0 => Bracket::Curly,
		1 => Bracket::Square,
		2 => Bracket::Round,
		3 => Bracket::Less,
		_ => Bracket::None,
	}
}

/// Whether an int is a fixnum (an i31-boxed small int), not a big int on the heap
pub fn is_fixnum(n: i64) -> bool {
	(FIXNUM_MIN..=FIXNUM_MAX).contains(&(n as i128))
}

/// The library the linear words belong to: emitted by the module, not imported (import_manager skips it)
pub const LINEAR_LIBRARY: &str = "linear";
pub const LINEAR_NEW: &str = "linear_new";
pub const LINEAR_COUNT: &str = "linear_count";
/// get, set, add of an Int cell and of a float cell (the order of shared_arrays' element words)
pub const LINEAR_INT_WORDS: [&str; 3] = ["linear_get", "linear_set", "linear_add"];
pub const LINEAR_FLOAT_WORDS: [&str; 3] = ["linear_getf", "linear_setf", "linear_addf"];
/// linear_dotf(xs, ys) -> the sum of xs#i * ys#i over xs's count of cells, two pairs at a time in f64x2 lanes: `dot` and
/// `sum(xs .* ys)` of two linear float arrays of equal length (shared_arrays.rs checks it)
pub const LINEAR_DOT: &str = "linear_dotf";

/// The prefix of a split kernel's registry name (ffi_linear_mapf·…), as tasks call it in instances of their own
pub const SPLIT_KERNEL_PREFIX: &str = "ffi_";

/// name, parameters, results of the linear words
pub fn linear_word_signatures() -> [(&'static str, Vec<ValType>, Vec<ValType>); 9] {
	let [get, set, add] = LINEAR_INT_WORDS;
	let [getf, setf, addf] = LINEAR_FLOAT_WORDS;
	[
		(LINEAR_NEW, vec![ValType::I64], vec![ValType::I64]),
		(LINEAR_COUNT, vec![ValType::I64], vec![ValType::I64]),
		(get, vec![ValType::I64, ValType::I64], vec![ValType::I64]),
		(set, vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
		(add, vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
		(getf, vec![ValType::I64, ValType::I64], vec![ValType::F64]),
		(setf, vec![ValType::I64, ValType::I64, ValType::F64], vec![ValType::F64]),
		(addf, vec![ValType::I64, ValType::I64, ValType::F64], vec![ValType::F64]),
		(LINEAR_DOT, vec![ValType::I64, ValType::I64], vec![ValType::F64]),
	]
}

/// The literal an Int read back shows, by the radix mark above its kind bits (type_kinds HEX_INFO, BINARY_INFO): 0xFF,
/// -0x1, 0b101; None for a plain Int
pub fn marked_literal(info: i64, value: i64) -> Option<String> {
	let sign = if value < 0 { "-" } else { "" };
	let magnitude = value.unsigned_abs();
	match info {
		crate::type_kinds::HEX_INFO => Some(format!("{sign}0x{magnitude:X}")),
		crate::type_kinds::BINARY_INFO => Some(format!("{sign}0b{magnitude:b}")),
		_ => None,
	}
}

/// Between a variable and the name of a temporary made for it (`xs·range`, `xs·item`, `m·removed`)
pub const TEMPORARY_SEPARATOR: &str = "·";

/// The SQL function a query calls a warp function through (database.rs)
pub const WARP_CALL: &str = "warp_call";
/// `Team?`: an optional field
pub const OPTIONAL_MARK: char = '?';
