// FFI (Foreign Function Interface) support for calling native C libraries
// Provides imports for libc and libm functions via wasmtime linker

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

/// Get known FFI function signatures
pub fn get_ffi_signatures() -> HashMap<String, FfiSignature> {
    use wasm_encoder::ValType;
    let mut sigs = HashMap::new();

    // libm functions (f64 -> f64)
    for name in ["floor", "ceil", "round", "sqrt", "sin", "cos", "tan", "fabs", "exp", "log", "log10"] {
        sigs.insert(
            name.to_string(),
            FfiSignature {
                name,
                library: "m",
                params: vec![ValType::F64],
                results: vec![ValType::F64],
            },
        );
    }

    // libm functions (f64, f64 -> f64)
    for name in ["fmin", "fmax", "fmod", "pow"] {
        sigs.insert(
            name.to_string(),
            FfiSignature {
                name,
                library: "m",
                params: vec![ValType::F64, ValType::F64],
                results: vec![ValType::F64],
            },
        );
    }

    // libc: abs (i32 -> i32)
    sigs.insert(
        "abs".to_string(),
        FfiSignature {
            name: "abs",
            library: "c",
            params: vec![ValType::I32],
            results: vec![ValType::I32],
        },
    );

    // libc: strlen (ptr -> i64) - C string with null terminator
    sigs.insert(
        "strlen".to_string(),
        FfiSignature {
            name: "strlen",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I64],
        },
    );

    // libc: atoi (ptr -> i32) - C string with null terminator
    sigs.insert(
        "atoi".to_string(),
        FfiSignature {
            name: "atoi",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I32],
        },
    );

    // libc: atol (ptr -> i64) - C string with null terminator
    sigs.insert(
        "atol".to_string(),
        FfiSignature {
            name: "atol",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::I64],
        },
    );

    // libc: atof (ptr -> f64) - C string with null terminator
    sigs.insert(
        "atof".to_string(),
        FfiSignature {
            name: "atof",
            library: "c",
            params: vec![ValType::I32], // ptr to null-terminated string
            results: vec![ValType::F64],
        },
    );

    // libc: strcmp (ptr1, len1, ptr2, len2 -> i32)
    sigs.insert(
        "strcmp".to_string(),
        FfiSignature {
            name: "strcmp",
            library: "c",
            params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            results: vec![ValType::I32],
        },
    );

    // libc: strncmp (ptr1, len1, ptr2, len2, n -> i32)
    sigs.insert(
        "strncmp".to_string(),
        FfiSignature {
            name: "strncmp",
            library: "c",
            params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64],
            results: vec![ValType::I32],
        },
    );

    // libc: rand (-> i32)
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
    get_ffi_signatures().contains_key(name)
}

/// Get FFI signature for a function
pub fn get_ffi_signature(name: &str) -> Option<FfiSignature> {
    get_ffi_signatures().get(name).cloned()
}

/// Link FFI functions into a wasmtime linker
pub fn link_ffi_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
    use wasmtime::ValType;

    // libm: fmin(f64, f64) -> f64
    let fmin_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "fmin", fmin_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmin(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: fmax(f64, f64) -> f64
    let fmax_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "fmax", fmax_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmax(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: fabs(f64) -> f64
    let fabs_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "fabs", fabs_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { fabs(x) }.to_bits());
        Ok(())
    })?;

    // libm: floor(f64) -> f64
    let floor_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "floor", floor_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { floor(x) }.to_bits());
        Ok(())
    })?;

    // libm: ceil(f64) -> f64
    let ceil_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "ceil", ceil_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { ceil(x) }.to_bits());
        Ok(())
    })?;

    // libm: round(f64) -> f64
    let round_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "round", round_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { round(x) }.to_bits());
        Ok(())
    })?;

    // libm: sqrt(f64) -> f64
    let sqrt_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "sqrt", sqrt_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { sqrt(x) }.to_bits());
        Ok(())
    })?;

    // libm: sin(f64) -> f64
    let sin_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "sin", sin_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { sin(x) }.to_bits());
        Ok(())
    })?;

    // libm: cos(f64) -> f64
    let cos_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "cos", cos_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { cos(x) }.to_bits());
        Ok(())
    })?;

    // libm: tan(f64) -> f64
    let tan_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "tan", tan_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { tan(x) }.to_bits());
        Ok(())
    })?;

    // libm: fmod(f64, f64) -> f64
    let fmod_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "fmod", fmod_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { fmod(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: pow(f64, f64) -> f64
    let pow_type = FuncType::new(engine, [ValType::F64, ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "pow", pow_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        let y = params[1].unwrap_f64();
        results[0] = Val::F64(unsafe { pow(x, y) }.to_bits());
        Ok(())
    })?;

    // libm: exp(f64) -> f64
    let exp_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "exp", exp_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { exp(x) }.to_bits());
        Ok(())
    })?;

    // libm: log(f64) -> f64
    let log_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "log", log_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { log(x) }.to_bits());
        Ok(())
    })?;

    // libm: log10(f64) -> f64
    let log10_type = FuncType::new(engine, [ValType::F64], [ValType::F64]);
    linker.func_new("ffi", "log10", log10_type, |_caller, params, results| {
        let x = params[0].unwrap_f64();
        results[0] = Val::F64(unsafe { log10(x) }.to_bits());
        Ok(())
    })?;

    // libc: abs(i32) -> i32
    let abs_type = FuncType::new(engine, [ValType::I32], [ValType::I32]);
    linker.func_new("ffi", "abs", abs_type, |_caller, params, results| {
        let x = params[0].unwrap_i32();
        results[0] = Val::I32(unsafe { abs(x) });
        Ok(())
    })?;

    // libc: strlen(ptr) -> i64
    // We receive ptr to null-terminated string in WASM memory
    let strlen_type = FuncType::new(engine, [ValType::I32], [ValType::I64]);
    linker.func_new("ffi", "strlen", strlen_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "atoi", atoi_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "atol", atol_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "atof", atof_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "strcmp", strcmp_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "strncmp", strncmp_type, |mut caller, params, results| {
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
    linker.func_new("ffi", "rand", rand_type, |_caller, _params, results| {
        results[0] = Val::I32(unsafe { rand() });
        Ok(())
    })?;

    Ok(())
}

/// Resolve library alias to full name
pub fn resolve_library_alias(alias: &str) -> &str {
    match alias {
        "m" | "math" | "libm" => "m",
        "c" | "libc" => "c",
        "SDL2" | "sdl2" | "sdl" => "SDL2",
        "raylib" => "raylib",
        _ => alias,
    }
}

/// Check if a library is a known FFI library
pub fn is_ffi_library(lib: &str) -> bool {
    matches!(resolve_library_alias(lib), "m" | "c" | "SDL2" | "raylib")
}
