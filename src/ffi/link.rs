//! Linking C functions into wasmtime: libm and the hand-linked libc functions, any library through its headers
//! (libloading + one wrapper per signature shape), and the run's handle table for C pointers

#[cfg(feature = "native")]
use super::*;

// Extern C Functions and Hardcoded Signatures (Fallback)

#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
const LIBM_UNARY: [(&str, unsafe extern "C" fn(f64) -> f64); 11] = [("fabs", fabs), ("floor", floor), ("ceil", ceil), ("round", round), ("sqrt", sqrt), ("sin", sin), ("cos", cos), ("tan", tan), ("exp", exp), ("log", log), ("log10", log10)];
#[cfg(feature = "native")]
const LIBM_BINARY: [(&str, unsafe extern "C" fn(f64, f64) -> f64); 4] = [("fmin", fmin), ("fmax", fmax), ("fmod", fmod), ("pow", pow)];

#[cfg(feature = "native")]
pub fn link_ffi_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
    link_libm(linker, engine, &libm_header_signatures())?;
    link_libc_functions(linker, engine)
}

/// The import module of libm
pub const LIBM: &str = "m";

/// Where the libm functions of a run come from
#[derive(Debug, PartialEq)]
pub enum LibmSource {
    /// that many functions declared by the system headers
    Headers(usize),
    /// the hand-linked LIBM_UNARY/LIBM_BINARY
    Table,
}

/// The libm functions the system headers declare (none from glibc, which declares them through macros)
#[cfg(feature = "native")]
pub fn libm_header_signatures() -> Vec<FfiHeaderSignature> {
    crate::ffi_parser::find_library_headers(LIBM).iter().flat_map(|header| parse_header_file(header, LIBM)).collect()
}

/// libm from its headers first; the hand-linked table only when they declare nothing for "m" (user decision 2026-10-03)
#[cfg(feature = "native")]
pub fn link_libm(linker: &mut Linker<FfiState>, engine: &Engine, header_signatures: &[FfiHeaderSignature]) -> Result<LibmSource> {
    let linked = match get_or_load_library(LIBM) {
        Some(library) => header_signatures.iter().filter(|signature| link_single_function(linker, engine, LIBM, &library, signature).is_ok()).count(),
        None => 0,
    };
    if linked > 0 {
        return Ok(LibmSource::Headers(linked));
    }
    link_libm_table(linker, engine)?;
    Ok(LibmSource::Table)
}

#[cfg(feature = "native")]
fn link_libm_table(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
    use wasmtime::ValType;

    for (name, function) in LIBM_UNARY {
        let unary = FuncType::new(engine, [ValType::F64], [ValType::F64]);
        linker.func_new("m", name, unary, move |_caller, params, results| {
            results[0] = Val::F64(unsafe { function(params[0].unwrap_f64()) }.to_bits());
            Ok(())
        })?;
    }
    for (name, function) in LIBM_BINARY {
        let binary = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
        linker.func_new("m", name, binary, move |_caller, params, results| {
            results[0] = Val::F64(unsafe { function(params[0].unwrap_f64(), params[1].unwrap_f64()) }.to_bits());
            Ok(())
        })?;
    }
    Ok(())
}

/// The hand-linked libc functions under the import module "c"
#[cfg(feature = "native")]
fn link_libc_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
    use wasmtime::ValType;

    // libc: abs(i32) -> i32
    let abs_type = FuncType::new(engine, [ValType::I32], [ValType::I32]);
    linker.func_new("c", "abs", abs_type, |_caller, params, results| {
        let x = params[0].unwrap_i32();
        results[0] = Val::I32(unsafe { abs(x) });
        Ok(())
    })?;

    // libc: strlen(ptr) -> i64
    // We receive ptr to null-terminated string in WASM memory
    let strlen_type = FuncType::new(engine, [ValType::I32], [ValType::I64]);
    linker.func_new("c", "strlen", strlen_type, |mut caller, params, results| {
        let ptr = params[0].unwrap_i32() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr < data.len() {
                // Find null terminator and calculate length
                let mut len = 0usize;
                while ptr + len < data.len() && data[ptr + len] != 0 {
                    len += 1;
                }
                results[0] = Val::I64(len as i64);
                return Ok(());
            }
        }
        results[0] = Val::I64(0);
        Ok(())
    })?;

    // libc: atoi(ptr) -> i32
    // We receive ptr to null-terminated string in WASM memory
    let atoi_type = FuncType::new(engine, [ValType::I32], [ValType::I32]);
    linker.func_new("c", "atoi", atoi_type, |mut caller, params, results| {
        let ptr = params[0].unwrap_i32() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr < data.len() {
                // Find end of null-terminated string
                let mut end = ptr;
                while end < data.len() && data[end] != 0 {
                    end += 1;
                }
                // String is already null-terminated in memory, call atoi directly
                let result = unsafe { atoi(data[ptr..].as_ptr() as *const i8) };
                results[0] = Val::I32(result);
                return Ok(());
            }
        }
        results[0] = Val::I32(0);
        Ok(())
    })?;

    // libc: atol(ptr) -> i64
    // We receive ptr to null-terminated string in WASM memory
    let atol_type = FuncType::new(engine, [ValType::I32], [ValType::I64]);
    linker.func_new("c", "atol", atol_type, |mut caller, params, results| {
        let ptr = params[0].unwrap_i32() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr < data.len() {
                // String is already null-terminated in memory, call atol directly
                let result = unsafe { atol(data[ptr..].as_ptr() as *const i8) };
                results[0] = Val::I64(result);
                return Ok(());
            }
        }
        results[0] = Val::I64(0);
        Ok(())
    })?;

    // libc: atof(ptr) -> f64
    // We receive ptr to null-terminated string in WASM memory
    let atof_type = FuncType::new(engine, [ValType::I32], [ValType::F64]);
    linker.func_new("c", "atof", atof_type, |mut caller, params, results| {
        let ptr = params[0].unwrap_i32() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr < data.len() {
                // String is already null-terminated in memory, call atof directly
                let result = unsafe { atof(data[ptr..].as_ptr() as *const i8) };
                results[0] = Val::F64(result.to_bits());
                return Ok(());
            }
        }
        results[0] = Val::F64(0.0f64.to_bits());
        Ok(())
    })?;

    // libc: strcmp(ptr1, len1, ptr2, len2) -> i32
    let strcmp_type = FuncType::new(
        engine,
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    linker.func_new("c", "strcmp", strcmp_type, |mut caller, params, results| {
        let ptr1 = params[0].unwrap_i32() as usize;
        let len1 = params[1].unwrap_i32() as usize;
        let ptr2 = params[2].unwrap_i32() as usize;
        let len2 = params[3].unwrap_i32() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr1 + len1 <= data.len() && ptr2 + len2 <= data.len() {
                let bytes1 = &data[ptr1..ptr1 + len1];
                let bytes2 = &data[ptr2..ptr2 + len2];
                if let (Ok(s1), Ok(s2)) = (std::str::from_utf8(bytes1), std::str::from_utf8(bytes2)) {
                    let mut buf1 = s1.as_bytes().to_vec();
                    buf1.push(0);
                    let mut buf2 = s2.as_bytes().to_vec();
                    buf2.push(0);
                    let result = unsafe { strcmp(buf1.as_ptr() as *const i8, buf2.as_ptr() as *const i8) };
                    results[0] = Val::I32(result);
                    return Ok(());
                }
            }
        }
        results[0] = Val::I32(0);
        Ok(())
    })?;

    // libc: strncmp(ptr1, len1, ptr2, len2, n) -> i32
    let strncmp_type = FuncType::new(
        engine,
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64],
        [ValType::I32],
    );
    linker.func_new("c", "strncmp", strncmp_type, |mut caller, params, results| {
        let ptr1 = params[0].unwrap_i32() as usize;
        let len1 = params[1].unwrap_i32() as usize;
        let ptr2 = params[2].unwrap_i32() as usize;
        let len2 = params[3].unwrap_i32() as usize;
        let n = params[4].unwrap_i64() as usize;

        if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(&caller);
            if ptr1 + len1 <= data.len() && ptr2 + len2 <= data.len() {
                let bytes1 = &data[ptr1..ptr1 + len1];
                let bytes2 = &data[ptr2..ptr2 + len2];
                if let (Ok(s1), Ok(s2)) = (std::str::from_utf8(bytes1), std::str::from_utf8(bytes2)) {
                    let mut buf1 = s1.as_bytes().to_vec();
                    buf1.push(0);
                    let mut buf2 = s2.as_bytes().to_vec();
                    buf2.push(0);
                    let result = unsafe { strncmp(buf1.as_ptr() as *const i8, buf2.as_ptr() as *const i8, n) };
                    results[0] = Val::I32(result);
                    return Ok(());
                }
            }
        }
        results[0] = Val::I32(0);
        Ok(())
    })?;

    // libc: rand() -> i32
    let rand_type = FuncType::new(engine, [], [ValType::I32]);
    linker.func_new("c", "rand", rand_type, |_caller, _params, results| {
        results[0] = Val::I32(unsafe { rand() });
        Ok(())
    })?;

    // Add aliases: "libm" → "m", "libc" → "c"
    // This allows both (import "m" "fmin" ...) and (import "libm" "fmin" ...)
    linker.alias_module("m", "libm")?;
    linker.alias_module("c", "libc")?;

    Ok(())
}

// Dynamic FFI - Load any library through reflection (raylib, SDL2, etc.)
// Parses C headers to discover signatures, loads dylib, creates wasm wrappers
// Uses direct function pointer calls with signature pattern matching

/// Global cache of loaded dynamic libraries
#[cfg(feature = "native")]
static LOADED_LIBRARIES: OnceLock<std::sync::Mutex<HashMap<String, Arc<Library>>>> = OnceLock::new();

/// Get or load a dynamic library
#[cfg(feature = "native")]
fn get_or_load_library(lib_name: &str) -> Option<Arc<Library>> {
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
#[cfg(feature = "native")]
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
    }

    paths
}

/// Normalized parameter type for signature matching
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RetType {
    Void,
    I32,
    I64,
    F32,
    F64,
    Bool,
}

/// Map C type string to normalized ParamType
#[cfg(feature = "native")]
fn c_type_to_param_type(c_type: &str) -> ParamType {
    let t = c_type.trim();
    let t = t.strip_prefix("const ").unwrap_or(t).trim();

    match t {
        "float" => ParamType::F32,
        "double" => ParamType::F64,
        "long" | "int64_t" | "long long" | "size_t" | "ssize_t" | "unsigned long" | "uint64_t" => {
            ParamType::I64
        }
        s if s.contains('*') => match pointer_kind(s) {
            Some(CPointer::Handle) => ParamType::Handle,
            Some(CPointer::Out) => ParamType::Out,
            _ => ParamType::Ptr,
        },
        _ => ParamType::I32, // int, bool, char, short, Color, etc.
    }
}

/// Map C type string to normalized RetType
#[cfg(feature = "native")]
fn c_type_to_ret_type(c_type: &str) -> RetType {
    let t = c_type.trim();
    let t = t.strip_prefix("const ").unwrap_or(t).trim();

    match t {
        "void" => RetType::Void,
        "float" => RetType::F32,
        "double" => RetType::F64,
        "bool" => RetType::Bool,
        "long" | "int64_t" | "long long" | "size_t" | "ssize_t" | "unsigned long" | "uint64_t" => {
            RetType::I64
        }
        _ => RetType::I32,
    }
}

/// Map C type string to wasmtime ValType
#[cfg(feature = "native")]
fn c_type_to_wasm_valtype(c_type: &str) -> Option<ValType> {
    let t = c_type.trim();
    let t = t.strip_prefix("const ").unwrap_or(t).trim();

    match t {
        "void" => None,
        "float" => Some(ValType::F32),
        "double" => Some(ValType::F64),
        "long" | "int64_t" | "long long" | "size_t" | "ssize_t" | "unsigned long" | "uint64_t" => {
            Some(ValType::I64)
        }
        _ => Some(ValType::I32),
    }
}

/// Link all functions from a library discovered through header reflection
#[cfg(feature = "native")]
pub fn link_dynamic_library(
    linker: &mut Linker<FfiState>,
    engine: &Engine,
    lib_name: &str,
    imported: &std::collections::HashSet<String>,
) -> Result<usize> {
    // the module imports under the name the program wrote (`use zlib`), the library and its headers go by their own (z)
    let import_name = lib_name;
    let lib_name = resolve_library_alias(lib_name);
    let library = match get_or_load_library(lib_name) {
        Some(lib) => lib,
        None => {
            return Ok(0);
        }
    };

    // Parse headers to discover function signatures
    let header_paths = crate::ffi_parser::find_library_headers(lib_name);
    if header_paths.is_empty() {
        return Ok(0);
    }

    let mut linked_count = 0;

    // all headers first: their struct types decide how pointers cross (is_struct_type)
    let signatures: Vec<FfiHeaderSignature> = header_paths.iter().flat_map(|path| parse_header_file(path, lib_name)).collect();
    // only what the module imports: libc's headers declare hundreds of functions
    for sig in signatures.iter().filter(|sig| imported.contains(&sig.name)) {
        if link_single_function(linker, engine, import_name, &library, sig).is_ok() {
            linked_count += 1;
        }
    }

    Ok(linked_count)
}

/// Link a single function from a parsed header signature
#[cfg(feature = "native")]
fn link_single_function(
    linker: &mut Linker<FfiState>,
    engine: &Engine,
    lib_name: &str,
    library: &Arc<Library>,
    sig: &FfiHeaderSignature,
) -> Result<()> {
    let func_name = sig.name.clone();

    let func_ptr: usize = unsafe {
        let symbol: libloading::Symbol<*const ()> = library
            .get(func_name.as_bytes())
            .map_err(|e| anyhow::anyhow!("Symbol {} not found: {}", func_name, e))?;
        *symbol as usize
    };

    if func_ptr == 0 {
        return Err(anyhow::anyhow!("Null function pointer for {}", func_name));
    }

    let wasm_params: Vec<ValType> = wasp_parameters(&sig.param_types)
        .filter_map(|t| c_type_to_wasm_valtype(t))
        .collect();

    let param_types: Vec<ParamType> = sig.param_types.iter().map(|t| c_type_to_param_type(t)).collect();
    let ret_type = c_type_to_ret_type(&sig.return_type);
    let result = pointer_result(sig);
    if result.is_some() || param_types.contains(&ParamType::Handle) {
        let wasm_results: Vec<ValType> = match result {
            Some(CPointer::Text) => vec![ValType::ANYREF],
            Some(_) => vec![ValType::I32],
            None => c_type_to_wasm_valtype(&sig.return_type).into_iter().collect(),
        };
        let func_type = FuncType::new(engine, wasm_params, wasm_results);
        let out_pointee = sig.param_types.iter().find(|t| pointer_kind(t) == Some(CPointer::Out)).map(|t| pointee(t));
        let call = PointerCall { name: func_name.clone(), func_ptr, param_types, result, ret_type, out_pointee };
        return create_pointer_wrapper(linker, lib_name, func_type, call);
    }

    let wasm_results: Vec<ValType> = c_type_to_wasm_valtype(&sig.return_type)
        .into_iter()
        .collect();

    let func_type = FuncType::new(engine, wasm_params.clone(), wasm_results.clone());

    // Generate signature key for dispatch (e.g., "III_I" for 3 ints returning int)
    let sig_key = generate_signature_key(&param_types, ret_type);

    // Create the wasmtime function wrapper using macro-generated dispatchers
    create_ffi_wrapper(linker, lib_name, &func_name, func_type, func_ptr, &sig_key, &param_types, ret_type)
}

/// Generate a signature key string for dispatch
#[cfg(feature = "native")]
fn generate_signature_key(params: &[ParamType], ret: RetType) -> String {
    let mut key = String::new();
    for p in params {
        key.push(match p {
            ParamType::I32 => 'I',
            ParamType::I64 => 'L',
            ParamType::F32 => 'F',
            ParamType::F64 => 'D',
            ParamType::Ptr => 'P',
            ParamType::Handle => 'H',
            ParamType::Out => 'O',
        });
    }
    key.push('_');
    key.push(match ret {
        RetType::Void => 'V',
        RetType::I32 => 'I',
        RetType::I64 => 'L',
        RetType::F32 => 'F',
        RetType::F64 => 'D',
        RetType::Bool => 'B',
    });
    key
}

/// Create FFI wrapper with typed function pointer call
#[cfg(feature = "native")]
#[allow(clippy::too_many_arguments)]
fn create_ffi_wrapper(
    linker: &mut Linker<FfiState>,
    lib_name: &str,
    func_name: &str,
    func_type: FuncType,
    func_ptr: usize,
    sig_key: &str,
    param_types: &[ParamType],
    ret_type: RetType,
) -> Result<()> {
    // Clone for closure
    let param_types = param_types.to_vec();

    // Dispatch based on signature - common patterns for raylib/SDL/etc.
    match sig_key {
        // void -> void
        "_V" => {
            linker.func_new(lib_name, func_name, func_type, move |_, _, _| {
                let f: extern "C" fn() = unsafe { std::mem::transmute(func_ptr) };
                f();
                Ok(())
            })?;
        }
        // void -> int (WindowShouldClose, GetMouseX, etc.)
        "_I" => {
            linker.func_new(lib_name, func_name, func_type, move |_, _, results| {
                let f: extern "C" fn() -> i32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::I32(f());
                Ok(())
            })?;
        }
        // void -> bool
        "_B" => {
            linker.func_new(lib_name, func_name, func_type, move |_, _, results| {
                let f: extern "C" fn() -> i32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::I32(if f() != 0 { 1 } else { 0 });
                Ok(())
            })?;
        }
        // void -> float
        "_F" => {
            linker.func_new(lib_name, func_name, func_type, move |_, _, results| {
                let f: extern "C" fn() -> f32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::F32(f().to_bits());
                Ok(())
            })?;
        }
        // void -> double
        "_D" => {
            linker.func_new(lib_name, func_name, func_type, move |_, _, results| {
                let f: extern "C" fn() -> f64 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::F64(f().to_bits());
                Ok(())
            })?;
        }
        // int -> void (SetTargetFPS, etc.)
        "I_V" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, _| {
                let f: extern "C" fn(i32) = unsafe { std::mem::transmute(func_ptr) };
                f(params[0].unwrap_i32());
                Ok(())
            })?;
        }
        // int -> int (IsKeyPressed, etc.)
        "I_I" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::I32(f(params[0].unwrap_i32()));
                Ok(())
            })?;
        }
        // int -> bool
        "I_B" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::I32(if f(params[0].unwrap_i32()) != 0 { 1 } else { 0 });
                Ok(())
            })?;
        }
        // int,int -> void
        "II_V" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, _| {
                let f: extern "C" fn(i32, i32) = unsafe { std::mem::transmute(func_ptr) };
                f(params[0].unwrap_i32(), params[1].unwrap_i32());
                Ok(())
            })?;
        }
        // int,int -> int
        "II_I" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::I32(f(params[0].unwrap_i32(), params[1].unwrap_i32()));
                Ok(())
            })?;
        }
        // int,int,ptr -> void (InitWindow with title)
        "IIP_V" => {
            linker.func_new(lib_name, func_name, func_type, move |mut caller, params, _| {
                let f: extern "C" fn(i32, i32, *const u8) = unsafe { std::mem::transmute(func_ptr) };
                let ptr = get_memory_ptr(&mut caller, params[2].unwrap_i32() as usize);
                f(params[0].unwrap_i32(), params[1].unwrap_i32(), ptr);
                Ok(())
            })?;
        }
        // int,int,float,int -> void (DrawCircle: x, y, radius, color)
        "IIFI_V" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, _| {
                let f: extern "C" fn(i32, i32, f32, i32) = unsafe { std::mem::transmute(func_ptr) };
                f(
                    params[0].unwrap_i32(),
                    params[1].unwrap_i32(),
                    params[2].unwrap_f32(),
                    params[3].unwrap_i32(),
                );
                Ok(())
            })?;
        }
        // int,int,int,int,int -> void (DrawRectangle: x, y, w, h, color)
        "IIIII_V" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, _| {
                let f: extern "C" fn(i32, i32, i32, i32, i32) = unsafe { std::mem::transmute(func_ptr) };
                f(
                    params[0].unwrap_i32(),
                    params[1].unwrap_i32(),
                    params[2].unwrap_i32(),
                    params[3].unwrap_i32(),
                    params[4].unwrap_i32(),
                );
                Ok(())
            })?;
        }
        // ptr,int,int,int -> void (DrawText: text, x, y, fontSize, color)
        "PIII_V" => {
            linker.func_new(lib_name, func_name, func_type, move |mut caller, params, _| {
                let f: extern "C" fn(*const u8, i32, i32, i32) = unsafe { std::mem::transmute(func_ptr) };
                let ptr = get_memory_ptr(&mut caller, params[0].unwrap_i32() as usize);
                f(ptr, params[1].unwrap_i32(), params[2].unwrap_i32(), params[3].unwrap_i32());
                Ok(())
            })?;
        }
        // ptr,int,int,int,int -> void (DrawText with color: text, x, y, fontSize, color)
        "PIIII_V" => {
            linker.func_new(lib_name, func_name, func_type, move |mut caller, params, _| {
                let f: extern "C" fn(*const u8, i32, i32, i32, i32) = unsafe { std::mem::transmute(func_ptr) };
                let ptr = get_memory_ptr(&mut caller, params[0].unwrap_i32() as usize);
                f(ptr, params[1].unwrap_i32(), params[2].unwrap_i32(), params[3].unwrap_i32(), params[4].unwrap_i32());
                Ok(())
            })?;
        }
        // float -> float
        "F_F" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(f32) -> f32 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::F32(f(params[0].unwrap_f32()).to_bits());
                Ok(())
            })?;
        }
        // double -> double
        "D_D" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::F64(f(params[0].unwrap_f64()).to_bits());
                Ok(())
            })?;
        }
        // double,double -> double
        "DD_D" => {
            linker.func_new(lib_name, func_name, func_type, move |_, params, results| {
                let f: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(func_ptr) };
                results[0] = Val::F64(f(params[0].unwrap_f64(), params[1].unwrap_f64()).to_bits());
                Ok(())
            })?;
        }
        // Generic fallback using dynamic dispatch
        _ => {
            return create_generic_ffi_wrapper(linker, lib_name, func_name, func_type, func_ptr, &param_types, ret_type);
        }
    }

    Ok(())
}

/// Link all dynamic libraries required by a WASM module's imports
/// Scans the module for import statements and links matching libraries via reflection
#[cfg(feature = "native")]
pub fn link_module_libraries(
    linker: &mut Linker<FfiState>,
    engine: &Engine,
    module: &wasmtime::Module,
) -> Result<()> {
    use std::collections::HashSet;

    // Collect unique library names from imports
    let mut libs_to_link: HashMap<String, HashSet<String>> = HashMap::new();

    for import in module.imports() {
        let module_name = import.module();
        // Skip built-in libraries that are already linked; libc beyond its built-in functions (toupper) is linked here
        let built_in_libc = matches!(module_name, "c" | "libc") && HAND_LINKED_LIBC.contains(&import.name());
        if !built_in_libc && !matches!(module_name, "m" | "libm" | "env" | "wasi_snapshot_preview1" | crate::host::HOST_LIBRARY) {
            libs_to_link.entry(module_name.to_string()).or_default().insert(import.name().to_string());
        }
    }

    // Link each discovered library
    for (lib_name, imported) in libs_to_link {
        match link_dynamic_library(linker, engine, &lib_name, &imported) {
            Ok(count) if count > 0 => {
            }
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

/// Generic FFI wrapper for uncommon signatures - uses dynamic argument handling
#[cfg(feature = "native")]
fn create_generic_ffi_wrapper(
    linker: &mut Linker<FfiState>,
    lib_name: &str,
    func_name: &str,
    func_type: FuncType,
    func_ptr: usize,
    param_types: &[ParamType],
    ret_type: RetType,
) -> Result<()> {
    let param_types = param_types.to_vec();

    // For generic case, we pack all args into an array and use assembly/platform-specific calling
    // This is a simplified version that handles up to 8 args (enough for most APIs)
    linker.func_new(lib_name, func_name, func_type, move |mut caller, params, results| {
        let args = native_arguments(&mut caller, "", params, &param_types, &mut 0)?;

        // Call using platform-specific calling convention
        // ARM64 and x86_64 use similar conventions for first 8 integer/pointer args
        let ret = unsafe { call_native_function(func_ptr, &args, params.len()) };

        if let Some(result) = results.first_mut() {
            *result = native_result(ret, ret_type);
        }

        Ok(())
    })?;

    Ok(())
}

/// The raw arguments of a native call (up to 8), one per C parameter: numbers as they are, a memory pointer as the
/// address of its text in linear memory, a handle id as its C pointer; the first out-pointer gets `out_slot`, others NULL
#[cfg(feature = "native")]
fn native_arguments(caller: &mut wasmtime::Caller<'_, FfiState>, name: &str, params: &[Val], param_types: &[ParamType], out_slot: &mut usize) -> wasmtime::Result<[u64; 8]> {
    let mut args: [u64; 8] = [0; 8];
    let mut wasp_params = params.iter();
    let mut out_slot = Some(out_slot);
    for (i, ptype) in param_types.iter().enumerate().take(8) {
        if *ptype == ParamType::Out {
            args[i] = out_slot.take().map_or(0, |slot| slot as *mut usize as u64);
            continue;
        }
        let Some(param) = wasp_params.next() else { break };
        args[i] = match ptype {
            ParamType::I32 => param.unwrap_i32() as u64,
            ParamType::I64 => param.unwrap_i64() as u64,
            ParamType::F32 => (param.unwrap_f32() as f64).to_bits(),
            ParamType::F64 => param.unwrap_f64().to_bits(),
            ParamType::Ptr => get_memory_ptr(caller, param.unwrap_i32() as usize) as u64,
            ParamType::Handle => {
                let id = param.unwrap_i32();
                caller.data().c_handles.pointer(id).ok_or_else(|| wasmtime::Error::msg(format!("{name}: {id} is no C handle of this run")))? as u64
            }
            ParamType::Out => unreachable!("out-pointers are filled above"),
        };
    }
    Ok(args)
}

/// The C pointers a run got, handed to wasp as ids: id n is the n-th pointer, 0 is NULL; the same pointer keeps its id.
/// The table lives in the run's HostState and goes with it: ids are never addresses, and never outlive the run
#[cfg(feature = "native")]
#[derive(Default)]
pub struct CHandles {
    pointers: Vec<usize>,
}

#[cfg(feature = "native")]
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

/// A C function crossing pointers (notes/ffi_handles.md): handle parameters, an out-pointer, a text or handle result
#[cfg(feature = "native")]
struct PointerCall {
    name: String,
    func_ptr: usize,
    param_types: Vec<ParamType>,
    /// Text, Handle, or None for the C function's own number (ret_type)
    result: Option<CPointer>,
    ret_type: RetType,
    /// what the first out-pointer points to (`sqlite3_stmt` of `sqlite3_stmt **ppStmt`), for the error of a NULL there
    out_pointee: Option<String>,
}

/// One wrapper for every C function crossing pointers: `sqlite3_column_text(stmt, i)` takes a handle and gives a text,
/// `sqlite3_open(name)` gives the handle its out-pointer received (NULL there is an error naming the C status), a
/// `char *` result is copied into the module as a wasp text (NULL is ø)
#[cfg(feature = "native")]
fn create_pointer_wrapper(linker: &mut Linker<FfiState>, lib_name: &str, func_type: FuncType, call: PointerCall) -> Result<()> {
    let name = call.name.clone();
    linker.func_new(lib_name, &name, func_type, move |mut caller, params, results| {
        let mut out_slot: usize = 0;
        let args = native_arguments(&mut caller, &call.name, params, &call.param_types, &mut out_slot)?;
        let returned = unsafe { call_native_function(call.func_ptr, &args, call.param_types.len()) };
        if let Some(pointee) = &call.out_pointee {
            if out_slot == 0 {
                return Err(wasmtime::Error::msg(format!("{} gave no {pointee} (C status {})", call.name, returned as i32)));
            }
        }
        let pointer = if call.out_pointee.is_some() { out_slot } else { returned as usize };
        let Some(result) = results.first_mut() else { return Ok(()) };
        *result = match call.result {
            Some(CPointer::Text) => {
                // copied before anything else runs: a C string from getenv may change with the next call
                let pointer = pointer as *const std::ffi::c_char;
                let text = (!pointer.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(pointer) }.to_bytes().to_vec());
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
#[cfg(feature = "native")]
fn native_result(returned: u64, ret_type: RetType) -> Val {
    match ret_type {
        RetType::Void | RetType::I32 => Val::I32(returned as i32),
        RetType::I64 => Val::I64(returned as i64),
        RetType::F32 => Val::F32((returned as f32).to_bits()),
        RetType::F64 => Val::F64(returned),
        RetType::Bool => Val::I32(if returned != 0 { 1 } else { 0 }),
    }
}

/// A wasp text of `bytes` in the calling module (its memory, text heap and new_text), or ø
#[cfg(feature = "native")]
fn text_node(caller: &mut wasmtime::Caller<'_, FfiState>, bytes: Option<&[u8]>) -> Result<Val> {
    let export = |caller: &mut wasmtime::Caller<'_, FfiState>, name: &str| caller.get_export(name).ok_or_else(|| anyhow::anyhow!("a C text result needs the module's {name}"));
    let mut result = [Val::AnyRef(None)];
    let Some(bytes) = bytes else {
        let new_empty = export(caller, "new_empty")?.into_func().ok_or_else(|| anyhow::anyhow!("new_empty is no function"))?;
        new_empty.call(&mut *caller, &[], &mut result)?;
        return Ok(result[0]);
    };
    let memory = export(caller, "memory")?.into_memory().ok_or_else(|| anyhow::anyhow!("memory is no memory"))?;
    let heap = export(caller, crate::host::TEXT_HEAP_EXPORT)?.into_global().ok_or_else(|| anyhow::anyhow!("the text heap is no global"))?;
    let new_text = export(caller, "new_text")?.into_func().ok_or_else(|| anyhow::anyhow!("new_text is no function"))?;
    let (pointer, length) = crate::host::write_to_heap(&memory, heap, &mut wasmtime::AsContextMut::as_context_mut(caller), bytes)?;
    new_text.call(&mut *caller, &[Val::I32(pointer as i32), Val::I32(length as i32)], &mut result)?;
    Ok(result[0])
}

/// Get pointer into WASM linear memory
#[cfg(feature = "native")]
fn get_memory_ptr(caller: &mut wasmtime::Caller<'_, FfiState>, offset: usize) -> *const u8 {
    if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        unsafe { memory.data_ptr(&caller).add(offset) }
    } else {
        std::ptr::null()
    }
}

/// Call native function with up to 8 arguments
/// Uses platform calling convention (ARM64/x86_64)
#[cfg(feature = "native")]
#[inline(never)]
unsafe fn call_native_function(func_ptr: usize, args: &[u64; 8], arg_count: usize) -> u64 {
    // Cast to function pointer type based on arg count
    // Both ARM64 and x86_64 pass first 6-8 integer args in registers
    match arg_count {
        0 => {
            let f: extern "C" fn() -> u64 = std::mem::transmute(func_ptr);
            f()
        }
        1 => {
            let f: extern "C" fn(u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0])
        }
        2 => {
            let f: extern "C" fn(u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1])
        }
        3 => {
            let f: extern "C" fn(u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2])
        }
        4 => {
            let f: extern "C" fn(u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2], args[3])
        }
        5 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2], args[3], args[4])
        }
        6 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2], args[3], args[4], args[5])
        }
        7 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2], args[3], args[4], args[5], args[6])
        }
        _ => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            f(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7])
        }
    }
}
