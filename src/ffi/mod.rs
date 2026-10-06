// FFI (Foreign Function Interface) support for calling native C libraries
// Provides imports for libc and libm functions via wasmtime linker
//
// Files: header.rs reads C headers into signatures, link.rs links them into wasmtime (native only), this file
// looks signatures up and says how C pointers cross (notes/ffi_handles.md).
//
// Architecture:
// - ffi_parser.rs: Built-in/embedded C header definitions (portable, no dependencies)
// - This file (ffi.rs): Runtime parsing of system headers for additional functions
//
// Signature lookup order (see get_ffi_signature):
// 1. Built-in signatures from ffi_parser (libc, libm basics)
// 2. System header parsing for extended functions (SDL2, raylib, etc.)

#[cfg(feature = "native")]
use anyhow::Result;
#[cfg(feature = "native")]
use libloading::Library;
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::sync::Arc;
use std::sync::OnceLock;
#[cfg(feature = "native")]
use wasmtime::{Engine, FuncType, Linker, Val, ValType};

/// libm f64 functions linked by link_ffi_functions, with their arity.
/// Fallback when system headers declare them via macros (glibc __MATHCALL).
pub const LIBM_F64_FUNCTIONS: [(&str, usize); 15] = [
    ("fmin", 2), ("fmax", 2), ("fabs", 1), ("floor", 1), ("ceil", 1), ("round", 1), ("sqrt", 1),
    ("sin", 1), ("cos", 1), ("tan", 1), ("fmod", 2), ("pow", 2), ("exp", 1), ("log", 1), ("log10", 1),
];

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

/// FFI functions run in the program's one state, with the host functions and WASI
#[cfg(feature = "native")]
pub use crate::host::HostState as FfiState;

mod header;
mod link;
pub use header::*;
pub use link::*;

const SDL_PREFIX: &str = "SDL_";
/// The libc functions link_libc_functions defines by hand; any other libc import is linked from the process
const HAND_LINKED_LIBC: [&str; 8] = ["abs", "strlen", "atoi", "atol", "atof", "strcmp", "strncmp", "rand"];


/// Get FFI signatures by parsing header files for a library, once per process: every name the analyzer meets is looked up here
pub fn get_signatures_from_headers(library: &str) -> &'static HashMap<String, FfiSignature> {
    static PARSED: std::sync::Mutex<Vec<(String, &'static HashMap<String, FfiSignature>)>> = std::sync::Mutex::new(Vec::new());
    let mut parsed = PARSED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, signatures)) = parsed.iter().find(|(parsed_library, _)| parsed_library == library) {
        return signatures;
    }
    let signatures: &'static HashMap<String, FfiSignature> = Box::leak(Box::new(parse_signatures_from_headers(library)));
    parsed.push((library.to_string(), signatures));
    signatures
}

/// The error of a call nothing resolves: a libc function says how to import it
pub fn undefined_function_message(name: &str) -> String {
    match get_ffi_signature_from_lib(name, "c") {
        Some(_) => format!("{name} is a C function: write `use c` or `import {name} from \"c\"`"),
        None => format!("undefined function: {name}"),
    }
}

/// What a C pointer type crosses as (notes/ffi_handles.md)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CPointer {
    /// `char *`, `const unsigned char *`: a wasp text (a parameter its NUL-terminated letters in linear memory)
    Text,
    /// `void *`, `int *`: a parameter points into linear memory; a result is a handle
    Memory,
    /// `sqlite3 *`, `FILE *`: an id into the run's handle table (CHandles), 0 is NULL
    Handle,
    /// `sqlite3 **ppDb`: left out of the wasp call; the first one becomes the result, the others receive NULL
    Out,
}

/// C words that qualify a type without naming it
const C_QUALIFIERS: [&str; 7] = ["const", "volatile", "struct", "enum", "union", "unsigned", "signed"];

/// The type a C pointer type points to: the last word before its `*` (`char` of `SQLITE_API const unsigned char *`,
/// `FILE` of `FILE *`), "" for none (`unsigned *`)
fn pointee(c_type: &str) -> String {
    let before_star = &c_type[..c_type.find('*').unwrap_or(c_type.len())];
    before_star.split_whitespace().filter(|word| !C_QUALIFIERS.contains(word)).last().unwrap_or_default().to_string()
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

/// The C parameters a wasp call passes: all but the out-pointers
fn wasp_parameters(param_types: &[String]) -> impl Iterator<Item = &String> {
    param_types.iter().filter(|t| pointer_kind(t) != Some(CPointer::Out))
}

/// What a C function's result crosses as: a text, a handle (a pointer result or the first out-pointer), else None for
/// its number
fn pointer_result(hsig: &FfiHeaderSignature) -> Option<CPointer> {
    let has_out_pointer = hsig.param_types.iter().any(|t| pointer_kind(t) == Some(CPointer::Out));
    match pointer_kind(&hsig.return_type) {
        Some(CPointer::Text) => Some(CPointer::Text),
        Some(_) => Some(CPointer::Handle),
        None => has_out_pointer.then_some(CPointer::Handle),
    }
}

/// The wasm result of a C function: a text Node, a handle id, else its number
fn header_result(hsig: &FfiHeaderSignature) -> Option<wasm_encoder::ValType> {
    match pointer_result(hsig) {
        Some(CPointer::Text) => Some(wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)),
        Some(_) => Some(wasm_encoder::ValType::I32),
        None => map_c_type_to_valtype(&hsig.return_type),
    }
}

fn parse_signatures_from_headers(library: &str) -> HashMap<String, FfiSignature> {
    let mut sigs = HashMap::new();
    let paths = get_library_header_paths(library);

    // all headers first: their struct types decide how pointers cross (is_struct_type)
    let header_sigs: Vec<FfiHeaderSignature> = paths.iter().flat_map(|path| parse_header_file(path, library)).collect();
    for hsig in header_sigs {
        if let Some(ffi_sig) = header_sig_to_ffi_sig(&hsig) {
            sigs.insert(hsig.name.clone(), ffi_sig);
        }
    }

    sigs
}

/// Get known FFI function signatures by parsing system header files, once per process
/// Uses unified Kind/Signature types from ffi_parser module
pub fn get_ffi_signatures() -> &'static HashMap<String, FfiSignature> {
    static PARSED: OnceLock<HashMap<String, FfiSignature>> = OnceLock::new();
    PARSED.get_or_init(parse_ffi_signatures)
}

fn parse_ffi_signatures() -> HashMap<String, FfiSignature> {
    use crate::ffi_parser::get_all_signatures;
    use crate::function::kind_to_valtype;

    let mut sigs = HashMap::new();

    // Convert from ffi_parser's FfiFunction to FfiSignature
    for (name, func) in get_all_signatures() {
        // Convert Kind parameters to ValType (kind_to_valtype returns wasm_encoder::ValType)
        let params: Vec<wasm_encoder::ValType> = func.signature.parameters.iter()
            .map(|p| kind_to_valtype(p.kind))
            .collect();

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
    for (name, arity) in LIBM_F64_FUNCTIONS {
        sigs.entry(name.to_string())
            .or_insert_with(|| FfiSignature::new(name, "m", vec![ValType::F64; arity], vec![ValType::F64]));
    }
    // `ln(x)` is the natural logarithm: libm's log under the name the program calls
    sigs.insert("ln".to_string(), FfiSignature::new("log", "m", vec![ValType::F64], vec![ValType::F64]));
    sigs.insert(
        "rand".to_string(),
        FfiSignature {
            name: "rand",
            library: "c",
            params: vec![],
            results: vec![ValType::I32],
        },
    );
    for (name, params, results) in crate::host::host_word_signatures() {
        sigs.insert(name.to_string(), FfiSignature::new(name, crate::host::HOST_LIBRARY, params, results));
    }
    use crate::wasm_emitter::linear_arrays::{linear_word_signatures, LINEAR_LIBRARY};
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
    let declared = |lib: &&str| get_signatures_from_headers(lib).get(name).cloned();
    // a C string result as its header declares it (getenv's text), where the cached table has it as a number
    let text_result = |sig: &FfiSignature| matches!(sig.results.first(), Some(wasm_encoder::ValType::Ref(_)));
    if let Some(sig) = implicit_header_libraries(name).iter().filter_map(declared).find(text_result) {
        return Some(sig);
    }
    // the cached signatures of well-known libraries first (wasp's own `random` is no libc random), then the headers
    get_ffi_signatures().get(name).cloned().or_else(|| implicit_header_libraries(name).iter().find_map(declared))
}

/// The libraries whose headers an undeclared call `name(…)` is looked up in: libm and libc, SDL2 only for SDL_ names,
/// so a program that never names SDL never reads its headers
pub fn implicit_header_libraries(name: &str) -> &'static [&'static str] {
    if name.starts_with(SDL_PREFIX) { &["m", "c", "SDL2"] } else { &["m", "c"] }
}

/// Get FFI signature from a specific library's headers
pub fn get_ffi_signature_from_lib(name: &str, library: &str) -> Option<FfiSignature> {
    let library = resolve_library_alias(library);
    let built_in = get_ffi_signatures().get(name).filter(|sig| sig.library == library).cloned();
    // the hand-linked libc functions and libm keep their wasm-adapted signatures (strcmp of two texts)
    if library == "m" || (library == "c" && HAND_LINKED_LIBC.contains(&name)) {
        if built_in.is_some() {
            return built_in;
        }
    }
    // any other function is linked from its header declaration (link_single_function), so it is imported as declared
    get_signatures_from_headers(library).get(name).cloned().or(built_in)
}

/// Link FFI functions into a wasmtime linker


/// Resolve library alias to canonical name
pub fn resolve_library_alias(alias: &str) -> &'static str {
    match alias {
        "m" | "math" | "libm" => "m",
        "c" | "libc" => "c",
        "SDL2" | "sdl2" | "sdl" => "SDL2",
        "z" | "zlib" => "z",
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
