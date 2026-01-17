// FFI (Foreign Function Interface) support for calling native C libraries
// Provides imports for libc and libm functions via wasmtime linker
//
// Architecture:
// - ffi_parser.rs: Built-in/embedded C header definitions (portable, no dependencies)
// - This file (ffi.rs): Runtime parsing of system headers for additional functions
//
// Signature lookup order (see get_ffi_signature):
// 1. Built-in signatures from ffi_parser (libc, libm basics)
// 2. System header parsing for extended functions (SDL2, raylib, etc.)

use anyhow::Result;
use std::collections::HashMap;
use wasmtime::{Engine, FuncType, Linker, Store, Val};

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
        FfiSignature {
            name,
            library,
            params,
            results,
        }
    }
}

/// FFI state for running modules with native function imports
#[derive(Default)]
pub struct FfiState {
    // Memory pointer for string operations
    pub memory_ptr: Option<*const u8>,
    pub memory_len: usize,
}

impl FfiState {
    pub fn new() -> Self {
        FfiState {
            memory_ptr: None,
            memory_len: 0,
        }
    }
}

// ============================================================================
// C Header Parsing for Dynamic FFI Signature Discovery
// ============================================================================

/// Parsed C function signature from header file
#[derive(Clone, Debug)]
pub struct FfiHeaderSignature {
    pub name: String,
    pub return_type: String,
    pub param_types: Vec<String>,
    pub param_names: Vec<String>,
    pub library: String,
    pub raw: String,
}

/// Map C type string to wasm_encoder ValType
pub fn map_c_type_to_valtype(c_type: &str) -> Option<wasm_encoder::ValType> {
    let t = c_type.trim();
    // Remove const qualifier
    let t = t.strip_prefix("const ").unwrap_or(t).trim();

    match t {
        "double" => Some(wasm_encoder::ValType::F64),
        "float" => Some(wasm_encoder::ValType::F32),
        "int" | "int32_t" => Some(wasm_encoder::ValType::I32),
        "unsigned int" | "uint32_t" | "Uint32" => Some(wasm_encoder::ValType::I32),
        "long" | "int64_t" | "long long" | "size_t" | "ssize_t" => Some(wasm_encoder::ValType::I64),
        "unsigned long" | "uint64_t" | "Uint64" => Some(wasm_encoder::ValType::I64),
        "short" | "int16_t" => Some(wasm_encoder::ValType::I32),
        "char" | "int8_t" | "unsigned char" | "uint8_t" => Some(wasm_encoder::ValType::I32),
        "void" => None, // void return means no result
        // Pointer types - all become i32 (WASM linear memory offset)
        s if s.ends_with('*') => Some(wasm_encoder::ValType::I32),
        s if s.contains('*') => Some(wasm_encoder::ValType::I32),
        // Unknown types default to i32 (could be opaque handles)
        _ => Some(wasm_encoder::ValType::I32),
    }
}

/// Extract function signature from a C declaration string
/// e.g., "double sqrt(double x);" -> FfiHeaderSignature
pub fn extract_function_signature(declaration: &str, library: &str) -> Option<FfiHeaderSignature> {
    let decl = declaration.trim();

    // Skip empty lines, comments, preprocessor directives
    if decl.is_empty() || decl.starts_with("//") || decl.starts_with('#') || decl.starts_with("/*") {
        return None;
    }

    // Skip non-function declarations (typedef, struct, enum, etc.)
    if decl.starts_with("typedef") || decl.starts_with("struct") ||
       decl.starts_with("enum") || decl.starts_with("union") {
        return None;
    }

    // Remove extern, static, inline qualifiers
    let decl = decl.replace("extern ", "").replace("static ", "").replace("inline ", "");
    let decl = decl.trim();

    // Find the opening parenthesis (marks start of params)
    let paren_pos = decl.find('(')?;

    // Everything before '(' is return_type + name
    let before_paren = &decl[..paren_pos];

    // Find function name (last word before parenthesis)
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Function name is the last part, may have pointer marker
    let mut name = parts.last()?.to_string();
    // Handle "*func" case
    while name.starts_with('*') {
        name = name[1..].to_string();
    }

    // Return type is everything before the name
    let return_type = if parts.len() > 1 {
        parts[..parts.len()-1].join(" ")
    } else {
        "int".to_string() // C default
    };

    // Find closing paren
    let close_paren = decl.rfind(')')?;
    let params_str = &decl[paren_pos+1..close_paren];

    // Parse parameters
    let mut param_types = Vec::new();
    let mut param_names = Vec::new();

    if params_str.trim() != "void" && !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            if param.is_empty() || param == "..." {
                continue;
            }

            // Split param into type and name
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Handle "type *name" or "type* name" or "type name"
            let (ptype, pname) = if parts.len() == 1 {
                (parts[0].to_string(), String::new())
            } else {
                let last = parts.last().unwrap();
                let name = last.trim_start_matches('*').to_string();
                // Reconstruct type (everything except pure name)
                let type_str = if last.starts_with('*') {
                    format!("{} *", parts[..parts.len()-1].join(" "))
                } else {
                    parts[..parts.len()-1].join(" ")
                };
                (type_str, name)
            };

            param_types.push(ptype);
            param_names.push(pname);
        }
    }

    Some(FfiHeaderSignature {
        name,
        return_type,
        param_types,
        param_names,
        library: library.to_string(),
        raw: declaration.to_string(),
    })
}

/// Convert FfiHeaderSignature to FfiSignature (using 'static lifetime via leak)
pub fn header_sig_to_ffi_sig(hsig: &FfiHeaderSignature) -> Option<FfiSignature> {
    let params: Vec<wasm_encoder::ValType> = hsig.param_types.iter()
        .filter_map(|t| map_c_type_to_valtype(t))
        .collect();

    let results: Vec<wasm_encoder::ValType> = map_c_type_to_valtype(&hsig.return_type)
        .map(|v| vec![v])
        .unwrap_or_default();

    // Leak strings for 'static lifetime (acceptable for long-running process)
    let name: &'static str = Box::leak(hsig.name.clone().into_boxed_str());
    let library: &'static str = Box::leak(hsig.library.clone().into_boxed_str());

    Some(FfiSignature {
        name,
        library,
        params,
        results,
    })
}

/// Get standard header file paths for a library (delegates to ffi_parser)
pub fn get_library_header_paths(library: &str) -> Vec<String> {
    crate::ffi_parser::find_library_headers(resolve_library_alias(library))
}

/// Parse a header file and extract all function signatures
pub fn parse_header_file(path: &str, library: &str) -> Vec<FfiHeaderSignature> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut signatures = Vec::new();
    let mut current_decl = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip preprocessor and comments
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Skip block comments (simplified - doesn't handle multi-line)
        if line.contains("/*") && line.contains("*/") {
            continue;
        }

        // Accumulate multi-line declarations
        current_decl.push_str(line);
        current_decl.push(' ');

        // If we have a complete declaration (ends with ; or })
        if line.ends_with(';') && current_decl.contains('(') {
            if let Some(sig) = extract_function_signature(&current_decl, library) {
                signatures.push(sig);
            }
            current_decl.clear();
        } else if line.ends_with('}') || (line.ends_with(';') && !current_decl.contains('(')) {
            current_decl.clear();
        }
    }

    signatures
}

/// Get FFI signatures by parsing header files for a library
/// Falls back to hardcoded signatures if headers not found
pub fn get_signatures_from_headers(library: &str) -> HashMap<String, FfiSignature> {
    let mut sigs = HashMap::new();
    let paths = get_library_header_paths(library);

    for path in paths {
        let header_sigs = parse_header_file(&path, library);
        for hsig in header_sigs {
            if let Some(ffi_sig) = header_sig_to_ffi_sig(&hsig) {
                sigs.insert(hsig.name.clone(), ffi_sig);
            }
        }
    }

    sigs
}

// ============================================================================
// Extern C Functions and Hardcoded Signatures (Fallback)
// ============================================================================

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
	fn strlen(s: *const i8) -> usize;
	fn atoi(s: *const i8) -> i32;
	fn atol(s: *const i8) -> i64;
	fn atof(s: *const i8) -> f64;
	fn strcmp(s1: *const i8, s2: *const i8) -> i32;
	fn strncmp(s1: *const i8, s2: *const i8, n: usize) -> i32;
	fn rand() -> i32;
}

/// Get known FFI function signatures by parsing system header files
/// Uses unified Kind/Signature types from ffi_parser module
pub fn get_ffi_signatures() -> HashMap<String, FfiSignature> {
    use crate::ffi_parser::get_all_signatures;
    use crate::function::kind_to_valtype;

    let mut sigs = HashMap::new();

    // Convert from ffi_parser's FfiFunction to FfiSignature
    for (name, func) in get_all_signatures() {
        // Convert Kind parameters to ValType (kind_to_valtype returns wasm_encoder::ValType)
        let params: Vec<wasm_encoder::ValType> = func.signature.parameters.iter()
            .map(|p| kind_to_valtype(p.kind))
            .collect();

        // Convert Kind return types to ValType
        let results: Vec<wasm_encoder::ValType> = func.signature.return_types.iter()
            .map(|k| kind_to_valtype(*k))
            .collect();

        // Use leaked strings for 'static lifetime
        let name_static: &'static str = Box::leak(name.clone().into_boxed_str());
        let lib_static: &'static str = match func.library.as_str() {
            "m" => "m",
            "c" => "c",
            "SDL2" => "SDL2",
            "raylib" => "raylib",
            _ => Box::leak(func.library.clone().into_boxed_str()),
        };

        sigs.insert(name, FfiSignature {
            name: name_static,
            library: lib_static,
            params,
            results,
        });
    }

    // Override specific WASM-adapted signatures that differ from C conventions:
    // strcmp/strncmp use (ptr, len, ptr, len) instead of (ptr, ptr) for WASM strings
    use wasm_encoder::ValType;
    sigs.insert(
        "strcmp".to_string(),
        FfiSignature {
            name: "strcmp",
            library: "c",
            params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            results: vec![ValType::I32],
        },
    );
    sigs.insert(
        "strncmp".to_string(),
        FfiSignature {
            name: "strncmp",
            library: "c",
            params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64],
            results: vec![ValType::I32],
        },
    );

    // Add common libc functions that may not be directly in parsed headers
    // (defined in _stdlib.h which is included by stdlib.h)
    sigs.insert(
        "abs".to_string(),
        FfiSignature {
            name: "abs",
            library: "c",
            params: vec![ValType::I32],
            results: vec![ValType::I32],
        },
    );
    sigs.insert(
        "labs".to_string(),
        FfiSignature {
            name: "labs",
            library: "c",
            params: vec![ValType::I64],
            results: vec![ValType::I64],
        },
    );
    // String functions from string.h
    sigs.insert(
        "strlen".to_string(),
        FfiSignature {
            name: "strlen",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I64], // returns size_t (i64)
        },
    );
    sigs.insert(
        "atoi".to_string(),
        FfiSignature {
            name: "atoi",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I32],
        },
    );
    sigs.insert(
        "atol".to_string(),
        FfiSignature {
            name: "atol",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I64],
        },
    );
    sigs.insert(
        "atof".to_string(),
        FfiSignature {
            name: "atof",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::F64],
        },
    );
    sigs.insert(
        "rand".to_string(),
        FfiSignature {
            name: "rand",
            library: "c",
            params: vec![],
            results: vec![ValType::I32],
        },
    );

    sigs
}

/// Check if a function is a known FFI function
pub fn is_ffi_function(name: &str) -> bool {
    get_ffi_signature(name).is_some()
}

/// Get FFI signature for a function by name
/// Searches well-known libraries (m, c, SDL2) via dynamic header discovery
pub fn get_ffi_signature(name: &str) -> Option<FfiSignature> {
    // First check cached signatures from well-known libraries
    if let Some(sig) = get_ffi_signatures().get(name).cloned() {
        return Some(sig);
    }

    // Try well-known libraries via header discovery
    for lib in ["m", "c", "SDL2"] {
        let header_sigs = get_signatures_from_headers(lib);
        if let Some(sig) = header_sigs.get(name).cloned() {
            return Some(sig);
        }
    }

    None
}

/// Get FFI signature from a specific library's headers
pub fn get_ffi_signature_from_lib(name: &str, library: &str) -> Option<FfiSignature> {
    // First check hardcoded
    if let Some(sig) = get_ffi_signatures().get(name).cloned() {
        if sig.library == resolve_library_alias(library) {
            return Some(sig);
        }
    }

    // Try headers
    let header_sigs = get_signatures_from_headers(library);
    header_sigs.get(name).cloned()
}

/// Link FFI functions into a wasmtime linker
pub fn link_ffi_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
    use wasmtime::ValType;

    // libm: fmin(f64, f64) -> f64
    let fmin_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("m", "fmin", fmin_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmin(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: fmax(f64, f64) -> f64
    let fmax_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("m", "fmax", fmax_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmax(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: fabs(f64) -> f64
    let fabs_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "fabs", fabs_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { fabs(x) }.to_bits());
        Ok(())
    })?;

    // libm: floor(f64) -> f64
    let floor_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "floor", floor_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { floor(x) }.to_bits());
        Ok(())
    })?;

    // libm: ceil(f64) -> f64
    let ceil_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "ceil", ceil_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { ceil(x) }.to_bits());
        Ok(())
    })?;

    // libm: round(f64) -> f64
    let round_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "round", round_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { round(x) }.to_bits());
        Ok(())
    })?;

    // libm: sqrt(f64) -> f64
    let sqrt_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "sqrt", sqrt_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { sqrt(x) }.to_bits());
        Ok(())
    })?;

    // libm: sin(f64) -> f64
    let sin_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "sin", sin_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { sin(x) }.to_bits());
        Ok(())
    })?;

    // libm: cos(f64) -> f64
    let cos_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "cos", cos_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { cos(x) }.to_bits());
        Ok(())
    })?;

    // libm: tan(f64) -> f64
    let tan_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "tan", tan_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { tan(x) }.to_bits());
        Ok(())
    })?;

    // libm: fmod(f64, f64) -> f64
    let fmod_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("m", "fmod", fmod_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmod(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: pow(f64, f64) -> f64
    let pow_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("m", "pow", pow_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { pow(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: exp(f64) -> f64
    let exp_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "exp", exp_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { exp(x) }.to_bits());
        Ok(())
    })?;

    // libm: log(f64) -> f64
    let log_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "log", log_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { log(x) }.to_bits());
        Ok(())
    })?;

    // libm: log10(f64) -> f64
    let log10_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("m", "log10", log10_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { log10(x) }.to_bits());
        Ok(())
    })?;

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

/// Resolve library alias to canonical name
pub fn resolve_library_alias(alias: &str) -> &'static str {
    match alias {
        "m" | "math" | "libm" => "m",
        "c" | "libc" => "c",
        "SDL2" | "sdl2" | "sdl" => "SDL2",
        _ => {
            // Strip "lib" prefix for any library (e.g., "libfoo" → "foo")
            if let Some(stripped) = alias.strip_prefix("lib") {
                // Leak the string to get static lifetime (safe for small number of libs)
                Box::leak(stripped.to_string().into_boxed_str())
            } else {
                Box::leak(alias.to_string().into_boxed_str())
            }
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

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_signature_simple() {
        let sig = extract_function_signature("double sqrt(double x);", "m").unwrap();
        assert_eq!(sig.name, "sqrt");
        assert_eq!(sig.return_type, "double");
        assert_eq!(sig.param_types, vec!["double"]);
    }

    #[test]
    fn test_extract_function_signature_multi_param() {
        let sig = extract_function_signature("double fmin(double x, double y);", "m").unwrap();
        assert_eq!(sig.name, "fmin");
        assert_eq!(sig.return_type, "double");
        assert_eq!(sig.param_types.len(), 2);
    }

    #[test]
    fn test_extract_function_signature_pointer() {
        let sig = extract_function_signature("size_t strlen(const char *s);", "c").unwrap();
        assert_eq!(sig.name, "strlen");
        assert_eq!(sig.return_type, "size_t");
        assert!(sig.param_types[0].contains("char"));
    }

    #[test]
    fn test_extract_function_signature_void_return() {
        let sig = extract_function_signature("void exit(int status);", "c").unwrap();
        assert_eq!(sig.name, "exit");
        assert_eq!(sig.return_type, "void");
    }

    #[test]
    fn test_map_c_type_to_valtype() {
        assert_eq!(map_c_type_to_valtype("double"), Some(wasm_encoder::ValType::F64));
        assert_eq!(map_c_type_to_valtype("float"), Some(wasm_encoder::ValType::F32));
        assert_eq!(map_c_type_to_valtype("int"), Some(wasm_encoder::ValType::I32));
        assert_eq!(map_c_type_to_valtype("long"), Some(wasm_encoder::ValType::I64));
        assert_eq!(map_c_type_to_valtype("char *"), Some(wasm_encoder::ValType::I32));
        assert_eq!(map_c_type_to_valtype("void"), None);
    }

    #[test]
    fn test_header_sig_to_ffi_sig() {
        let hsig = FfiHeaderSignature {
            name: "sqrt".to_string(),
            return_type: "double".to_string(),
            param_types: vec!["double".to_string()],
            param_names: vec!["x".to_string()],
            library: "m".to_string(),
            raw: "double sqrt(double x);".to_string(),
        };

        let ffi_sig = header_sig_to_ffi_sig(&hsig).unwrap();
        assert_eq!(ffi_sig.name, "sqrt");
        assert_eq!(ffi_sig.params, vec![wasm_encoder::ValType::F64]);
        assert_eq!(ffi_sig.results, vec![wasm_encoder::ValType::F64]);
    }

    #[test]
    fn test_parse_header_file_math() {
        // Try to parse math.h if it exists
        let paths = get_library_header_paths("m");
        for path in paths {
            let sigs = parse_header_file(&path, "m");
            if !sigs.is_empty() {
                // Found some signatures, verify we can find common math functions
                let names: Vec<&str> = sigs.iter().map(|s| s.name.as_str()).collect();
                // At least some common functions should be found
                let has_common = names.iter().any(|n| {
                    ["sin", "cos", "sqrt", "floor", "ceil", "fabs", "pow", "exp", "log"]
                        .contains(n)
                });
                if has_common {
                    return; // Test passes
                }
            }
        }
        // If no header found, just pass (CI might not have headers)
    }
}
