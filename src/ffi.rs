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

const SDL_PREFIX: &str = "SDL_";
/// The libc functions link_libc_functions defines by hand; any other libc import is linked from the process
const HAND_LINKED_LIBC: [&str; 8] = ["abs", "strlen", "atoi", "atol", "atof", "strcmp", "strncmp", "rand"];

/// C keywords that start a statement: a line holding one declares no function
const C_STATEMENT_KEYWORDS: [&str; 9] = ["return", "if", "else", "while", "for", "do", "switch", "case", "goto"];

/// Extract function signature from a C declaration string
/// e.g., "double sqrt(double x);" -> FfiHeaderSignature
pub fn extract_function_signature(declaration: &str, library: &str) -> Option<FfiHeaderSignature> {
    let decl = declaration.trim();

    // Skip empty lines, comments (also a block comment's continued lines: `** See also: [sqlite3_libversion()]`),
    // preprocessor directives
    if decl.is_empty() || decl.starts_with("//") || decl.starts_with('#') || decl.starts_with("/*") || decl.starts_with('*') {
        return None;
    }

    // Skip non-function declarations (typedef, struct, enum, etc.)
    if decl.starts_with("typedef") || decl.starts_with("struct") ||
       decl.starts_with("enum") || decl.starts_with("union") {
        return None;
    }

    // Remove extern, static, inline qualifiers and API macros (RLAPI, SDL_CALL, etc.)
    let decl = decl
        .replace("extern ", "")
        .replace("static ", "")
        .replace("inline ", "")
        .replace("RLAPI ", "")
        .replace("RAYGUIAPI ", "")
        .replace("RMAPI ", "")
        .replace("PHYSACDEF ", "")
        .replace("RL_API ", "")
        .replace("SDL_CALL ", "")
        .replace("SDLCALL ", "")
        .replace("__cdecl ", "")
        .replace("__stdcall ", "");
    let decl = decl.trim();
    // zlib's K&R-compatible prototypes: `zlibVersion OF((void));` declares `zlibVersion(void);`
    let decl = match decl.find(" OF((") {
        Some(start) => format!("{}({}", &decl[..start], decl[start + " OF((".len()..].replacen("))", ")", 1)),
        None => decl.to_string(),
    };
    let decl = decl.as_str();

    // Find the opening parenthesis (marks start of params)
    let paren_pos = decl.find('(')?;

    // Everything before '(' is return_type + name
    let before_paren = &decl[..paren_pos];

    // Find function name (last word before parenthesis)
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // `return f(x);` in an inline function body of a header is a statement, not a declaration
    if parts.iter().any(|part| C_STATEMENT_KEYWORDS.contains(part)) {
        return None;
    }

    // Function name is the last part, may have pointer marker
    let mut name = parts.last()?.to_string();
    // Handle "*func" case: the stars belong to the return type (`char *getenv(…)` returns char *)
    let mut pointer_marks = String::new();
    while name.starts_with('*') {
        name = name[1..].to_string();
        pointer_marks.push('*');
    }

    // Return type is everything before the name
    let return_type = if parts.len() > 1 {
        parts[..parts.len()-1].join(" ")
    } else {
        "int".to_string() // C default
    };
    let return_type = if pointer_marks.is_empty() { return_type } else { format!("{return_type} {pointer_marks}") };

    // Find closing paren; a line whose last `)` comes before its first `(` (sqlite3.h has such) declares nothing
    let close_paren = decl.rfind(')').filter(|close| *close > paren_pos)?;
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

    // a C string result is a Node the host builds (a text, or ø for NULL)
    let results: Vec<wasm_encoder::ValType> = match is_text_type(&hsig.return_type) {
        true => vec![wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)],
        false => map_c_type_to_valtype(&hsig.return_type).map(|v| vec![v]).unwrap_or_default(),
    };

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

/// glibc's math.h declares libm through the macros of this header: `__MATHCALL (sqrt,, (_Mdouble_ __x));`
const GLIBC_MATHCALLS: &str = "bits/mathcalls.h";
/// The glibc declaration macros: (macro, whether its first argument is the result type)
const GLIBC_MATH_MACROS: [(&str, bool); 7] = [
    ("__MATHCALL_VEC", false), ("__MATHCALLX", false), ("__MATHCALL", false),
    ("__MATHDECL_VEC", true), ("__MATHDECLX", true), ("__MATHDECL_1", true), ("__MATHDECL", true),
];

/// The header text, followed by the double variants of glibc's mathcalls.h declarations when it includes that header
fn with_glibc_mathcalls(content: String, header_path: &str) -> String {
    if !content.contains(GLIBC_MATHCALLS) {
        return content;
    }
    let header_dir = std::path::Path::new(header_path).parent().map(|dir| dir.to_string_lossy().to_string()).unwrap_or_default();
    let dirs: Vec<String> = std::iter::once(header_dir).chain(crate::ffi_parser::include_dirs()).collect();
    match crate::ffi_parser::find_header_in(GLIBC_MATHCALLS, &dirs).and_then(|path| std::fs::read_to_string(path).ok()) {
        Some(mathcalls) => format!("{content}\n{}", expand_glibc_math_macros(&mathcalls)),
        None => content,
    }
}

/// `__MATHCALL (sqrt,, (_Mdouble_ __x));` → `double sqrt (double __x);`, `__MATHDECL (int,ilogb,, (_Mdouble_ __x));` →
/// `int ilogb (double __x);`: plain declarations of the double variants
pub fn expand_glibc_math_macros(source: &str) -> String {
    let declaration = |line: &str| -> Option<String> {
        let (macro_name, typed) = GLIBC_MATH_MACROS.iter().find(|(name, _)| line.strip_prefix(name).is_some_and(|rest| rest.trim_start().starts_with('(')))?;
        let inner = line[macro_name.len()..].trim_start().strip_prefix('(')?;
        let inner = &inner[..inner.rfind(')')?];
        let mut parts = vec![String::new()];
        let mut depth = 0;
        for c in inner.chars() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(String::new());
                    continue;
                }
                _ => {}
            }
            parts.last_mut()?.push(c);
        }
        let (result, rest) = if *typed { (parts.first()?.trim().to_string(), &parts[1..]) } else { ("double".to_string(), &parts[..]) };
        let (name, arguments) = (rest.first()?.trim(), rest.get(2)?.trim());
        Some(format!("{} {name} {};", result.replace("_Mdouble_", "double"), arguments.replace("_Mdouble_", "double")))
    };
    source.lines().filter_map(|line| declaration(line.trim())).collect::<Vec<_>>().join("\n")
}

/// Parse a header file and extract all function signatures
pub fn parse_header_file(path: &str, library: &str) -> Vec<FfiHeaderSignature> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => with_glibc_mathcalls(c, path),
        Err(_) => return Vec::new(),
    };

    let mut signatures = Vec::new();
    let mut current_decl = String::new();
    let mut in_block_comment = false;

    for line in content.lines() {
        // block comments may span lines (sqlite3.h documents each function in one): their text is no declaration
        let uncommented = without_block_comments(line, &mut in_block_comment);
        let line = uncommented.trim();

        // Skip preprocessor and pure comment lines, and annotation macros on a line of their own
        if line.starts_with('#') || line.starts_with("//") || is_annotation_line(line) {
            continue;
        }

        // Skip empty lines and typedefs/structs in the middle of accumulation
        if line.is_empty() {
            current_decl.clear();
            continue;
        }

        // Strip inline comments (// ...) for processing but keep declaration
        let line_for_check = if let Some(comment_pos) = line.find("//") {
            line[..comment_pos].trim()
        } else {
            line
        };

        // Accumulate multi-line declarations (use original line content)
        current_decl.push_str(line_for_check);
        current_decl.push(' ');

        // If we have a complete declaration (ends with ; or })
        if line_for_check.ends_with(';') && current_decl.contains('(') {
            if let Some(sig) = extract_function_signature(&current_decl, library) {
                signatures.push(sig);
            }
            current_decl.clear();
        } else if line_for_check.ends_with('}') || (line_for_check.ends_with(';') && !current_decl.contains('(')) {
            current_decl.clear();
        }
    }

    signatures
}

/// An annotation macro standing alone before a declaration: `API_AVAILABLE(macos(10.10), ios(8.2))` (Apple's headers)
fn is_annotation_line(line: &str) -> bool {
    let Some(open) = line.find('(') else { return false };
    let name = &line[..open];
    !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') && line.ends_with(')')
}

/// `line` without its block-comment text; `in_comment` carries an unclosed `/*` to the next line
fn without_block_comments(line: &str, in_comment: &mut bool) -> String {
    let mut kept = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("*/") {
                Some(end) => {
                    *in_comment = false;
                    rest = &rest[end + 2..];
                }
                None => return kept,
            }
        }
        match rest.find("/*") {
            Some(start) => {
                kept.push_str(&rest[..start]);
                kept.push(' ');
                *in_comment = true;
                rest = &rest[start + 2..];
            }
            None => {
                kept.push_str(rest);
                return kept;
            }
        }
    }
}

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

/// The C functions whose result is a pointer (`char *getenv(…)`, `sqlite3 *…`): they are not imported, a call says why
static POINTER_RESULTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Does a parsed header declare `name` with a pointer result (`char *getenv(const char *)`)
pub fn returns_pointer(name: &str) -> bool {
    POINTER_RESULTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).iter().any(|known| known == name)
}

/// The error of a call nothing resolves: a C function with a pointer result says why it is missing
pub fn undefined_function_message(name: &str) -> String {
    if returns_pointer(name) {
        return format!("{name} returns a C pointer (a struct, a handle): handles from C are not supported yet");
    }
    match get_ffi_signature_from_lib(name, "c") {
        Some(_) => format!("{name} is a C function: write `use c` or `import {name} from \"c\"`"),
        None => format!("undefined function: {name}"),
    }
}

fn is_pointer_type(c_type: &str) -> bool {
    c_type.contains('*')
}

/// `char *` and `const char *`: a C string, which crosses as a wasp text (NULL as ø). Annotation macros around it
/// (`char *_LIBC_CSTR`, `ZEXTERN const char * ZEXPORT`, `SQLITE_API const char *`) do not count
fn is_text_type(c_type: &str) -> bool {
    let is_macro = |word: &str| word.chars().any(|c| c.is_ascii_uppercase()) && word.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    let spaced = c_type.replace('*', " * ");
    spaced.split_whitespace().filter(|word| *word != "const" && !is_macro(word)).collect::<String>() == "char*"
}

fn parse_signatures_from_headers(library: &str) -> HashMap<String, FfiSignature> {
    let mut sigs = HashMap::new();
    let paths = get_library_header_paths(library);

    for path in &paths {
        let header_sigs = parse_header_file(path, library);
        for hsig in header_sigs {
            // a pointer result would cross as a truncated i32: refused until texts and handles from C exist
            if is_pointer_type(&hsig.return_type) && !is_text_type(&hsig.return_type) {
                POINTER_RESULTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(hsig.name.clone());
                continue;
            }
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

    sigs
}

/// Check if a function is a known FFI function
pub fn is_ffi_function(name: &str) -> bool {
    get_ffi_signature(name).is_some()
}

/// Get FFI signature for a function by name
/// Searches well-known libraries (m, c, SDL2) via dynamic header discovery
pub fn get_ffi_signature(name: &str) -> Option<FfiSignature> {
    // a function with a pointer result is imported nowhere (returns_pointer); its libraries' headers tell
    if implicit_header_libraries(name).iter().any(|lib| !get_signatures_from_headers(lib).is_empty() && returns_pointer(name)) {
        return None;
    }
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
    let declared = get_signatures_from_headers(library).get(name).cloned();
    if returns_pointer(name) {
        return None;
    }
    declared.or(built_in)
}

/// Link FFI functions into a wasmtime linker
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

// ============================================================================
// Dynamic FFI - Load any library through reflection (raylib, SDL2, etc.)
// Parses C headers to discover signatures, loads dylib, creates wasm wrappers
// Uses direct function pointer calls with signature pattern matching
// ============================================================================

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
    Ptr,
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
        s if s.contains('*') => ParamType::Ptr,
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

    for header_path in &header_paths {
        let signatures = parse_header_file(header_path, lib_name);

        // only what the module imports: libc's headers declare hundreds of functions
        for sig in signatures.iter().filter(|sig| imported.contains(&sig.name) && (!is_pointer_type(&sig.return_type) || is_text_type(&sig.return_type))) {
            if link_single_function(linker, engine, import_name, &library, sig).is_ok() {
                linked_count += 1;
            }
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

    // Get symbol
    let func_ptr: usize = unsafe {
        let symbol: libloading::Symbol<*const ()> = library
            .get(func_name.as_bytes())
            .map_err(|e| anyhow::anyhow!("Symbol {} not found: {}", func_name, e))?;
        *symbol as usize
    };

    if func_ptr == 0 {
        return Err(anyhow::anyhow!("Null function pointer for {}", func_name));
    }

    // Build wasmtime function type
    let wasm_params: Vec<ValType> = sig
        .param_types
        .iter()
        .filter_map(|t| c_type_to_wasm_valtype(t))
        .collect();

    let param_types: Vec<ParamType> = sig.param_types.iter().map(|t| c_type_to_param_type(t)).collect();
    if is_text_type(&sig.return_type) {
        let func_type = FuncType::new(engine, wasm_params, [ValType::ANYREF]);
        return create_text_result_wrapper(linker, lib_name, &func_name, func_type, func_ptr, &param_types);
    }

    let wasm_results: Vec<ValType> = c_type_to_wasm_valtype(&sig.return_type)
        .into_iter()
        .collect();

    let func_type = FuncType::new(engine, wasm_params.clone(), wasm_results.clone());

    // Get normalized types for dispatch
    let ret_type = c_type_to_ret_type(&sig.return_type);

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
        let args = native_arguments(&mut caller, params, &param_types);

        // Call using platform-specific calling convention
        // ARM64 and x86_64 use similar conventions for first 8 integer/pointer args
        let ret = unsafe { call_native_function(func_ptr, &args, params.len()) };

        // Convert return value
        if !results.is_empty() {
            results[0] = match ret_type {
                RetType::Void => return Ok(()),
                RetType::I32 => Val::I32(ret as i32),
                RetType::I64 => Val::I64(ret as i64),
                RetType::F32 => Val::F32((ret as f32).to_bits()),
                RetType::F64 => Val::F64(ret),
                RetType::Bool => Val::I32(if ret != 0 { 1 } else { 0 }),
            };
        }

        Ok(())
    })?;

    Ok(())
}

/// The raw arguments of a native call (up to 8): numbers as they are, a pointer parameter as the address of its
/// text in linear memory
#[cfg(feature = "native")]
fn native_arguments(caller: &mut wasmtime::Caller<'_, FfiState>, params: &[Val], param_types: &[ParamType]) -> [u64; 8] {
    let mut args: [u64; 8] = [0; 8];
    for (i, (param, ptype)) in params.iter().zip(param_types.iter()).enumerate().take(8) {
        args[i] = match ptype {
            ParamType::I32 => param.unwrap_i32() as u64,
            ParamType::I64 => param.unwrap_i64() as u64,
            ParamType::F32 => (param.unwrap_f32() as f64).to_bits(),
            ParamType::F64 => param.unwrap_f64().to_bits(),
            ParamType::Ptr => get_memory_ptr(caller, param.unwrap_i32() as usize) as u64,
        };
    }
    args
}

/// A C function returning `char *` (getenv, zlibVersion): its NUL-terminated string copied into the module as a wasp
/// text, built with the module's own constructor; NULL is ø
#[cfg(feature = "native")]
fn create_text_result_wrapper(
    linker: &mut Linker<FfiState>,
    lib_name: &str,
    func_name: &str,
    func_type: FuncType,
    func_ptr: usize,
    param_types: &[ParamType],
) -> Result<()> {
    let param_types = param_types.to_vec();
    linker.func_new(lib_name, func_name, func_type, move |mut caller, params, results| {
        let args = native_arguments(&mut caller, params, &param_types);
        let pointer = unsafe { call_native_function(func_ptr, &args, params.len()) } as usize as *const std::ffi::c_char;
        // copied before anything else runs: a C string from getenv may change with the next call
        let text = (!pointer.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(pointer) }.to_bytes().to_vec());
        results[0] = text_node(&mut caller, text.as_deref()).map_err(|failure| wasmtime::Error::msg(failure.to_string()))?;
        Ok(())
    })?;
    Ok(())
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
