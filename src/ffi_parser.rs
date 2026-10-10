// FFI header parser - discovers function signatures from system header files
// Uses unified Kind and Signature types from function.rs

use std::collections::HashMap;
use crate::type_kinds::Kind;
use crate::function::{Signature, Arg};

/// Include directories searched for headers, in order (a C compiler's order: the first holding a header wins)
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const INCLUDE_DIRS: &[&str] = &[
    "/opt/homebrew/include",
    "/usr/local/include",
    "/usr/include",
    // Debian and Ubuntu: glibc's bits/ headers, which math.h includes (bits/mathcalls.h declares cbrt, pow, …)
    "/usr/include/x86_64-linux-gnu",
    "/usr/include/aarch64-linux-gnu",
    "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include",
];
/// The page's include directory: the C headers of what its host provides (web/playground/lib/libc.h, P147)
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
const PAGE_INCLUDE: &str = "page:lib";
/// `:`-separated include directories replacing INCLUDE_DIRS (web/playground's test runner serves exactly one)
const INCLUDE_VARIABLE: &str = "WARP_INCLUDE";
const SDL_HEADERS: [&str; 5] = ["SDL.h", "SDL_events.h", "SDL_render.h", "SDL_timer.h", "SDL_video.h"];
/// libc's headers: strings, conversions and memory, stdio, character classes (`toupper`). macOS declares much of them
/// in the _stdlib.h, _stdio.h and _ctype.h that stdlib.h, stdio.h and ctype.h include (`getenv`, `fopen`, `toupper`); a header missing on Linux is skipped
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const LIBC_HEADERS: [&str; 8] = ["string.h", "_string.h", "stdlib.h", "_stdlib.h", "stdio.h", "_stdio.h", "ctype.h", "_ctype.h"];
/// the page's libc is libc.wasm, its functions declared in one header
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
const LIBC_HEADERS: [&str; 1] = ["libc.h"];
/// Libraries whose header is not named after them: `use z` reads zlib.h
const LIBRARY_HEADERS: [(&str, &str); 1] = [("z", "zlib.h")];

pub(crate) fn include_dirs() -> Vec<String> {
    match std::env::var(INCLUDE_VARIABLE) {
        Ok(list) => list.split(':').filter(|dir| !dir.is_empty()).map(str::to_string).collect(),
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        Err(_) => vec![PAGE_INCLUDE.to_string()],
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        Err(_) => INCLUDE_DIRS.iter().map(|dir| dir.to_string()).collect(),
    }
}

/// The path of `header` (`math.h`, `SDL2/SDL_render.h`) in the first of `dirs` that holds it
pub fn find_header_in(header: &str, dirs: &[impl AsRef<str>]) -> Option<String> {
    dirs.iter().map(|dir| format!("{}/{}", dir.as_ref(), header)).find(|path| crate::web::file_exists(path))
}

/// The headers that declare a library's functions, each found once in the include directories
pub fn find_library_headers(library: &str) -> Vec<String> {
    let headers: Vec<String> = match library {
        "m" | "math" | "cmath" | "libm" => vec!["math.h".into()],
        "c" | "libc" => LIBC_HEADERS.iter().map(|header| header.to_string()).collect(),
        "SDL2" | "sdl2" | "sdl" => SDL_HEADERS.iter().map(|header| format!("SDL2/{header}")).collect(),
        _ => match LIBRARY_HEADERS.iter().find(|(name, _)| *name == library) {
            Some((_, header)) => vec![header.to_string()],
            None => vec![format!("{library}.h"), format!("{library}/{library}.h")],
        },
    };
    let dirs = include_dirs();
    headers.iter().filter_map(|header| find_header_in(header, &dirs)).collect()
}

/// FFI function info - combines Signature with library metadata
#[derive(Debug, Clone)]
pub struct FfiFunction {
    pub name: String,
    pub signature: Signature,
    pub library: String,
}

impl FfiFunction {
    pub fn new(name: impl Into<String>, library: impl Into<String>) -> Self {
        FfiFunction {
            name: name.into(),
            signature: Signature::new(),
            library: library.into(),
        }
    }
}

/// Parse a C type string to Kind
fn parse_c_type(s: &str) -> Kind {
    Kind::from_c_type(s)
}

/// Extract function signature from a C declaration line
pub fn parse_declaration(decl: &str, library: &str) -> Option<FfiFunction> {
    let decl = decl.trim();

    // Skip non-function lines
    if decl.is_empty()
        || decl.starts_with("//")
        || decl.starts_with("/*")
        || decl.starts_with("*")
        || decl.starts_with("#")
        || decl.starts_with("typedef")
        || decl.starts_with("struct")
        || decl.starts_with("enum")
        || decl.starts_with("union")
        || decl.starts_with("return")
        || !decl.contains('(')
        || !decl.contains(')')
        || decl.contains('[')
        || decl.contains("->")
    {
        return None;
    }

    let decl = decl.split("//").next()?.trim();

    // Remove qualifiers
    let decl = decl
        .replace("extern \"C\"", "")
        .replace("extern ", "")
        .replace("static ", "")
        .replace("inline ", "")
        .replace("SDLCALL ", "")
        .replace("RLAPI ", "");
    let decl = decl.trim().trim_end_matches(';').trim();

    let paren_pos = decl.find('(')?;
    let close_paren = crate::ffi::matching_paren(decl, paren_pos)?;

    let before_paren = &decl[..paren_pos];
    let params_str = &decl[paren_pos + 1..close_paren];

    // Extract function name
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // the stars of `FILE *fopen(…)` belong to the return type
    let (name, pointer_marks) = crate::ffi::split_pointer_marks(parts.last()?);

    if name.is_empty() || !name.chars().next()?.is_alphabetic() || crate::ffi::C_TYPE_WORDS.contains(&name.as_str()) {
        return None;
    }

    let return_type_str = if parts.len() > 1 {
        format!("{} {pointer_marks}", parts[..parts.len() - 1].join(" "))
    } else {
        "int".to_string()
    };
    let return_kind = parse_c_type(&return_type_str);

    let mut sig = Signature::new();

    // Add return type
    if return_kind != Kind::Empty {
        sig.return_types.push(return_kind);
    }

    if params_str.trim() != "void" && !params_str.trim().is_empty() {
        for (i, p) in params_str.split(',').enumerate() {
            let p = p.trim();
            if p.is_empty() || p == "..." {
                continue;
            }

            let p = p.replace("const ", "");
            let parts: Vec<&str> = p.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Extract type and optional name
            let (type_str, param_name) = if parts.len() == 1 {
                (parts[0].to_string(), format!("p{}", i))
            } else {
                let last = *parts.last().unwrap();
                let is_name = last.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
                    && !last.contains('*');
                if is_name && parts.len() > 1 {
                    (parts[..parts.len() - 1].join(" "), last.trim_start_matches('*').to_string())
                } else {
                    (parts.join(" "), format!("p{}", i))
                }
            };

            let kind = parse_c_type(&type_str);
            sig.parameters.push(Arg::new(param_name, kind));
        }
    }

    Some(FfiFunction {
        name,
        signature: sig,
        library: library.to_string(),
    })
}

/// Parse a header file and extract all function signatures
pub fn parse_header_file(path: &str, library: &str) -> Vec<FfiFunction> {
    let Some(content) = crate::web::read_text(path) else { return vec![] };

    // block comments span lines, and their prose may read like a call (glibc's "because tolower (EOF) must be EOF")
    let mut in_comment = false;
    content
        .lines()
        .map(|line| crate::ffi::without_block_comments(line, &mut in_comment))
        .filter_map(|line| parse_declaration(&line, library))
        .collect()
}

/// Get all FFI signatures by parsing system headers for known libraries
pub fn get_all_signatures() -> HashMap<String, FfiFunction> {
    let mut sigs = HashMap::new();

    for library in ["m", "c", "SDL2"] {
        for path in find_library_headers(library) {
            for func in parse_header_file(&path, library) {
                if !sigs.contains_key(&func.name) {
                    sigs.insert(func.name.clone(), func);
                }
            }
        }
    }

    sigs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_simple_function() {
        let func = parse_declaration("double sin(double x);", "m").unwrap();
        assert_eq!(func.name, "sin");
        assert_eq!(func.signature.return_types, vec![Kind::Float]);
        assert_eq!(func.signature.parameters.len(), 1);
        assert_eq!(func.signature.parameters[0].kind, Kind::Float);
    }

    #[test]
    fn test_parse_two_params() {
        let func = parse_declaration("double pow(double base, double exp);", "m").unwrap();
        assert_eq!(func.name, "pow");
        assert_eq!(func.signature.parameters.len(), 2);
    }

    #[test]
    fn test_parse_void_params() {
        let func = parse_declaration("int rand(void);", "c").unwrap();
        assert_eq!(func.name, "rand");
        assert_eq!(func.signature.parameters.len(), 0);
    }

    #[test]
    fn test_parse_pointer_param() {
        let func = parse_declaration("size_t strlen(const char *s);", "c").unwrap();
        assert_eq!(func.name, "strlen");
        assert_eq!(func.signature.parameters[0].kind, Kind::Text); // char* -> Text
    }

    #[test]
    fn test_parse_int32_return() {
        let func = parse_declaration("int abs(int x);", "c").unwrap();
        assert_eq!(func.name, "abs");
        assert_eq!(func.signature.return_types, vec![Kind::Int32]);
        assert_eq!(func.signature.parameters[0].kind, Kind::Int32);
    }

    #[test]
    fn test_parse_system_headers() {
        let sigs = get_all_signatures();
        if !sigs.is_empty() {
            let has_math = sigs.contains_key("sin") || sigs.contains_key("cos");
            if !has_math {
                eprintln!("Available: {:?}", sigs.keys().take(10).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn test_raylib_functions() {
        let raylib_path = "/opt/homebrew/include/raylib.h";
        if !Path::new(raylib_path).exists() {
            eprintln!("raylib.h not found - skipping");
            return;
        }

        let funcs = parse_header_file(raylib_path, "raylib");
        eprintln!("Found {} raylib functions", funcs.len());

        let init = funcs.iter().find(|f| f.name == "InitWindow").unwrap();
        assert_eq!(init.signature.return_types, vec![]); // void
        assert_eq!(init.signature.parameters.len(), 3);
        assert_eq!(init.signature.parameters[0].kind, Kind::Int32); // width
        assert_eq!(init.signature.parameters[1].kind, Kind::Int32); // height
        assert_eq!(init.signature.parameters[2].kind, Kind::Text);  // title (char*)
        eprintln!("✓ InitWindow: void(i32, i32, string)");
    }
}
