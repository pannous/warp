// FFI header parser - discovers function signatures from system header files
// Uses unified Kind and Signature types from function.rs

use std::collections::HashMap;
use std::path::Path;
use crate::type_kinds::Kind;
use crate::function::{Signature, Arg};

/// Common include directories to search for headers
const INCLUDE_DIRS: &[&str] = &[
    "/opt/homebrew/include",
    "/usr/local/include",
    "/usr/include",
    "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include",
];

/// Find header files for a library by searching common locations
pub fn find_library_headers(library: &str) -> Vec<String> {
    match library {
        "m" | "math" | "libm" => find_existing_headers(&["math.h"]),
        "c" | "libc" => find_existing_headers(&["string.h", "stdlib.h", "stdio.h"]),
        "SDL2" | "sdl2" | "sdl" => {
            let mut paths = Vec::new();
            for dir in INCLUDE_DIRS {
                for header in ["SDL.h", "SDL_events.h", "SDL_render.h", "SDL_timer.h"] {
                    let path = format!("{}/SDL2/{}", dir, header);
                    if Path::new(&path).exists() {
                        paths.push(path);
                    }
                }
            }
            paths
        }
        _ => {
            // Generic: search for {library}.h
            let header_name = format!("{}.h", library);
            let mut paths = Vec::new();
            for dir in INCLUDE_DIRS {
                let path = format!("{}/{}", dir, header_name);
                if Path::new(&path).exists() {
                    paths.push(path);
                }
                let subdir_path = format!("{}/{}/{}", dir, library, header_name);
                if Path::new(&subdir_path).exists() {
                    paths.push(subdir_path);
                }
            }
            paths
        }
    }
}

fn find_existing_headers(headers: &[&str]) -> Vec<String> {
    let mut paths = Vec::new();
    for dir in INCLUDE_DIRS {
        for header in headers {
            let path = format!("{}/{}", dir, header);
            if Path::new(&path).exists() {
                paths.push(path);
            }
        }
    }
    paths
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

    // Remove trailing comment
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

    // Find parentheses
    let paren_pos = decl.find('(')?;
    let close_paren = decl.rfind(')')?;

    if paren_pos >= close_paren {
        return None;
    }

    let before_paren = &decl[..paren_pos];
    let params_str = &decl[paren_pos + 1..close_paren];

    // Extract function name
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut name = parts.last()?.to_string();
    while name.starts_with('*') {
        name = name[1..].to_string();
    }

    if name.is_empty() || !name.chars().next()?.is_alphabetic() {
        return None;
    }

    // Extract return type
    let return_type_str = if parts.len() > 1 {
        parts[..parts.len() - 1].join(" ")
    } else {
        "int".to_string()
    };
    let return_kind = parse_c_type(&return_type_str);

    // Build signature
    let mut sig = Signature::new();

    // Add return type
    if return_kind != Kind::Empty {
        sig.return_types.push(return_kind);
    }

    // Parse parameters
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
                let is_name = last.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
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
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter_map(|line| parse_declaration(line, library))
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

/// Get signature for a specific function by name
pub fn get_signature(name: &str, library: &str) -> Option<FfiFunction> {
    for path in find_library_headers(library) {
        for func in parse_header_file(&path, library) {
            if func.name == name {
                return Some(func);
            }
        }
    }
    None
}

/// Get all signatures from a specific library
pub fn get_library_signatures(library: &str) -> Vec<FfiFunction> {
    let mut sigs = Vec::new();
    for path in find_library_headers(library) {
        sigs.extend(parse_header_file(&path, library));
    }
    sigs
}

#[cfg(test)]
mod tests {
    use super::*;

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
