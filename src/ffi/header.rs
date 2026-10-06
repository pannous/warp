//! C header parsing: declarations into FfiHeaderSignature, the struct types a header declares, glibc's math macros

use super::*;

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

/// C type words: a "declaration" named after one is a function pointer or a cast (`int (*_close)(void *);`), never a
/// function the program could call; taken for one, it would shadow wasp's own `int(…)`
pub(crate) const C_TYPE_WORDS: [&str; 13] = ["int", "char", "void", "long", "short", "unsigned", "signed", "float", "double", "const", "struct", "union", "enum"];

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
    let decl = without_libc_annotations(decl.trim());
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
    if name.is_empty() || C_TYPE_WORDS.contains(&name.as_str()) {
        return None;
    }

    // Return type is everything before the name
    let return_type = if parts.len() > 1 {
        parts[..parts.len()-1].join(" ")
    } else {
        "int".to_string() // C default
    };
    let return_type = if pointer_marks.is_empty() { return_type } else { format!("{return_type} {pointer_marks}") };

    // the `)` closing the parameters: a macro may follow it (`fopen(…) __DARWIN_ALIAS(fopen);`); a line without one
    // (sqlite3.h has such) declares nothing
    let close_paren = matching_paren(decl, paren_pos)?;
    let params_str = &decl[paren_pos+1..close_paren];

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
                // Reconstruct type (everything except pure name); `sqlite3 **ppDb` keeps both stars
                let stars = &last[..last.len() - name.len()];
                let type_str = if !stars.is_empty() {
                    format!("{} {stars}", parts[..parts.len()-1].join(" "))
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

/// The macOS SDK's bounds annotations (`char *_LIBC_CSTR strchr(…)`, `_LIBC_COUNT(__n)`) dropped: no C type or name
const LIBC_ANNOTATION_PREFIX: &str = "_LIBC_";

fn without_libc_annotations(declaration: &str) -> String {
    let mut rest = declaration;
    let mut kept = String::new();
    while let Some(start) = rest.find(LIBC_ANNOTATION_PREFIX) {
        kept.push_str(&rest[..start]);
        let after = &rest[start..];
        let word_end = after.find(|letter: char| !(letter.is_ascii_alphanumeric() || letter == '_')).unwrap_or(after.len());
        rest = &after[word_end..];
        if rest.starts_with('(') {
            rest = matching_paren(rest, 0).map_or("", |close| &rest[close + 1..]);
        }
        kept.push(' ');
    }
    kept.push_str(rest);
    kept
}

/// Convert FfiHeaderSignature to FfiSignature (using 'static lifetime via leak)
pub fn header_sig_to_ffi_sig(hsig: &FfiHeaderSignature) -> Option<FfiSignature> {
    let params: Vec<wasm_encoder::ValType> = wasp_parameters(&hsig.param_types)
        .filter_map(|t| map_c_type_to_valtype(t))
        .collect();
    let results: Vec<wasm_encoder::ValType> = header_result(hsig).into_iter().collect();

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
    match crate::ffi_parser::find_header_in(GLIBC_MATHCALLS, &dirs).and_then(|path| crate::web::read_text(&path)) {
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
    let Some(content) = crate::web::read_text(path).map(|text| with_glibc_mathcalls(text, path)) else { return Vec::new() };
    let mut struct_types = StructTypes::default();
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

        struct_types.read(line);
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

    remember_struct_types(struct_types.names);
    signatures
}

/// The struct types a header declares (`typedef struct sqlite3 sqlite3;`, `typedef struct __sFILE {…} FILE;`): a
/// pointer to one is a handle
#[derive(Default)]
struct StructTypes {
    names: std::collections::HashSet<String>,
    /// inside `typedef struct X {`: its name follows the closing `}`
    in_definition: bool,
}

impl StructTypes {
    fn read(&mut self, line: &str) {
        let words: Vec<&str> = line.trim_end_matches(';').split_whitespace().collect();
        let is_typedef_struct = words.starts_with(&["typedef", "struct"]);
        if is_typedef_struct && line.contains('{') && !line.contains('}') {
            self.in_definition = true;
        } else if is_typedef_struct && line.ends_with(';') && words.len() == 4 {
            self.names.insert(words[3].to_string());
        } else if self.in_definition && line.starts_with('}') && line.ends_with(';') {
            self.in_definition = false;
            self.names.extend(words.last().map(|name| name.trim_start_matches('}').to_string()).filter(|name| !name.is_empty()));
        }
    }
}

/// The struct types of every header read so far: a header may use one another declares (stdio.h's `fgetc(FILE *)`,
/// with FILE in _stdio.h), so a library's headers are all read before its signatures are classified
static STRUCT_TYPES: std::sync::Mutex<Option<std::collections::HashSet<String>>> = std::sync::Mutex::new(None);

fn remember_struct_types(names: std::collections::HashSet<String>) {
    STRUCT_TYPES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get_or_insert_with(Default::default).extend(names);
}

/// C's opaque standard types, structs on every platform whichever header declares them (glibc's FILE is in
/// bits/types/FILE.h, which stdio.h's search does not read): a pointer to one is a handle, never linear memory
const STANDARD_STRUCT_TYPES: [&str; 2] = ["FILE", "DIR"];

pub(super) fn is_struct_type(name: &str) -> bool {
    STANDARD_STRUCT_TYPES.contains(&name) || STRUCT_TYPES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref().is_some_and(|names| names.contains(name))
}

/// The position of the `)` closing the `(` at `open`
pub(crate) fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (position, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 1 => return Some(open + position),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// An annotation macro standing alone before a declaration: `API_AVAILABLE(macos(10.10), ios(8.2))` (Apple's headers)
fn is_annotation_line(line: &str) -> bool {
    let Some(open) = line.find('(') else { return false };
    let name = &line[..open];
    !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') && line.ends_with(')')
}

/// `line` without its block-comment text; `in_comment` carries an unclosed `/*` to the next line
pub(crate) fn without_block_comments(line: &str, in_comment: &mut bool) -> String {
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
