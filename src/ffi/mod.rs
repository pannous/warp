//! Foreign functions: calls into native C libraries (libc, libm, SDL2, …).
//! header.rs reads C headers into signatures, link.rs links them into wasmtime (native only), this file looks
//! signatures up (hand-linked ones first, then the library's system headers) and says how C pointers cross
//! (notes/ffi_handles.md).

#[cfg(feature = "native")]
use anyhow::Result;
#[cfg(feature = "native")]
use libloading::Library;
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::sync::Arc;
use std::sync::OnceLock;
#[cfg(feature = "native")]
use wasmtime::{Engine, Linker};

/// libc signatures as warp calls them, over what the headers declare (or miss: abs and labs are in _stdlib.h): strcmp
/// and strncmp take (ptr, len) pairs instead of NUL-terminated pointers; (name, params, result)
const LIBC_SIGNATURES: [(&str, &[wasm_encoder::ValType], wasm_encoder::ValType); 9] = {
	use wasm_encoder::ValType::{F64, I32, I64};
	[
		("strcmp", &[I32, I32, I32, I32], I32),
		("strncmp", &[I32, I32, I32, I32, I64], I32),
		("abs", &[I32], I32),
		("labs", &[I64], I64),
		("strlen", &[I32], I64),
		("atoi", &[I32], I32),
		("atol", &[I32], I64),
		("atof", &[I32], F64),
		("rand", &[], I32),
	]
};

/// libm f64 functions linked by link_ffi_functions, with their arity: lib/math.warp's foreign lines
/// `atan2(y: float, x: float): float from 'm'` (decision foreign-signature), so headers that declare them by macros
/// (glibc __MATHCALL) don't hide them. A libm name missing there compiled to its last argument (`hypot(3, 4)` was 4):
/// the browser host takes them from Math.
pub fn libm_f64_functions() -> &'static [(&'static str, usize)] {
	static FUNCTIONS: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();
	FUNCTIONS.get_or_init(|| math_foreign_lines(LIBM).map(|line| (line.name, line.arity())).collect())
}

const MATH_SOURCE: &str = include_str!("../../lib/math.warp");
const PRELUDE_SOURCE: &str = include_str!("../../lib/prelude.warp");
/// `floor(x: number): int from warp`: a word warp computes itself, declared with its signature like a foreign one
pub const BUILTIN_LIBRARY: &str = "warp";
/// `count(xs): int from emitter`: a builtin the emitter expands in place, without a runtime function
pub const EMITTER_LIBRARY: &str = "emitter";

/// `atan2(y: float, x: float): float from 'm'`: a function's name, parameters and result as its foreign line writes them
#[derive(Clone, Copy)]
pub struct ForeignLine<'a> {
	pub name: &'a str,
	pub parameters: &'a str,
	pub result: &'a str,
}

impl<'a> ForeignLine<'a> {
	pub fn arity(&self) -> usize {
		self.defaults().count()
	}

	/// Whether a call of `arguments` values fits: the parameters without a default at least, a rest parameter
	/// (`values...`) any number more
	pub fn takes(&self, arguments: usize) -> bool {
		let required = self.defaults().filter(Option::is_none).count();
		let rest = self.parameters.contains("...");
		arguments >= required - usize::from(rest) && (rest || arguments <= self.arity())
	}

	/// Each parameter's name, `separator` of `separator: text = " "`, `values` of `values...`
	pub fn names(&self) -> impl Iterator<Item = &'a str> {
		self.parameters
			.split(',')
			.map(|parameter| parameter.split([':', '=']).next().unwrap_or_default().trim().trim_end_matches("..."))
	}

	/// Each parameter's default as written, `" "` of `separator: text = " "`; None for one without
	pub fn defaults(&self) -> impl Iterator<Item = Option<&'a str>> {
		let parameters = self.parameters.trim();
		let parameters = (!parameters.is_empty()).then_some(parameters).into_iter().flat_map(|all| all.split(','));
		parameters.map(|parameter| parameter.split_once('=').map(|(_, default)| default.trim()))
	}

	/// The type of the first parameter, `text` of `t: text`; None for one of any value
	pub fn first_type(&self) -> Option<&'a str> {
		let first = self.parameters.split(',').next()?.split('=').next()?;
		first.split_once(':').map(|(_, type_name)| type_name.trim())
	}

	/// `(x:number) -> int`, as a signature reads
	pub fn signature(&self) -> String {
		let parameters = self.parameters.replace(": ", ":");
		match self.result {
			"" => format!("({parameters})"),
			result => format!("({parameters}) -> {result}"),
		}
	}
}

/// lib/prelude.warp's `from warp` lines: the words the emitter computes as runtime functions (upper, split, map_keys …)
pub fn runtime_word_lines() -> &'static [ForeignLine<'static>] {
	static LINES: OnceLock<Vec<ForeignLine<'static>>> = OnceLock::new();
	LINES.get_or_init(|| PRELUDE_SOURCE.lines().filter_map(|line| foreign_line(line, BUILTIN_LIBRARY)).collect())
}

/// lib/prelude.warp's `from emitter` lines: the builtins the emitter expands in place (count, print, fetch …)
pub fn emitter_word_lines() -> &'static [ForeignLine<'static>] {
	static LINES: OnceLock<Vec<ForeignLine<'static>>> = OnceLock::new();
	LINES.get_or_init(|| PRELUDE_SOURCE.lines().filter_map(|line| foreign_line(line, EMITTER_LIBRARY)).collect())
}

/// Every typed builtin line of the prelude, runtime functions and words the emitter expands
pub fn typed_word_lines() -> impl Iterator<Item = &'static ForeignLine<'static>> {
	runtime_word_lines().iter().chain(emitter_word_lines())
}

/// lib/math.warp's foreign lines of one library: `'m'` (libm) or warp itself (BUILTIN_LIBRARY)
pub fn math_foreign_lines(library: &str) -> impl Iterator<Item = ForeignLine<'static>> + '_ {
	MATH_SOURCE.lines().filter_map(move |line| foreign_line(line, library))
}

/// `atan2(y: float, x: float): float from 'm'`, `floor(x: number): int from warp`: a foreign line of `library`
pub fn foreign_line<'a>(line: &'a str, library: &str) -> Option<ForeignLine<'a>> {
	let line = line.trim();
	let signature =
		line.strip_suffix(&format!(" from '{library}'")).or_else(|| line.strip_suffix(&format!(" from {library}")))?;
	let (name, rest) = signature.split_once('(')?;
	let (parameters, result) = rest.rsplit_once(')')?;
	let result = result.trim().strip_prefix(':').unwrap_or_default().trim();
	crate::warp_parser::is_plain_name(name).then_some(ForeignLine { name, parameters, result })
}

/// FFI function signature descriptor
/// Uses wasm_encoder::ValType for consistency with emitter
#[derive(Clone, Debug)]
pub struct FfiSignature {
	pub name: &'static str,
	pub library: &'static str,
	pub params: Vec<wasm_encoder::ValType>,
	pub results: Vec<wasm_encoder::ValType>,
}

impl FfiSignature {
	pub fn new(
		name: &'static str,
		library: &'static str,
		params: Vec<wasm_encoder::ValType>,
		results: Vec<wasm_encoder::ValType>,
	) -> Self {
		FfiSignature { name, library, params, results }
	}
}

/// FFI functions run in the program's one state, with the host functions and WASI
#[cfg(feature = "native")]
pub use crate::host::HostState as FfiState;

mod header;
mod header_cache;
#[cfg(feature = "native")]
mod link;
mod macros;
pub use header::*;
#[cfg(feature = "native")]
pub use link::*;

/// The import module of libm
pub const LIBM: &str = "m";

const SDL_PREFIX: &str = "SDL_";
use warp_runtime::c_signatures::HAND_LINKED_LIBC;

/// Get FFI signatures by parsing header files for a library, once per process: every name the analyzer meets is looked up here
pub fn get_signatures_from_headers(library: &str) -> &'static HashMap<String, FfiSignature> {
	static PARSED: std::sync::Mutex<Vec<(String, &'static HashMap<String, FfiSignature>)>> =
		std::sync::Mutex::new(Vec::new());
	let mut parsed = PARSED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((_, signatures)) = parsed.iter().find(|(parsed_library, _)| parsed_library == library) {
		return signatures;
	}
	let signatures: &'static HashMap<String, FfiSignature> =
		Box::leak(Box::new(parse_signatures_from_headers(library)));
	parsed.push((library.to_string(), signatures));
	signatures
}

/// The error of a call nothing resolves at `call`; a word of a standard module offers to add its `use` line
pub fn undefined_function_diagnostic(call: &crate::node::Node, name: &str) -> crate::errors::diagnostic::Diagnostic {
	let diagnostic = crate::errors::diagnostic::Diagnostic::at(call, undefined_function_message(name));
	match crate::modules::std_module_defining(name)
		.filter(|module| crate::modules::module_file_shadowing(module).is_none())
	{
		Some(module) => diagnostic.offering(crate::errors::fixits::added_first_line(
			format!("{name} from the standard module {module}"),
			format!("use {module}"),
		)),
		None => diagnostic,
	}
}

/// The error of a call of `import name from 'library'` whose headers are missing or do not declare it
pub fn unresolved_import_message(name: &str, library: &str) -> String {
	let headers = crate::formats::ffi_parser::find_library_headers(library);
	match headers.is_empty() {
		true => {
			format!("{name} is imported from {library}, but no header of {library} is found in the include directories")
		}
		false => {
			format!("{name} is imported from {library}, but its headers do not declare it: {}", headers.join(", "))
		}
	}
}

/// The error of a call nothing resolves, always "undefined function: name", then where the name is found: a standard
/// module or a libc function says how to bring it in; a free symbol used as an operator (`1 ⇔ 1`, decision
/// unicode-symbols) says how to define it
pub fn undefined_function_message(name: &str) -> String {
	if let Some(glyph) = name.strip_prefix(crate::operators::OPERATOR_FUNCTION_PREFIX) {
		return format!("operator {glyph} is not defined (define it with infix operator {glyph}(a, b) := …)");
	}
	match where_undefined_function_is(name) {
		Some(hint) => format!("undefined function: {name} ({hint})"),
		None => format!("undefined function: {name}"),
	}
}

fn where_undefined_function_is(name: &str) -> Option<String> {
	if let Some(module) = crate::modules::std_module_defining(name) {
		if let Some(file) = crate::modules::module_file_shadowing(module) {
			return Some(format!("{name} is in the standard module {module}, but `use {module}` finds the file {} first, which has no {name}: rename that file", file.display()));
		}
		return Some(format!("{name} is in the standard module {module}: write `use {module}`"));
	}
	if let Some(c_type) = unlinkable_header_function(name) {
		return Some(format!(
			"{name} is a C function using {c_type}, which wasm has no type for: no warp program can call it"
		));
	}
	if get_ffi_signature_from_lib(name, "c").is_some() {
		return Some(format!("{name} is a C function: write `use c` or `import {name} from \"c\"`"));
	}
	// only f64 functions link by themselves (analyzer imports.rs is_f64_header_function): an int parameter may be a
	// pointer (frexp's int *), which the import's author knows
	get_ffi_signature_from_lib(name, "m").map(|_| {
		format!("{name} is a libm function with a parameter other than a float: write `import {name} from \"m\"`")
	})
}

/// The type wasm lacks of a libc or libm function its headers declare (`sqrtl`: long double); headers read again,
/// only for the error
fn unlinkable_header_function(name: &str) -> Option<&'static str> {
	implicit_header_libraries(name)
		.iter()
		.flat_map(|library| get_library_header_paths(library).into_iter().map(move |path| (path, library)))
		.flat_map(|(path, library)| parse_header_file(&path, library))
		.find(|declared| declared.name == name)
		.and_then(|declared| unlinkable_type(&declared))
}

/// What a C pointer type crosses as (notes/ffi_handles.md); the C function descriptions executables carry
pub use warp_runtime::c_signatures::{CFunction, CPointer, ParamType, RetType};

/// C words that qualify a type without naming it
const C_QUALIFIERS: [&str; 7] = ["const", "volatile", "struct", "enum", "union", "unsigned", "signed"];

/// The type a C pointer type points to: the last word before its `*` (`char` of `SQLITE_API const unsigned char *`,
/// `FILE` of `FILE *`), "" for none (`unsigned *`)
fn pointee(c_type: &str) -> String {
	let before_star = &c_type[..c_type.find('*').unwrap_or(c_type.len())];
	before_star.split_whitespace().rfind(|word| !C_QUALIFIERS.contains(word)).unwrap_or_default().to_string()
}

/// How the C type `c_type` crosses, None for no pointer. A handle points to a struct (`struct stat *`, or one a header
/// declares: `sqlite3 *`, `FILE *`); a pointer to anything else (`Bytef *`, `void *`) is memory, as before handles
pub fn pointer_kind(c_type: &str) -> Option<CPointer> {
	let names_struct = || c_type.split_whitespace().any(|word| word == "struct") || is_struct_type(&pointee(c_type));
	match c_type.matches('*').count() {
		0 => None,
		1 if pointee(c_type) == "char" => Some(CPointer::Text),
		1 if names_struct() => Some(CPointer::Handle),
		1 => Some(CPointer::Memory),
		_ => Some(CPointer::Out),
	}
}

/// An out-pointer whose filled-in pointer is the call's result, a handle: `sqlite3 **ppDb`; a `char **` (strtod's endptr)
/// is not one, the C function's own result stays
pub(crate) fn is_handle_out_pointer(c_type: &str) -> bool {
	pointer_kind(c_type) == Some(CPointer::Out) && pointee(c_type) != "char"
}

/// What a `const T *` points to when a `size_t` after it says how many bytes: `const uint8_t *module, size_t length`
const BYTE_TYPES: [&str; 5] = ["uint8_t", "int8_t", "char", "void", "Bytef"];
const LENGTH_TYPE: &str = "size_t";

/// Whether the C parameter at `index` takes bytes with their length in the next one: a const single pointer to bytes,
/// then a size_t
fn takes_bytes(c_types: &[String], index: usize) -> bool {
	let c_type = &c_types[index];
	c_type.matches('*').count() == 1
		&& c_type.split_whitespace().any(|word| word == "const")
		&& BYTE_TYPES.contains(&pointee(c_type).as_str())
		&& c_types.get(index + 1).is_some_and(|next| bare_c_type(next) == LENGTH_TYPE)
}

/// How each C parameter crosses: a pointer by what it points to, bytes with their length, else by its wasm value type
/// (int, bool, char, short, Color: i32)
pub fn c_param_types(c_types: &[String]) -> Vec<ParamType> {
	let mut types: Vec<ParamType> = vec![];
	for (index, c_type) in c_types.iter().enumerate() {
		types.push(match types.last() {
			Some(ParamType::Bytes) => ParamType::Length,
			_ if takes_bytes(c_types, index) => ParamType::Bytes,
			_ => c_type_to_param_type(c_type),
		});
	}
	types
}

/// The C parameters a warp call passes, with how they cross: all but the out-pointers and lengths
fn warp_parameters(c_types: &[String]) -> impl Iterator<Item = (&String, ParamType)> {
	c_types.iter().zip(c_param_types(c_types)).filter(|(_, param)| param.is_passed())
}

/// The wasm type a passed parameter has in the warp call: bytes are a value (a text, a list), pointers and handles i32
fn param_valtype(param: ParamType) -> Option<wasm_encoder::ValType> {
	match param {
		ParamType::Bytes => Some(wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)),
		ParamType::I64 => Some(wasm_encoder::ValType::I64),
		ParamType::F32 => Some(wasm_encoder::ValType::F32),
		ParamType::F64 => Some(wasm_encoder::ValType::F64),
		ParamType::Out | ParamType::Length => None,
		ParamType::I32 | ParamType::Ptr | ParamType::Handle => Some(wasm_encoder::ValType::I32),
	}
}

/// What a C function's result crosses as: a text, a handle (a pointer result or the first out-pointer), else None for
/// its number
pub(crate) fn pointer_result(hsig: &FfiHeaderSignature) -> Option<CPointer> {
	let has_out_pointer = hsig.param_types.iter().any(|t| is_handle_out_pointer(t));
	match pointer_kind(&hsig.return_type) {
		Some(CPointer::Text) => Some(CPointer::Text),
		Some(_) => Some(CPointer::Handle),
		None => has_out_pointer.then_some(CPointer::Handle),
	}
}

/// The C type's parameter: a pointer by what it points to, else by its wasm value type (int, bool, char, short, Color: i32)
fn c_type_to_param_type(c_type: &str) -> ParamType {
	let t = bare_c_type(c_type);
	match (pointer_kind(t), map_c_type_to_valtype(t)) {
		(Some(CPointer::Handle), _) => ParamType::Handle,
		(Some(CPointer::Out), _) => ParamType::Out,
		(Some(_), _) => ParamType::Ptr,
		(None, Some(wasm_encoder::ValType::F32)) => ParamType::F32,
		(None, Some(wasm_encoder::ValType::F64)) => ParamType::F64,
		(None, Some(wasm_encoder::ValType::I64)) => ParamType::I64,
		(None, _) => ParamType::I32,
	}
}

/// The C type's result: a bool by its low byte, else by its wasm value type
fn c_type_to_ret_type(c_type: &str) -> RetType {
	match map_c_type_to_valtype(c_type) {
		_ if bare_c_type(c_type) == "bool" => RetType::Bool,
		None => RetType::Void,
		Some(wasm_encoder::ValType::F32) => RetType::F32,
		Some(wasm_encoder::ValType::F64) => RetType::F64,
		Some(wasm_encoder::ValType::I64) => RetType::I64,
		Some(_) => RetType::I32,
	}
}

/// The header declaration `hsig` as a C function imported from `module` (`zlib`, whose library is `z`)
pub fn c_function_of(module: &str, hsig: &FfiHeaderSignature) -> CFunction {
	CFunction {
		module: module.to_string(),
		library: resolve_library_alias(module).to_string(),
		name: hsig.name.clone(),
		param_types: c_param_types(&hsig.param_types),
		result: pointer_result(hsig),
		ret_type: c_type_to_ret_type(&hsig.return_type),
		out_pointee: hsig.param_types.iter().find(|t| is_handle_out_pointer(t)).map(|t| pointee(t)),
	}
}

/// The functions of `imported` that the headers of `module`'s library declare, as C functions imported from `module`
pub fn header_c_functions(module: &str, imported: &std::collections::HashSet<String>) -> Vec<CFunction> {
	let library = resolve_library_alias(module);
	// all headers first: their struct types decide how pointers cross (is_struct_type)
	let signatures: Vec<FfiHeaderSignature> = crate::formats::ffi_parser::find_library_headers(library)
		.iter()
		.flat_map(|path| parse_header_file(path, library))
		.collect();
	// only what the module imports, once (ctype.h declares toupper twice): libc's headers declare hundreds of functions
	let mut seen = std::collections::HashSet::new();
	signatures
		.iter()
		.filter(|hsig| imported.contains(&hsig.name) && seen.insert(hsig.name.as_str()))
		.map(|hsig| c_function_of(module, hsig))
		.collect()
}

/// The wasm result of a C function: a text Node, a handle id, else its number
fn header_result(hsig: &FfiHeaderSignature) -> Option<wasm_encoder::ValType> {
	match pointer_result(hsig) {
		Some(CPointer::Text) => Some(wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)),
		Some(_) => Some(wasm_encoder::ValType::I32),
		None => map_c_type_to_valtype(&hsig.return_type),
	}
}

/// The C functions whose texts cross as (pointer, length) pairs, not NUL-terminated: that many leading pairs
pub fn string_pair_count(name: &str) -> usize {
	match name {
		"strcmp" | "strncmp" => 2,
		_ => 0,
	}
}

/// Which parameters of a library's C functions are texts (`char *`), as its headers declare them, by function; read
/// once per library and process
pub fn header_text_parameters(library: &str) -> &'static HashMap<String, Vec<bool>> {
	type TextParameters = HashMap<String, Vec<bool>>;
	static PARSED: std::sync::Mutex<Vec<(String, &'static TextParameters)>> = std::sync::Mutex::new(Vec::new());
	let mut parsed = PARSED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((_, texts)) = parsed.iter().find(|(parsed_library, _)| parsed_library == library) {
		return texts;
	}
	let texts = get_library_header_paths(library)
		.iter()
		.flat_map(|path| parse_header_file(path, library))
		.map(|declared| {
			(
				declared.name.clone(),
				warp_parameters(&declared.param_types)
					.map(|(c_type, param)| param == ParamType::Ptr && pointer_kind(c_type) == Some(CPointer::Text))
					.collect(),
			)
		})
		.collect();
	let texts: &'static HashMap<String, Vec<bool>> = Box::leak(Box::new(texts));
	parsed.push((library.to_string(), texts));
	texts
}

fn parse_signatures_from_headers(library: &str) -> HashMap<String, FfiSignature> {
	let headers = get_library_header_paths(library);
	header_cache::header_signatures(library, &headers, || {
		// all headers first: their struct types decide how pointers cross (is_struct_type)
		let header_sigs: Vec<FfiHeaderSignature> =
			headers.iter().flat_map(|path| parse_header_file(path, library)).collect();
		header_sigs.iter().filter_map(|hsig| Some((hsig.name.clone(), header_sig_to_ffi_sig(hsig)?))).collect()
	})
}

/// The signatures warp links by hand, once per process: the libc functions it adapts (strcmp of two texts), libm's f64
/// functions, `ln`, the host words and the linear-array words. Every other C function is as its header declares it
pub fn get_ffi_signatures() -> &'static HashMap<String, FfiSignature> {
	static PARSED: OnceLock<HashMap<String, FfiSignature>> = OnceLock::new();
	PARSED.get_or_init(hand_linked_signatures)
}

fn hand_linked_signatures() -> HashMap<String, FfiSignature> {
	let mut sigs = HashMap::new();
	for (name, params, result) in LIBC_SIGNATURES {
		sigs.insert(name.to_string(), FfiSignature::new(name, "c", params.to_vec(), vec![result]));
	}
	for &(name, arity) in libm_f64_functions() {
		sigs.entry(name.to_string()).or_insert_with(|| {
			FfiSignature::new(name, "m", vec![wasm_encoder::ValType::F64; arity], vec![wasm_encoder::ValType::F64])
		});
	}
	// `ln(x)` is the natural logarithm: libm's log under the name the program calls
	sigs.insert(
		"ln".to_string(),
		FfiSignature::new("log", "m", vec![wasm_encoder::ValType::F64], vec![wasm_encoder::ValType::F64]),
	);
	for (name, params, results) in crate::host::HOST_WORD_SIGNATURES {
		sigs.insert(
			name.to_string(),
			FfiSignature::new(name, crate::host::HOST_LIBRARY, params.to_vec(), results.to_vec()),
		);
	}
	use crate::wasm_emitter::lists::linear_arrays::{linear_word_signatures, LINEAR_LIBRARY};
	for (name, params, results) in linear_word_signatures() {
		sigs.insert(name.to_string(), FfiSignature::new(name, LINEAR_LIBRARY, params, results));
	}

	sigs
}

/// Check if a function is a known FFI function
pub fn is_ffi_function(name: &str) -> bool {
	get_ffi_signature(name).is_some()
}

/// Get FFI signature for a function by name
/// Searches well-known libraries (m, c, SDL2) via dynamic header discovery
pub fn get_ffi_signature(name: &str) -> Option<FfiSignature> {
	// the hand-linked signatures first (warp's own `random` is no libc random), then the headers
	get_ffi_signatures().get(name).cloned().or_else(|| {
		implicit_header_libraries(name)
			.iter()
			.find_map(|library| get_signatures_from_headers(library).get(name).cloned())
	})
}

/// The libraries whose headers an undeclared call `name(…)` is looked up in: libm and libc, SDL2 only for SDL_ names,
/// so a program that never names SDL never reads its headers
pub fn implicit_header_libraries(name: &str) -> &'static [&'static str] {
	if name.starts_with(SDL_PREFIX) {
		&["m", "c", "SDL2"]
	} else {
		&["m", "c"]
	}
}

/// Get FFI signature from a specific library's headers
pub fn get_ffi_signature_from_lib(name: &str, library: &str) -> Option<FfiSignature> {
	let library = resolve_library_alias(library);
	let built_in = get_ffi_signatures().get(name).filter(|sig| sig.library == library).cloned();
	// the hand-linked libc functions and libm keep their wasm-adapted signatures (strcmp of two texts)
	if (library == "m" || (library == "c" && HAND_LINKED_LIBC.contains(&name))) && built_in.is_some() {
		return built_in;
	}
	// any other function is linked from its header declaration (link_single_function), so it is imported as declared
	get_signatures_from_headers(library).get(name).cloned().or(built_in)
}

/// Resolve library alias to canonical name
pub fn resolve_library_alias(alias: &str) -> &'static str {
	match alias {
		"m" | "math" | "cmath" | "libm" => "m",
		"c" | "libc" => "c",
		"SDL2" | "sdl2" | "sdl" => "SDL2",
		"z" | "zlib" => "z",
		_ => interned(alias.strip_prefix("lib").unwrap_or(alias)),
	}
}

/// `name` with a static lifetime, leaked once per distinct name (the analyzer asks for every `use`d word)
fn interned(name: &str) -> &'static str {
	static NAMES: OnceLock<std::sync::Mutex<std::collections::HashSet<&'static str>>> = OnceLock::new();
	let mut names = NAMES.get_or_init(Default::default).lock().unwrap();
	match names.get(name) {
		Some(known) => known,
		None => {
			let leaked: &'static str = Box::leak(name.into());
			names.insert(leaked);
			leaked
		}
	}
}

/// Check if a library has discoverable headers
/// Returns true for well-known libraries or any library with headers found on filesystem
pub fn is_ffi_library(lib: &str) -> bool {
	let canonical = resolve_library_alias(lib);
	// Well-known libraries
	if matches!(canonical, "m" | "c" | "SDL2") {
		return true;
	}
	// Dynamic discovery - check if headers exist
	!get_library_header_paths(canonical).is_empty()
}
