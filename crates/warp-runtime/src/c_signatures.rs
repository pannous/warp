//! C functions as warp calls them, without their headers: how each parameter and the result cross (notes/ffi_handles.md).
//! warp reads a library's headers into these (src/ffi); an executable carries the ones its program imports in front of
//! its machine code, so the stub links them without reading a header (c_calls.rs, card exe-c-ffi)

/// What a carried program starts with when it calls C, which machine code never does
pub const CARRIED_C_FUNCTIONS: &[u8; 8] = b"WRPCcfun";
/// The libc functions link_libc_functions defines by hand; any other libc import is linked from the process
pub const HAND_LINKED_LIBC: [&str; 8] = ["abs", "strlen", "atoi", "atol", "atof", "strcmp", "strncmp", "rand"];
const FIELD_SEPARATOR: char = '\t';
const LIST_SEPARATOR: char = ',';

/// What a C pointer type crosses as
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CPointer {
	/// `char *`, `const unsigned char *`: a warp text (a parameter its NUL-terminated letters in linear memory)
	Text,
	/// `void *`, `int *`: a parameter points into linear memory; a result is a handle
	Memory,
	/// `sqlite3 *`, `FILE *`: an id into the run's handle table (CHandles), 0 is NULL
	Handle,
	/// `sqlite3 **ppDb`: left out of the warp call; the first one becomes the result, the others receive NULL
	Out,
}

/// A C parameter: a number by its wasm type, or a pointer by what it crosses as
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParamType {
	I32,
	I64,
	F32,
	F64,
	/// linear memory: a text, a buffer
	Ptr,
	/// a handle id (CPointer::Handle)
	Handle,
	/// an out-pointer, filled by the call (CPointer::Out)
	Out,
}

/// A C result's register: none, an integer, a float, or a bool (its low byte)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RetType {
	Void,
	I32,
	I64,
	F32,
	F64,
	Bool,
}

const PARAM_NAMES: [(ParamType, &str); 7] = [
	(ParamType::I32, "i32"),
	(ParamType::I64, "i64"),
	(ParamType::F32, "f32"),
	(ParamType::F64, "f64"),
	(ParamType::Ptr, "ptr"),
	(ParamType::Handle, "handle"),
	(ParamType::Out, "out"),
];
const RET_NAMES: [(RetType, &str); 6] = [
	(RetType::Void, "void"),
	(RetType::I32, "i32"),
	(RetType::I64, "i64"),
	(RetType::F32, "f32"),
	(RetType::F64, "f64"),
	(RetType::Bool, "bool"),
];
/// The pointers a result crosses as; a number result is "number"
const RESULT_NAMES: [(Option<CPointer>, &str); 3] =
	[(None, "number"), (Some(CPointer::Text), "text"), (Some(CPointer::Handle), "handle")];

fn name_of<T: PartialEq + Copy>(names: &[(T, &'static str)], value: T) -> &'static str {
	names.iter().find(|(known, _)| *known == value).map_or("?", |(_, name)| name)
}

fn value_of<T: Copy>(names: &[(T, &str)], name: &str) -> Option<T> {
	names.iter().find(|(_, known)| *known == name).map(|(value, _)| *value)
}

/// A C function as an import of a program: its import module (the library as the program names it, `zlib`), the
/// library it loads (`z`), and how its parameters and result cross
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CFunction {
	pub module: String,
	pub library: String,
	pub name: String,
	pub param_types: Vec<ParamType>,
	/// Text, Handle, or None for the C function's own number (ret_type)
	pub result: Option<CPointer>,
	pub ret_type: RetType,
	/// what the first out-pointer points to (`sqlite3_stmt` of `sqlite3_stmt **ppStmt`), for the error of a NULL there
	pub out_pointee: Option<String>,
}

impl CFunction {
	/// One line of the carried table, its fields tab-separated: `zlib z compressBound i64 i64 number` and an empty
	/// out-pointee
	pub fn line(&self) -> String {
		let params: Vec<&str> = self.param_types.iter().map(|param| name_of(&PARAM_NAMES, *param)).collect();
		[
			self.module.as_str(),
			&self.library,
			&self.name,
			&params.join(&LIST_SEPARATOR.to_string()),
			name_of(&RET_NAMES, self.ret_type),
			name_of(&RESULT_NAMES, self.result),
			self.out_pointee.as_deref().unwrap_or_default(),
		]
		.join(&FIELD_SEPARATOR.to_string())
	}

	pub fn of_line(line: &str) -> Option<CFunction> {
		let [module, library, name, params, ret_type, result, out_pointee] =
			line.split(FIELD_SEPARATOR).collect::<Vec<_>>().try_into().ok()?;
		let param_types = params
			.split(LIST_SEPARATOR)
			.filter(|param| !param.is_empty())
			.map(|param| value_of(&PARAM_NAMES, param))
			.collect::<Option<_>>()?;
		Some(CFunction {
			module: module.to_string(),
			library: library.to_string(),
			name: name.to_string(),
			param_types,
			result: value_of(&RESULT_NAMES, result)?,
			ret_type: value_of(&RET_NAMES, ret_type)?,
			out_pointee: (!out_pointee.is_empty()).then(|| out_pointee.to_string()),
		})
	}
}

/// `machine_code` with the C functions it imports in front: CARRIED_C_FUNCTIONS, the table's length (u64 LE), one
/// line per function; machine code alone when it imports none
pub fn with_c_functions(functions: &[CFunction], machine_code: &[u8]) -> Vec<u8> {
	if functions.is_empty() {
		return machine_code.to_vec();
	}
	let table = functions.iter().map(CFunction::line).collect::<Vec<_>>().join("\n");
	[CARRIED_C_FUNCTIONS.as_slice(), &(table.len() as u64).to_le_bytes(), table.as_bytes(), machine_code].concat()
}

/// The C functions and the machine code `with_c_functions` joined; none for plain machine code
pub fn split_c_functions(carried: &[u8]) -> Result<(Vec<CFunction>, &[u8]), String> {
	let Some(rest) = carried.strip_prefix(CARRIED_C_FUNCTIONS.as_slice()) else {
		return Ok((vec![], carried));
	};
	let damaged = || "the executable's table of C functions is damaged".to_string();
	let (length, rest) = rest.split_first_chunk::<8>().ok_or_else(damaged)?;
	let (table, machine_code) = rest.split_at_checked(u64::from_le_bytes(*length) as usize).ok_or_else(damaged)?;
	let table = std::str::from_utf8(table).map_err(|_| damaged())?;
	let functions = table.lines().map(CFunction::of_line).collect::<Option<_>>().ok_or_else(damaged)?;
	Ok((functions, machine_code))
}
