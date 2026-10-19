//! Calling C from a program, for a store of any state that keeps the run's C handles: loading libraries, the
//! hand-linked libc functions, and one wrapper for every C function as c_signatures.rs describes it. warp links these
//! after reading the headers (src/ffi/link.rs); an executable links the functions it carries (standalone.rs)
use crate::c_signatures::{CFunction, CPointer, ParamType, RetType, HAND_LINKED_LIBC};
use libloading::Library;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use wasmtime::{Caller, Engine, FuncType, Linker, Result, Val, ValType};

/// The import module of libc
pub const LIBC: &str = "c";
/// The other names programs import libc and libm under: (alias, module)
const MODULE_ALIASES: [(&str, &str); 2] = [("libm", crate::libm::LIBM), ("libc", LIBC)];

/// Takes turns for calls into libraries with global state that is not thread-safe (SDL_Init and SDL_Quit crash when
/// two runs of one process call them at once): programs running on several threads queue here. Thread-safe
/// libraries (libm, sqlite with its own connections) are called in parallel
static NATIVE_CALL_TURN: Mutex<()> = Mutex::new(());

/// Lowercase name prefixes of the libraries whose calls take the native call turn
const NOT_THREAD_SAFE_LIBRARIES: [&str; 2] = ["sdl", "raylib"];

/// The state of a store whose program calls C: the run's handle table
pub trait CallsC: 'static {
	fn c_handles(&mut self) -> &mut CHandles;
}

impl CallsC for CHandles {
	fn c_handles(&mut self) -> &mut CHandles {
		self
	}
}

extern "C" {
	fn abs(x: i32) -> i32;
	fn atoi(s: *const i8) -> i32;
	fn atol(s: *const i8) -> i64;
	fn atof(s: *const i8) -> f64;
	fn strcmp(s1: *const i8, s2: *const i8) -> i32;
	fn strncmp(s1: *const i8, s2: *const i8, n: usize) -> i32;
	fn rand() -> i32;
}

/// The hand-linked libc functions under the import module "c", and the modules "libm" and "libc" as aliases
pub fn link_libc_functions<T: CallsC>(linker: &mut Linker<T>) -> Result<()> {
	linker.func_wrap(LIBC, "abs", |x: i32| unsafe { abs(x) })?;
	// strlen, atoi, atol and atof get a pointer to a null-terminated string in the module's memory
	linker.func_wrap(LIBC, "strlen", |mut caller: Caller<'_, T>, ptr: i32| -> i64 {
		memory_bytes(&mut caller)
			.and_then(|data| data.get(ptr as usize..))
			.map_or(0, |text| text.iter().take_while(|&&byte| byte != 0).count()) as i64
	})?;
	linker.func_wrap(LIBC, "atoi", |mut caller: Caller<'_, T>, ptr: i32| {
		parse_c_text(&mut caller, ptr, atoi).unwrap_or(0)
	})?;
	linker.func_wrap(LIBC, "atol", |mut caller: Caller<'_, T>, ptr: i32| {
		parse_c_text(&mut caller, ptr, atol).unwrap_or(0)
	})?;
	linker.func_wrap(LIBC, "atof", |mut caller: Caller<'_, T>, ptr: i32| {
		parse_c_text(&mut caller, ptr, atof).unwrap_or(0.0)
	})?;
	// strcmp(ptr1, len1, ptr2, len2) and strncmp(…, n) compare two texts of the module's memory
	linker.func_wrap(LIBC, "strcmp", |mut caller: Caller<'_, T>, ptr1: i32, len1: i32, ptr2: i32, len2: i32| {
		text_pair(&mut caller, [ptr1, len1, ptr2, len2])
			.map(|(first, second)| unsafe { strcmp(first.as_ptr() as *const i8, second.as_ptr() as *const i8) })
			.unwrap_or(0)
	})?;
	linker.func_wrap(
		LIBC,
		"strncmp",
		|mut caller: Caller<'_, T>, ptr1: i32, len1: i32, ptr2: i32, len2: i32, n: i64| {
			text_pair(&mut caller, [ptr1, len1, ptr2, len2])
				.map(|(first, second)| unsafe {
					strncmp(first.as_ptr() as *const i8, second.as_ptr() as *const i8, n as usize)
				})
				.unwrap_or(0)
		},
	)?;
	linker.func_wrap(LIBC, "rand", || unsafe { rand() })?;
	for (alias, module) in MODULE_ALIASES {
		linker.alias_module(module, alias)?;
	}
	Ok(())
}

/// The imports link_libc_functions provides: (module, name)
pub fn libc_names() -> impl Iterator<Item = (&'static str, &'static str)> {
	let libm = crate::libm::libm_names().map(|name| ("libm", name));
	[LIBC, "libc"].into_iter().flat_map(|module| HAND_LINKED_LIBC.map(|name| (module, name))).chain(libm)
}

/// The module's linear memory, if it exports one
fn memory_bytes<'caller, T>(caller: &'caller mut Caller<'_, T>) -> Option<&'caller [u8]> {
	Some(caller.get_export(crate::gc_values::MEMORY)?.into_memory()?.data(caller))
}

/// `parse` (atoi, atol, atof) of the null-terminated string the pointer `ptr` names, none outside the memory
fn parse_c_text<T, R>(caller: &mut Caller<'_, T>, ptr: i32, parse: unsafe extern "C" fn(*const i8) -> R) -> Option<R> {
	let text = memory_bytes(caller)?.get(ptr as usize..).filter(|text| !text.is_empty())?;
	Some(unsafe { parse(text.as_ptr() as *const i8) })
}

/// The UTF-8 texts at (ptr1, len1, ptr2, len2), null-terminated for C
fn text_pair<T>(caller: &mut Caller<'_, T>, places: [i32; 4]) -> Option<(Vec<u8>, Vec<u8>)> {
	let data = memory_bytes(caller)?;
	let text = |index: usize| {
		let (ptr, len) = (places[index] as usize, places[index + 1] as usize);
		let bytes = data.get(ptr..ptr + len)?;
		std::str::from_utf8(bytes).ok()?;
		Some([bytes, &[0]].concat())
	};
	Some((text(0)?, text(2)?))
}

/// The libraries loaded so far, by name
static LOADED_LIBRARIES: OnceLock<Mutex<HashMap<String, Arc<Library>>>> = OnceLock::new();

/// The library `name` (`m`, `sqlite3`, `SDL2`), loaded once per process; libc is the process itself
pub fn get_or_load_library(name: &str) -> Option<Arc<Library>> {
	let mut loaded = LOADED_LIBRARIES.get_or_init(Default::default).lock().ok()?;
	if let Some(library) = loaded.get(name) {
		return Some(Arc::clone(library));
	}
	#[cfg(unix)]
	let this_process = (name == LIBC).then(|| Library::from(libloading::os::unix::Library::this()));
	#[cfg(not(unix))]
	let this_process = None;
	let library = this_process
		.or_else(|| library_paths(name).iter().find_map(|path| unsafe { Library::new(path) }.ok()))
		.map(Arc::new)?;
	loaded.insert(name.to_string(), Arc::clone(&library));
	Some(library)
}

/// Where the library `name` may be
fn library_paths(name: &str) -> Vec<String> {
	#[cfg(target_os = "macos")]
	return vec![
		format!("/opt/homebrew/lib/lib{name}.dylib"),
		format!("/usr/local/lib/lib{name}.dylib"),
		format!("lib{name}.dylib"),
	];
	#[cfg(target_os = "linux")]
	return vec![
		format!("/usr/lib/lib{name}.so"),
		format!("/usr/local/lib/lib{name}.so"),
		format!("lib{name}.so"),
		// glibc's libm.so and libc.so are linker scripts dlopen refuses: the libraries go by their soname
		format!("lib{name}.so.6"),
	];
	#[cfg(not(any(target_os = "macos", target_os = "linux")))]
	return vec![format!("{name}.dll")];
}

/// The address of `function` in its loaded library
pub fn function_pointer(library: &Library, function: &str) -> Result<usize> {
	let pointer = unsafe {
		let symbol: libloading::Symbol<*const ()> = library
			.get(function.as_bytes())
			.map_err(|failure| wasmtime::Error::msg(format!("symbol {function} not found: {failure}")))?;
		*symbol as usize
	};
	match pointer {
		0 => Err(wasmtime::Error::msg(format!("null function pointer for {function}"))),
		pointer => Ok(pointer),
	}
}

/// Load `function`'s library and link it: what an executable does for each C function it carries
pub fn link_carried_function<T: CallsC>(linker: &mut Linker<T>, engine: &Engine, function: &CFunction) -> Result<()> {
	let library = get_or_load_library(&function.library).ok_or_else(|| {
		wasmtime::Error::msg(format!("{}: cannot load the C library {}", function.name, function.library))
	})?;
	let pointer = function_pointer(&library, &function.name)?;
	link_c_function(linker, engine, function.clone(), pointer)
}

/// The wasm type of a parameter as the warp call passes it: none for an out-pointer
fn wasm_param(param: ParamType) -> Option<ValType> {
	match param {
		ParamType::I64 => Some(ValType::I64),
		ParamType::F32 => Some(ValType::F32),
		ParamType::F64 => Some(ValType::F64),
		ParamType::Out => None,
		ParamType::I32 | ParamType::Ptr | ParamType::Handle => Some(ValType::I32),
	}
}

/// The wasm results of a C function: a text node, a handle id, else its number
fn wasm_results(function: &CFunction) -> Vec<ValType> {
	match (function.result, function.ret_type) {
		(Some(CPointer::Text), _) => vec![ValType::ANYREF],
		(Some(_), _) => vec![ValType::I32],
		(None, RetType::Void) => vec![],
		(None, RetType::I64) => vec![ValType::I64],
		(None, RetType::F32) => vec![ValType::F32],
		(None, RetType::F64) => vec![ValType::F64],
		(None, RetType::I32 | RetType::Bool) => vec![ValType::I32],
	}
}

fn is_thread_safe(library: &str) -> bool {
	let name = library.to_lowercase();
	!NOT_THREAD_SAFE_LIBRARIES.iter().any(|prefix| name.trim_start_matches("lib").starts_with(prefix))
}

/// One wrapper for every C function at `func_ptr`: its arguments go in the integer and float registers by the C calling
/// convention (call_native_function), whatever the signature. `sqlite3_column_text(stmt, i)` takes a handle and gives
/// a text, `sqlite3_open(name)` gives the handle its out-pointer received (NULL there is an error naming the C status),
/// a `char *` result is copied into the module as a warp text (NULL is ø). One of a library that is not thread-safe
/// holds the native call turn while it runs
pub fn link_c_function<T: CallsC>(
	linker: &mut Linker<T>,
	engine: &Engine,
	function: CFunction,
	func_ptr: usize,
) -> Result<()> {
	let params: Vec<ValType> = function.param_types.iter().filter_map(|param| wasm_param(*param)).collect();
	let func_type = FuncType::new(engine, params, wasm_results(&function));
	let (module, name) = (function.module.clone(), function.name.clone());
	let thread_safe = is_thread_safe(&function.library);
	linker.func_new(&module, &name, func_type, move |mut caller, params, results| {
		let _turn = (!thread_safe).then(|| NATIVE_CALL_TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
		call_c_function(&mut caller, &function, func_ptr, params, results)
	})?;
	Ok(())
}

fn call_c_function<T: CallsC>(
	caller: &mut Caller<'_, T>,
	function: &CFunction,
	func_ptr: usize,
	params: &[Val],
	results: &mut [Val],
) -> Result<()> {
	// lives through the call: a `char **` out-pointer (strtod's endptr) is written there
	let mut out_slot: usize = 0;
	let args = native_arguments(caller, &function.name, params, &function.param_types, &mut out_slot)?;
	let returned = unsafe { call_native_function(func_ptr, &args, function.ret_type) };
	if let Some(pointee) = &function.out_pointee {
		if out_slot == 0 {
			wasmtime::bail!("{} gave no {pointee} (C status {})", function.name, returned as i32);
		}
	}
	let pointer = if function.out_pointee.is_some() { out_slot } else { returned as usize };
	let Some(result) = results.first_mut() else {
		return Ok(());
	};
	*result = match function.result {
		Some(CPointer::Text) => {
			// copied before anything else runs: a C string from getenv may change with the next call
			let pointer = pointer as *const std::ffi::c_char;
			let text = (!pointer.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(pointer) }.to_bytes().to_vec());
			text_node(caller, text.as_deref())?
		}
		Some(_) => Val::I32(caller.data_mut().c_handles().id_of(pointer)),
		None => native_result(returned, function.ret_type),
	};
	Ok(())
}

/// A warp text of `bytes` in the calling module (its memory, text heap and new_text), or ø
pub fn text_node<T>(caller: &mut Caller<'_, T>, bytes: Option<&[u8]>) -> Result<Val> {
	let builders = crate::gc_values::Builders::of(&mut |name| caller.get_export(name))?;
	builders.text_node(bytes, &mut wasmtime::AsContextMut::as_context_mut(caller))
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
fn native_arguments<T: CallsC>(
	caller: &mut Caller<'_, T>,
	name: &str,
	params: &[Val],
	param_types: &[ParamType],
	out_slot: &mut usize,
) -> Result<NativeArguments> {
	let mut args = NativeArguments::default();
	let (mut integer, mut float) = (0, 0);
	let mut warp_params = params.iter();
	let mut out_slot = Some(out_slot);
	for ptype in param_types {
		let value = if *ptype == ParamType::Out {
			Some(out_slot.take().map_or(0, |slot| slot as *mut usize as u64))
		} else {
			let Some(param) = warp_params.next() else { break };
			match ptype {
				ParamType::I32 => Some(param.unwrap_i32() as u64),
				ParamType::I64 => Some(param.unwrap_i64() as u64),
				ParamType::Ptr => Some(memory_pointer(caller, param.unwrap_i32() as usize) as u64),
				ParamType::Handle => {
					let id = param.unwrap_i32();
					let pointer = caller.data_mut().c_handles().pointer(id);
					Some(
						pointer
							.ok_or_else(|| wasmtime::Error::msg(format!("{name}: {id} is no C handle of this run")))?
							as u64,
					)
				}
				ParamType::F32 | ParamType::F64 => {
					let bits = if *ptype == ParamType::F32 {
						param.unwrap_f32().to_bits() as u64
					} else {
						crate::floats::canonical_nan(param.unwrap_f64()).to_bits()
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

/// The address of `offset` in the module's linear memory, NULL without one
fn memory_pointer<T>(caller: &mut Caller<'_, T>, offset: usize) -> *const u8 {
	match caller.get_export(crate::gc_values::MEMORY).and_then(|export| export.into_memory()) {
		Some(memory) => unsafe { memory.data_ptr(&caller).add(offset) },
		None => std::ptr::null(),
	}
}

/// The C pointers a run got, handed to warp as ids: id n is the n-th pointer, 0 is NULL; the same pointer keeps its id.
/// The table lives in the run's state and goes with it: ids are never addresses, and never outlive the run
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
