//! Linking C functions into wasmtime: libm and the hand-linked libc functions, any library through its headers
//! (libloading + one wrapper per signature shape), and the run's handle table for C pointers

use super::*;

/// Takes turns for calls into libraries with global state that is not thread-safe (SDL_Init and SDL_Quit crash when
/// two runs of one process call them at once): programs running on several threads queue here. Thread-safe
/// libraries (libm, sqlite with its own connections) are called in parallel
static NATIVE_CALL_TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Lowercase name prefixes of the libraries whose calls take the native call turn
const NOT_THREAD_SAFE_LIBRARIES: [&str; 2] = ["sdl", "raylib"];

/// Library functions that open a native window: refused under WARP_NO_WINDOW or CI, so no test or agent run pops one up
const WINDOW_OPENERS: [&str; 4] = ["InitWindow", "SDL_CreateWindow", "SDL_CreateWindowAndRenderer", "glfwCreateWindow"];

// Extern C Functions and Hardcoded Signatures (Fallback)

// Extern C functions from libc/libm
extern "C" {
	// libm math functions
	fn fmin(x: f64, y: f64) -> f64;
	fn fmax(x: f64, y: f64) -> f64;
	fn fabs(x: f64) -> f64;
	fn floor(x: f64) -> f64;
	fn ceil(x: f64) -> f64;
	fn round(x: f64) -> f64;
	fn sqrt(x: f64) -> f64;
	fn sin(x: f64) -> f64;
	fn cos(x: f64) -> f64;
	fn tan(x: f64) -> f64;
	fn fmod(x: f64, y: f64) -> f64;
	fn pow(x: f64, y: f64) -> f64;
	fn exp(x: f64) -> f64;
	fn log(x: f64) -> f64;
	fn log10(x: f64) -> f64;
	fn asin(x: f64) -> f64;
	fn acos(x: f64) -> f64;
	fn atan(x: f64) -> f64;
	fn atan2(y: f64, x: f64) -> f64;
	fn sinh(x: f64) -> f64;
	fn cosh(x: f64) -> f64;
	fn tanh(x: f64) -> f64;
	fn hypot(x: f64, y: f64) -> f64;
	fn log2(x: f64) -> f64;
	fn trunc(x: f64) -> f64;
	fn log1p(x: f64) -> f64;
	fn expm1(x: f64) -> f64;

	// libc functions
	fn abs(x: i32) -> i32;
	fn atoi(s: *const i8) -> i32;
	fn atol(s: *const i8) -> i64;
	fn atof(s: *const i8) -> f64;
	fn strcmp(s1: *const i8, s2: *const i8) -> i32;
	fn strncmp(s1: *const i8, s2: *const i8, n: usize) -> i32;
	fn rand() -> i32;
}

/// libm functions linked under the import module "m".
/// Deliberately hand-linked FFI examples (user decision 2026-10-03, kept as marked examples): the explicit form of what
/// the header-driven path (get_signatures_from_headers + link_dynamic_library) does by reflection; keep the table.
/// link_libm uses it only when the headers declare nothing for "m" (glibc's __MATHCALL macros): headers first.
const LIBM_UNARY: [(&str, unsafe extern "C" fn(f64) -> f64); 21] = [
	("fabs", fabs),
	("floor", floor),
	("ceil", ceil),
	("round", round),
	("sqrt", sqrt),
	("sin", sin),
	("cos", cos),
	("tan", tan),
	("exp", exp),
	("log", log),
	("log10", log10),
	("asin", asin),
	("acos", acos),
	("atan", atan),
	("sinh", sinh),
	("cosh", cosh),
	("tanh", tanh),
	("log2", log2),
	("trunc", trunc),
	("log1p", log1p),
	("expm1", expm1),
];
const LIBM_BINARY: [(&str, unsafe extern "C" fn(f64, f64) -> f64); 6] =
	[("fmin", fmin), ("fmax", fmax), ("fmod", fmod), ("pow", pow), ("atan2", atan2), ("hypot", hypot)];

pub fn link_ffi_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
	link_libm(linker, engine, &libm_header_signatures())?;
	link_libc_functions(linker)
}

/// Where the libm functions of a run come from
#[derive(Debug, PartialEq)]
pub enum LibmSource {
	/// that many functions declared by the system headers
	Headers(usize),
	/// the hand-linked LIBM_UNARY/LIBM_BINARY
	Table,
}

/// The libm functions the system headers declare (none from glibc, which declares them through macros)
pub fn libm_header_signatures() -> Vec<FfiHeaderSignature> {
	crate::formats::ffi_parser::find_library_headers(LIBM)
		.iter()
		.flat_map(|header| parse_header_file(header, LIBM))
		.collect()
}

/// libm from its headers first; the hand-linked table only when they declare nothing for "m" (user decision 2026-10-03)
pub fn link_libm(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	header_signatures: &[FfiHeaderSignature],
) -> Result<LibmSource> {
	let linked = match get_or_load_library(LIBM) {
		Some(library) => header_signatures
			.iter()
			.filter(|signature| link_single_function(linker, engine, LIBM, &library, signature).is_ok())
			.count(),
		None => 0,
	};
	if linked > 0 {
		return Ok(LibmSource::Headers(linked));
	}
	link_libm_table(linker)?;
	Ok(LibmSource::Table)
}

fn link_libm_table(linker: &mut Linker<FfiState>) -> Result<()> {
	use warp_runtime::floats::canonical_nan;
	for (name, function) in LIBM_UNARY {
		linker.func_wrap("m", name, move |x: f64| unsafe { function(canonical_nan(x)) })?;
	}
	for (name, function) in LIBM_BINARY {
		linker.func_wrap("m", name, move |x: f64, y: f64| unsafe { function(canonical_nan(x), canonical_nan(y)) })?;
	}
	Ok(())
}

/// The hand-linked libc functions under the import module "c"
fn link_libc_functions(linker: &mut Linker<FfiState>) -> Result<()> {
	type Caller<'a> = wasmtime::Caller<'a, FfiState>;
	linker.func_wrap("c", "abs", |x: i32| unsafe { abs(x) })?;
	// strlen, atoi, atol and atof get a pointer to a null-terminated string in the module's memory
	linker.func_wrap("c", "strlen", |mut caller: Caller, ptr: i32| -> i64 {
		memory_bytes(&mut caller)
			.and_then(|data| data.get(ptr as usize..))
			.map_or(0, |text| text.iter().take_while(|&&byte| byte != 0).count()) as i64
	})?;
	linker.func_wrap("c", "atoi", |mut caller: Caller, ptr: i32| parse_c_text(&mut caller, ptr, atoi).unwrap_or(0))?;
	linker.func_wrap("c", "atol", |mut caller: Caller, ptr: i32| parse_c_text(&mut caller, ptr, atol).unwrap_or(0))?;
	linker
		.func_wrap("c", "atof", |mut caller: Caller, ptr: i32| parse_c_text(&mut caller, ptr, atof).unwrap_or(0.0))?;
	// strcmp(ptr1, len1, ptr2, len2) and strncmp(…, n) compare two texts of the module's memory
	linker.func_wrap("c", "strcmp", |mut caller: Caller, ptr1: i32, len1: i32, ptr2: i32, len2: i32| {
		text_pair(&mut caller, [ptr1, len1, ptr2, len2])
			.map(|(first, second)| unsafe { strcmp(first.as_ptr() as *const i8, second.as_ptr() as *const i8) })
			.unwrap_or(0)
	})?;
	linker.func_wrap("c", "strncmp", |mut caller: Caller, ptr1: i32, len1: i32, ptr2: i32, len2: i32, n: i64| {
		text_pair(&mut caller, [ptr1, len1, ptr2, len2])
			.map(|(first, second)| unsafe {
				strncmp(first.as_ptr() as *const i8, second.as_ptr() as *const i8, n as usize)
			})
			.unwrap_or(0)
	})?;
	linker.func_wrap("c", "rand", || unsafe { rand() })?;

	// Add aliases: "libm" → "m", "libc" → "c"
	// This allows both (import "m" "fmin" ...) and (import "libm" "fmin" ...)
	linker.alias_module("m", "libm")?;
	linker.alias_module("c", "libc")?;

	Ok(())
}

/// The module's linear memory, if it exports one
fn memory_bytes<'caller>(caller: &'caller mut wasmtime::Caller<'_, FfiState>) -> Option<&'caller [u8]> {
	Some(crate::host::exported_memory(caller)?.data(caller))
}

/// `parse` (atoi, atol, atof) of the null-terminated string the pointer `ptr` names, none outside the memory
fn parse_c_text<T>(
	caller: &mut wasmtime::Caller<'_, FfiState>,
	ptr: i32,
	parse: unsafe extern "C" fn(*const i8) -> T,
) -> Option<T> {
	let text = memory_bytes(caller)?.get(ptr as usize..).filter(|text| !text.is_empty())?;
	Some(unsafe { parse(text.as_ptr() as *const i8) })
}

/// The UTF-8 texts at (ptr1, len1, ptr2, len2), null-terminated for C
fn text_pair(caller: &mut wasmtime::Caller<'_, FfiState>, places: [i32; 4]) -> Option<(Vec<u8>, Vec<u8>)> {
	let data = memory_bytes(caller)?;
	let text = |index: usize| {
		let (ptr, len) = (places[index] as usize, places[index + 1] as usize);
		let bytes = data.get(ptr..ptr + len)?;
		std::str::from_utf8(bytes).ok()?;
		Some([bytes, &[0]].concat())
	};
	Some((text(0)?, text(2)?))
}

// Dynamic FFI - Load any library through reflection (raylib, SDL2, etc.)
// Parses C headers to discover signatures, loads dylib, creates wasm wrappers
// Uses direct function pointer calls with signature pattern matching

/// Global cache of loaded dynamic libraries
static LOADED_LIBRARIES: OnceLock<std::sync::Mutex<HashMap<String, Arc<Library>>>> = OnceLock::new();

/// Get or load a dynamic library
pub(crate) fn get_or_load_library(lib_name: &str) -> Option<Arc<Library>> {
	let cache = LOADED_LIBRARIES.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
	let mut guard = cache.lock().ok()?;

	if let Some(lib) = guard.get(lib_name) {
		return Some(Arc::clone(lib));
	}

	// libc is already loaded: the process itself has its symbols
	#[cfg(unix)]
	if lib_name == "c" {
		let arc = Arc::new(Library::from(libloading::os::unix::Library::this()));
		guard.insert(lib_name.to_string(), Arc::clone(&arc));
		return Some(arc);
	}

	// Try various library paths
	let paths = get_library_paths(lib_name);
	for path in paths {
		if let Ok(lib) = unsafe { Library::new(&path) } {
			let arc = Arc::new(lib);
			guard.insert(lib_name.to_string(), Arc::clone(&arc));
			return Some(arc);
		}
	}
	None
}

/// Get possible paths for a library
fn get_library_paths(lib_name: &str) -> Vec<String> {
	let mut paths = Vec::new();

	// macOS paths
	#[cfg(target_os = "macos")]
	{
		paths.push(format!("/opt/homebrew/lib/lib{}.dylib", lib_name));
		paths.push(format!("/usr/local/lib/lib{}.dylib", lib_name));
		paths.push(format!("lib{}.dylib", lib_name));
	}

	// Linux paths
	#[cfg(target_os = "linux")]
	{
		paths.push(format!("/usr/lib/lib{}.so", lib_name));
		paths.push(format!("/usr/local/lib/lib{}.so", lib_name));
		paths.push(format!("lib{}.so", lib_name));
		// glibc's libm.so and libc.so are linker scripts dlopen refuses: the libraries go by their soname
		paths.push(format!("lib{}.so.6", lib_name));
	}

	paths
}

/// Normalized parameter type for signature matching
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ParamType {
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

/// Normalized return type for signature matching
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RetType {
	Void,
	I32,
	I64,
	F32,
	F64,
	Bool,
}

/// The C type's parameter: a pointer by what it points to, else by its wasm value type (int, bool, char, short, Color: i32)
fn c_type_to_param_type(c_type: &str) -> ParamType {
	let t = bare_c_type(c_type);
	if t.contains('*') {
		return match pointer_kind(t) {
			Some(CPointer::Handle) => ParamType::Handle,
			Some(CPointer::Out) => ParamType::Out,
			_ => ParamType::Ptr,
		};
	}
	match c_type_to_wasm_valtype(t) {
		Some(ValType::F32) => ParamType::F32,
		Some(ValType::F64) => ParamType::F64,
		Some(ValType::I64) => ParamType::I64,
		_ => ParamType::I32,
	}
}

/// The C type's result: a bool by its low byte, else by its wasm value type
fn c_type_to_ret_type(c_type: &str) -> RetType {
	match c_type_to_wasm_valtype(c_type) {
		_ if bare_c_type(c_type) == "bool" => RetType::Bool,
		None => RetType::Void,
		Some(ValType::F32) => RetType::F32,
		Some(ValType::F64) => RetType::F64,
		Some(ValType::I64) => RetType::I64,
		_ => RetType::I32,
	}
}

/// The C type's wasm value type (map_c_type_to_valtype) for wasmtime
fn c_type_to_wasm_valtype(c_type: &str) -> Option<ValType> {
	map_c_type_to_valtype(c_type).map(|value_type| match value_type {
		wasm_encoder::ValType::I64 => ValType::I64,
		wasm_encoder::ValType::F32 => ValType::F32,
		wasm_encoder::ValType::F64 => ValType::F64,
		_ => ValType::I32,
	})
}

/// Link the functions of a library the module imports, as its headers declare them
pub fn link_dynamic_library(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	lib_name: &str,
	imported: &std::collections::HashSet<String>,
) -> Result<usize> {
	// the module imports under the name the program wrote (`use zlib`), the library and its headers go by their own (z)
	let import_name = lib_name;
	let lib_name = resolve_library_alias(lib_name);
	let Some(library) = get_or_load_library(lib_name) else {
		return Ok(0);
	};
	// all headers first: their struct types decide how pointers cross (is_struct_type)
	let signatures: Vec<FfiHeaderSignature> = crate::formats::ffi_parser::find_library_headers(lib_name)
		.iter()
		.flat_map(|path| parse_header_file(path, lib_name))
		.collect();
	// only what the module imports: libc's headers declare hundreds of functions
	Ok(signatures
		.iter()
		.filter(|sig| imported.contains(&sig.name))
		.filter(|sig| link_single_function(linker, engine, import_name, &library, sig).is_ok())
		.count())
}

/// Link a single function from a parsed header signature
fn link_single_function(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	lib_name: &str,
	library: &Arc<Library>,
	sig: &FfiHeaderSignature,
) -> Result<()> {
	let name = sig.name.clone();
	let func_ptr: usize = unsafe {
		let symbol: libloading::Symbol<*const ()> =
			library.get(name.as_bytes()).map_err(|e| anyhow::anyhow!("Symbol {} not found: {}", name, e))?;
		*symbol as usize
	};
	if func_ptr == 0 {
		return Err(anyhow::anyhow!("Null function pointer for {}", name));
	}
	let wasm_params: Vec<ValType> =
		warp_parameters(&sig.param_types).filter_map(|t| c_type_to_wasm_valtype(t)).collect();
	let result = pointer_result(sig);
	let wasm_results: Vec<ValType> = match result {
		Some(CPointer::Text) => vec![ValType::ANYREF],
		Some(_) => vec![ValType::I32],
		None => c_type_to_wasm_valtype(&sig.return_type).into_iter().collect(),
	};
	let call = NativeCall {
		name,
		func_ptr,
		param_types: sig.param_types.iter().map(|t| c_type_to_param_type(t)).collect(),
		result,
		ret_type: c_type_to_ret_type(&sig.return_type),
		out_pointee: sig.param_types.iter().find(|t| is_handle_out_pointer(t)).map(|t| pointee(t)),
	};
	link_native_call(linker, lib_name, FuncType::new(engine, wasm_params, wasm_results), call)
}

/// Link all dynamic libraries required by a WASM module's imports
/// Scans the module for import statements and links matching libraries via reflection
pub fn link_module_libraries(linker: &mut Linker<FfiState>, engine: &Engine, module: &wasmtime::Module) -> Result<()> {
	use std::collections::HashSet;

	// Collect unique library names from imports
	let mut libs_to_link: HashMap<String, HashSet<String>> = HashMap::new();

	for import in module.imports() {
		let module_name = import.module();
		// Skip built-in libraries that are already linked; libc beyond its built-in functions (toupper) is linked here
		let built_in_libc = matches!(module_name, "c" | "libc") && HAND_LINKED_LIBC.contains(&import.name());
		let wasm_module = crate::modules::wasm_modules::is_module_path(module_name); // linked by wasm_modules::link
		if !built_in_libc
			&& !wasm_module
			&& !matches!(module_name, "m" | "libm" | "env" | "wasi_snapshot_preview1" | crate::host::HOST_LIBRARY)
		{
			libs_to_link.entry(module_name.to_string()).or_default().insert(import.name().to_string());
		}
	}

	refuse_headless_windows(&libs_to_link)?;
	// Link each discovered library
	for (lib_name, imported) in libs_to_link {
		match link_dynamic_library(linker, engine, &lib_name, &imported) {
			Ok(count) if count > 0 => {}
			Ok(_) => {
				eprintln!("[FFI] Warning: No functions linked from {}", lib_name);
			}
			Err(e) => {
				eprintln!("[FFI] Warning: Failed to link {}: {}", lib_name, e);
			}
		}
	}

	Ok(())
}

fn refuse_headless_windows(libs_to_link: &HashMap<String, std::collections::HashSet<String>>) -> Result<()> {
	if !crate::graphics::paint::headless() {
		return Ok(());
	}
	for (lib_name, imported) in libs_to_link {
		if let Some(opener) = WINDOW_OPENERS.iter().find(|opener| imported.contains(**opener)) {
			anyhow::bail!(
				"{opener} from {lib_name} opens a window: refused under {} or CI",
				crate::graphics::paint::NO_WINDOW_VARIABLE
			);
		}
	}
	Ok(())
}

fn is_thread_safe(lib_name: &str) -> bool {
	let name = lib_name.to_lowercase();
	!NOT_THREAD_SAFE_LIBRARIES.iter().any(|prefix| name.trim_start_matches("lib").starts_with(prefix))
}

/// Link a wrapper of a library function; one of a library that is not thread-safe holds the native call turn while it runs
fn link_native(
	linker: &mut Linker<FfiState>,
	lib_name: &str,
	func_name: &str,
	func_type: FuncType,
	call: impl Fn(wasmtime::Caller<'_, FfiState>, &[Val], &mut [Val]) -> wasmtime::Result<()> + Send + Sync + 'static,
) -> Result<()> {
	if is_thread_safe(lib_name) {
		linker.func_new(lib_name, func_name, func_type, call)?;
		return Ok(());
	}
	linker.func_new(lib_name, func_name, func_type, move |caller, params, results| {
		let _turn = NATIVE_CALL_TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		call(caller, params, results)
	})?;
	Ok(())
}

/// The raw arguments of a native call: up to 8 integers and pointers, and up to 8 floats, each kind in the order of the
/// C parameters (the ARM64 and x86-64 conventions give them separate registers: `ldexp(double, int)` takes d0 and x0)
#[derive(Default)]
struct NativeArguments {
	integers: [u64; 8],
	floats: [f64; 8],
}

/// The arguments of a native call, one per C parameter: numbers as they are (a `float` in the low half of its
/// register), a memory pointer as the address of its text in linear memory, a handle id as its C pointer; the first
/// out-pointer gets `out_slot`, others NULL
fn native_arguments(
	caller: &mut wasmtime::Caller<'_, FfiState>,
	name: &str,
	params: &[Val],
	param_types: &[ParamType],
	out_slot: &mut usize,
) -> wasmtime::Result<NativeArguments> {
	let mut args = NativeArguments::default();
	let (mut integer, mut float) = (0, 0);
	let mut warp_params = params.iter();
	let mut out_slot = Some(out_slot);
	for ptype in param_types {
		let value =
			if *ptype == ParamType::Out {
				Some(out_slot.take().map_or(0, |slot| slot as *mut usize as u64))
			} else {
				let Some(param) = warp_params.next() else { break };
				match ptype {
					ParamType::I32 => Some(param.unwrap_i32() as u64),
					ParamType::I64 => Some(param.unwrap_i64() as u64),
					ParamType::Ptr => Some(get_memory_ptr(caller, param.unwrap_i32() as usize) as u64),
					ParamType::Handle => {
						let id = param.unwrap_i32();
						Some(
							caller.data().c_handles.pointer(id).ok_or_else(|| {
								wasmtime::Error::msg(format!("{name}: {id} is no C handle of this run"))
							})? as u64,
						)
					}
					ParamType::F32 | ParamType::F64 => {
						let bits = if *ptype == ParamType::F32 {
							param.unwrap_f32().to_bits() as u64
						} else {
							warp_runtime::floats::canonical_nan(param.unwrap_f64()).to_bits()
						};
						*args
							.floats
							.get_mut(float)
							.ok_or_else(|| wasmtime::Error::msg(format!("{name}: more than 8 float parameters")))? = f64::from_bits(bits);
						float += 1;
						None
					}
					ParamType::Out => unreachable!("out-pointers are filled above"),
				}
			};
		if let Some(value) = value {
			*args
				.integers
				.get_mut(integer)
				.ok_or_else(|| wasmtime::Error::msg(format!("{name}: more than 8 integer or pointer parameters")))? = value;
			integer += 1;
		}
	}
	Ok(args)
}

/// The C pointers a run got, handed to warp as ids: id n is the n-th pointer, 0 is NULL; the same pointer keeps its id.
/// The table lives in the run's HostState and goes with it: ids are never addresses, and never outlive the run
#[derive(Default)]
pub struct CHandles {
	pointers: Vec<usize>,
}

impl CHandles {
	pub fn id_of(&mut self, pointer: usize) -> i32 {
		if pointer == 0 {
			return 0;
		}
		let index = self.pointers.iter().position(|known| *known == pointer).unwrap_or_else(|| {
			self.pointers.push(pointer);
			self.pointers.len() - 1
		});
		index as i32 + 1
	}

	pub fn pointer(&self, id: i32) -> Option<usize> {
		match id {
			0 => Some(0),
			id if id > 0 => self.pointers.get(id as usize - 1).copied(),
			_ => None,
		}
	}
}

/// A C function as warp calls it: numbers in their registers, and the pointers it crosses (notes/ffi_handles.md):
/// handle parameters, an out-pointer, a text or handle result
struct NativeCall {
	name: String,
	func_ptr: usize,
	param_types: Vec<ParamType>,
	/// Text, Handle, or None for the C function's own number (ret_type)
	result: Option<CPointer>,
	ret_type: RetType,
	/// what the first out-pointer points to (`sqlite3_stmt` of `sqlite3_stmt **ppStmt`), for the error of a NULL there
	out_pointee: Option<String>,
}

/// One wrapper for every C function: its arguments go in the integer and float registers by the C calling convention
/// (call_native_function), whatever the signature. `sqlite3_column_text(stmt, i)` takes a handle and gives a text,
/// `sqlite3_open(name)` gives the handle its out-pointer received (NULL there is an error naming the C status), a
/// `char *` result is copied into the module as a warp text (NULL is ø)
fn link_native_call(
	linker: &mut Linker<FfiState>,
	lib_name: &str,
	func_type: FuncType,
	call: NativeCall,
) -> Result<()> {
	let name = call.name.clone();
	link_native(linker, lib_name, &name, func_type, move |mut caller, params, results| {
		// lives through the call: a `char **` out-pointer (strtod's endptr) is written there
		let mut out_slot: usize = 0;
		let args = native_arguments(&mut caller, &call.name, params, &call.param_types, &mut out_slot)?;
		let returned = unsafe { call_native_function(call.func_ptr, &args, call.ret_type) };
		if let Some(pointee) = &call.out_pointee {
			if out_slot == 0 {
				return Err(wasmtime::Error::msg(format!(
					"{} gave no {pointee} (C status {})",
					call.name, returned as i32
				)));
			}
		}
		let pointer = if call.out_pointee.is_some() { out_slot } else { returned as usize };
		let Some(result) = results.first_mut() else {
			return Ok(());
		};
		*result = match call.result {
			Some(CPointer::Text) => {
				// copied before anything else runs: a C string from getenv may change with the next call
				let pointer = pointer as *const std::ffi::c_char;
				let text =
					(!pointer.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(pointer) }.to_bytes().to_vec());
				text_node(&mut caller, text.as_deref()).map_err(|failure| wasmtime::Error::msg(failure.to_string()))?
			}
			Some(_) => Val::I32(caller.data_mut().c_handles.id_of(pointer)),
			None => native_result(returned, call.ret_type),
		};
		Ok(())
	})?;
	Ok(())
}

/// A C function's raw return value as the wasm value of its type
fn native_result(returned: u64, ret_type: RetType) -> Val {
	match ret_type {
		RetType::Void | RetType::I32 => Val::I32(returned as i32),
		RetType::I64 => Val::I64(returned as i64),
		RetType::F32 => Val::F32(returned as u32), // the low half of the float register
		RetType::F64 => Val::F64(returned),
		RetType::Bool => Val::I32(i32::from(returned as u8 != 0)), // a C bool is its register's low byte
	}
}

/// A warp text of `bytes` in the calling module (its memory, text heap and new_text), or ø
pub(crate) fn text_node(caller: &mut wasmtime::Caller<'_, FfiState>, bytes: Option<&[u8]>) -> Result<Val> {
	let export = |caller: &mut wasmtime::Caller<'_, FfiState>, name: &str| {
		caller.get_export(name).ok_or_else(|| anyhow::anyhow!("a C text result needs the module's {name}"))
	};
	let mut result = [Val::AnyRef(None)];
	let Some(bytes) = bytes else {
		let new_empty =
			export(caller, "new_empty")?.into_func().ok_or_else(|| anyhow::anyhow!("new_empty is no function"))?;
		new_empty.call(&mut *caller, &[], &mut result)?;
		return Ok(result[0]);
	};
	let memory = export(caller, "memory")?.into_memory().ok_or_else(|| anyhow::anyhow!("memory is no memory"))?;
	let heap = export(caller, crate::host::TEXT_HEAP_EXPORT)?
		.into_global()
		.ok_or_else(|| anyhow::anyhow!("the text heap is no global"))?;
	let new_text = export(caller, "new_text")?.into_func().ok_or_else(|| anyhow::anyhow!("new_text is no function"))?;
	let (pointer, length) =
		crate::host::write_to_heap(&memory, heap, &mut wasmtime::AsContextMut::as_context_mut(caller), bytes)?;
	new_text.call(&mut *caller, &[Val::I32(pointer as i32), Val::I32(length as i32)], &mut result)?;
	Ok(result[0])
}

/// Get pointer into WASM linear memory
fn get_memory_ptr(caller: &mut wasmtime::Caller<'_, FfiState>, offset: usize) -> *const u8 {
	if let Some(memory) = crate::host::exported_memory(caller) {
		unsafe { memory.data_ptr(&caller).add(offset) }
	} else {
		std::ptr::null()
	}
}

/// Call a native function with its integer and float arguments in their registers: a C function reads only the ones
/// its parameters name. Its result: the integer register, or the bits of the float register for a float result
#[inline(never)]
unsafe fn call_native_function(func_ptr: usize, args: &NativeArguments, ret_type: RetType) -> u64 {
	let [i0, i1, i2, i3, i4, i5, i6, i7] = args.integers;
	let [f0, f1, f2, f3, f4, f5, f6, f7] = args.floats;
	if matches!(ret_type, RetType::F32 | RetType::F64) {
		let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64, f64, f64, f64, f64, f64, f64, f64, f64) -> f64 =
			std::mem::transmute(func_ptr);
		f(i0, i1, i2, i3, i4, i5, i6, i7, f0, f1, f2, f3, f4, f5, f6, f7).to_bits()
	} else {
		let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64, f64, f64, f64, f64, f64, f64, f64, f64) -> u64 =
			std::mem::transmute(func_ptr);
		f(i0, i1, i2, i3, i4, i5, i6, i7, f0, f1, f2, f3, f4, f5, f6, f7)
	}
}
