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

/// libm f64 functions linked by link_ffi_functions, with their arity.
/// Fallback when system headers declare them via macros (glibc __MATHCALL).
/// A libm name missing here compiled to its last argument (`hypot(3, 4)` was 4): the browser host takes them from Math.
pub const LIBM_F64_FUNCTIONS: [(&str, usize); 27] = [
    ("fmin", 2), ("fmax", 2), ("fabs", 1), ("floor", 1), ("ceil", 1), ("round", 1), ("sqrt", 1),
    ("sin", 1), ("cos", 1), ("tan", 1), ("fmod", 2), ("pow", 2), ("exp", 1), ("log", 1), ("log10", 1),
    ("asin", 1), ("acos", 1), ("atan", 1), ("atan2", 2), ("sinh", 1), ("cosh", 1), ("tanh", 1), ("hypot", 2),
    ("log2", 1), ("trunc", 1), ("log1p", 1), ("expm1", 1),
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

/// The error of a call nothing resolves at `call`; a word of a standard module offers to add its `use` line
pub fn undefined_function_diagnostic(call: &crate::node::Node, name: &str) -> crate::diagnostic::Diagnostic {
    let diagnostic = crate::diagnostic::Diagnostic::at(call, undefined_function_message(name));
    match crate::modules::std_module_defining(name).filter(|module| crate::modules::module_file_shadowing(module).is_none()) {
        Some(module) => diagnostic.offering(crate::fixits::added_first_line(format!("{name} from the standard module {module}"), format!("use {module}"))),
        None => diagnostic,
    }
}

/// The error of a call of `import name from 'library'` whose headers are missing or do not declare it
pub fn unresolved_import_message(name: &str, library: &str) -> String {
    let headers = crate::ffi_parser::find_library_headers(library);
    match headers.is_empty() {
        true => format!("{name} is imported from {library}, but no header of {library} is found in the include directories"),
        false => format!("{name} is imported from {library}, but its headers do not declare it: {}", headers.join(", ")),
    }
}

/// The error of a call nothing resolves, always "undefined function: name", then where the name is found: a standard
/// module or a libc function says how to bring it in
pub fn undefined_function_message(name: &str) -> String {
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
    if get_ffi_signature_from_lib(name, "c").is_some() {
        return Some(format!("{name} is a C function: write `use c` or `import {name} from \"c\"`"));
    }
    // only f64 functions link by themselves (analyzer imports.rs is_f64_header_function): an int parameter may be a
    // pointer (frexp's int *), which the import's author knows
    get_ffi_signature_from_lib(name, "m").map(|_| format!("{name} is a libm function with a parameter other than a float: write `import {name} from \"m\"`"))
}

/// What a C pointer type crosses as (notes/ffi_handles.md)
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

/// The C parameters a warp call passes: all but the out-pointers
fn warp_parameters(param_types: &[String]) -> impl Iterator<Item = &String> {
    param_types.iter().filter(|t| pointer_kind(t) != Some(CPointer::Out))
}

/// What a C function's result crosses as: a text, a handle (a pointer result or the first out-pointer), else None for
/// its number
pub(crate) fn pointer_result(hsig: &FfiHeaderSignature) -> Option<CPointer> {
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
    let texts = get_library_header_paths(library).iter().flat_map(|path| parse_header_file(path, library))
        .map(|declared| (declared.name.clone(), warp_parameters(&declared.param_types).map(|c_type| pointer_kind(c_type) == Some(CPointer::Text)).collect()))
        .collect();
    let texts: &'static HashMap<String, Vec<bool>> = Box::leak(Box::new(texts));
    parsed.push((library.to_string(), texts));
    texts
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

    for (name, func) in get_all_signatures() {
        let params = func.signature.parameters.iter().map(|p| kind_to_valtype(p.kind)).collect();
        let results = func.signature.return_types.iter().map(|k| kind_to_valtype(*k)).collect();
        let library = match func.library.as_str() {
            "m" => "m",
            "c" => "c",
            "SDL2" => "SDL2",
            library => Box::leak(library.to_string().into_boxed_str()),
        };
        let signature = FfiSignature::new(Box::leak(name.clone().into_boxed_str()), library, params, results);
        sigs.insert(name, signature);
    }

    for (name, params, result) in LIBC_SIGNATURES {
        sigs.insert(name.to_string(), FfiSignature::new(name, "c", params.to_vec(), vec![result]));
    }
    for (name, arity) in LIBM_F64_FUNCTIONS {
        sigs.entry(name.to_string())
            .or_insert_with(|| FfiSignature::new(name, "m", vec![wasm_encoder::ValType::F64; arity], vec![wasm_encoder::ValType::F64]));
    }
    // `ln(x)` is the natural logarithm: libm's log under the name the program calls
    sigs.insert("ln".to_string(), FfiSignature::new("log", "m", vec![wasm_encoder::ValType::F64], vec![wasm_encoder::ValType::F64]));
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
    // the cached signatures of well-known libraries first (warp's own `random` is no libc random), then the headers
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
    if (library == "m" || (library == "c" && HAND_LINKED_LIBC.contains(&name)))
        && built_in.is_some() {
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
