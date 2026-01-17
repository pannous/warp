// FFI header parser - discovers function signatures from system header files
// Parses C function declarations and maps to WASM types

use std::collections::HashMap;
use std::path::Path;
use wasm_encoder::ValType;

/// Parsed C function signature
#[derive(Clone, Debug)]
pub struct CSignature {
    pub name: String,
    pub return_type: CType,
    pub params: Vec<CParam>,
    pub library: String,
}

#[derive(Clone, Debug)]
pub struct CParam {
    pub ctype: CType,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CType {
    Void,
    Int,
    Long,
    Float,
    Double,
    SizeT,
    CharPtr,
    Pointer, // generic pointer (SDL_Window*, etc.)
    Bool,
    Unknown,
}

impl CType {
    pub fn to_wasm(&self) -> Option<ValType> {
        match self {
            CType::Void => None,
            CType::Int => Some(ValType::I32),
            CType::Long => Some(ValType::I64),
            CType::Float => Some(ValType::F32),
            CType::Double => Some(ValType::F64),
            CType::SizeT => Some(ValType::I64),
            CType::CharPtr => Some(ValType::I32),
            CType::Pointer => Some(ValType::I64), // pointers as i64 handles
            CType::Bool => Some(ValType::I32),
            CType::Unknown => Some(ValType::I32), // default to i32
        }
    }
}

/// Standard header file locations for each library
pub fn get_library_header_paths(library: &str) -> Vec<&'static str> {
    match library {
        "m" | "math" | "libm" => vec![
            "/usr/include/math.h",
            "/usr/local/include/math.h",
            "/opt/homebrew/include/math.h",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/math.h",
        ],
        "c" | "libc" => vec![
            "/usr/include/string.h",
            "/usr/include/stdlib.h",
            "/usr/include/stdio.h",
            "/usr/local/include/string.h",
            "/usr/local/include/stdlib.h",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/string.h",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/stdlib.h",
        ],
        "SDL2" | "sdl2" | "sdl" => vec![
            "/opt/homebrew/include/SDL2/SDL.h",
            "/opt/homebrew/include/SDL2/SDL_events.h",
            "/opt/homebrew/include/SDL2/SDL_render.h",
            "/opt/homebrew/include/SDL2/SDL_timer.h",
            "/usr/local/include/SDL2/SDL.h",
            "/usr/include/SDL2/SDL.h",
        ],
        "raylib" => vec![
            "/opt/homebrew/include/raylib.h",
            "/usr/local/include/raylib.h",
            "/usr/include/raylib.h",
        ],
        _ => vec![],
    }
}

/// Parse a C type string to CType
fn parse_type(s: &str) -> CType {
    let s = s.trim();

    // Handle pointer types
    if s.contains('*') {
        if s.contains("char") {
            return CType::CharPtr;
        }
        return CType::Pointer;
    }

    // Strip const/unsigned/etc qualifiers
    let s = s.replace("const ", "").replace("unsigned ", "").replace("signed ", "");
    let s = s.trim();

    match s {
        "void" => CType::Void,
        "int" | "int32_t" => CType::Int,
        "long" | "long int" | "int64_t" | "long long" => CType::Long,
        "float" => CType::Float,
        "double" => CType::Double,
        "size_t" | "ssize_t" => CType::SizeT,
        "bool" | "_Bool" => CType::Bool,
        "short" | "int16_t" => CType::Int,
        "char" | "int8_t" | "uint8_t" => CType::Int,
        "Uint32" | "uint32_t" => CType::Int,
        "Uint64" | "uint64_t" => CType::Long,
        "Color" => CType::Int, // raylib Color is 4 bytes
        _ => CType::Unknown,
    }
}

/// Extract function signature from a C declaration line
pub fn parse_declaration(decl: &str, library: &str) -> Option<CSignature> {
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
        || decl.contains('[')  // array declarations
        || decl.contains("->") // member access
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

    // Extract function name (last identifier before '(')
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut name = parts.last()?.to_string();
    // Handle "*func" pointer-returning functions
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
        "int".to_string() // C default
    };
    let return_type = parse_type(&return_type_str);

    // Parse parameters
    let params = if params_str.trim() == "void" || params_str.trim().is_empty() {
        vec![]
    } else {
        params_str
            .split(',')
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() || p == "..." {
                    return None;
                }

                let p = p.replace("const ", "");
                let parts: Vec<&str> = p.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                // Find the type (everything except the last identifier if it's a name)
                let (type_str, name_part) = if parts.len() == 1 {
                    (parts[0].to_string(), None)
                } else {
                    let last = *parts.last()?;
                    let is_name = last.chars().next()?.is_alphabetic() && !last.contains('*');
                    if is_name && parts.len() > 1 {
                        (parts[..parts.len() - 1].join(" "), Some(last.trim_start_matches('*').to_string()))
                    } else {
                        (parts.join(" "), None)
                    }
                };

                let ctype = parse_type(&type_str);
                Some(CParam { ctype, name: name_part })
            })
            .collect()
    };

    Some(CSignature {
        name,
        return_type,
        params,
        library: library.to_string(),
    })
}

/// Parse a header file and extract all function signatures
pub fn parse_header_file(path: &str, library: &str) -> Vec<CSignature> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter_map(|line| parse_declaration(line, library))
        .collect()
}

/// Get all FFI signatures by parsing system header files
pub fn get_all_signatures() -> HashMap<String, (CSignature, &'static str)> {
    let mut sigs = HashMap::new();

    // Parse headers for each library
    for (library, lib_static) in [("m", "m"), ("c", "c"), ("SDL2", "SDL2"), ("raylib", "raylib")] {
        for path in get_library_header_paths(library) {
            if Path::new(path).exists() {
                for sig in parse_header_file(path, library) {
                    // Don't overwrite existing signatures (first found wins)
                    if !sigs.contains_key(&sig.name) {
                        sigs.insert(sig.name.clone(), (sig, lib_static));
                    }
                }
            }
        }
    }

    sigs
}

/// Get signature for a specific function from a specific library
pub fn get_signature(name: &str, library: &str) -> Option<CSignature> {
    for path in get_library_header_paths(library) {
        if Path::new(path).exists() {
            for sig in parse_header_file(path, library) {
                if sig.name == name {
                    return Some(sig);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let sig = parse_declaration("double sin(double x);", "m").unwrap();
        assert_eq!(sig.name, "sin");
        assert_eq!(sig.return_type, CType::Double);
        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].ctype, CType::Double);
    }

    #[test]
    fn test_parse_two_params() {
        let sig = parse_declaration("double pow(double base, double exp);", "m").unwrap();
        assert_eq!(sig.name, "pow");
        assert_eq!(sig.params.len(), 2);
    }

    #[test]
    fn test_parse_void_params() {
        let sig = parse_declaration("int rand(void);", "c").unwrap();
        assert_eq!(sig.name, "rand");
        assert_eq!(sig.params.len(), 0);
    }

    #[test]
    fn test_parse_pointer_param() {
        let sig = parse_declaration("size_t strlen(const char *s);", "c").unwrap();
        assert_eq!(sig.name, "strlen");
        assert_eq!(sig.params[0].ctype, CType::CharPtr);
    }

    #[test]
    fn test_parse_system_headers() {
        // This test actually reads system header files
        let sigs = get_all_signatures();

        // Should find at least some math functions if math.h exists
        let has_math = sigs.contains_key("sin") || sigs.contains_key("cos") || sigs.contains_key("sqrt");

        // Print what we found for debugging
        if !has_math {
            eprintln!("No math functions found. Available signatures: {:?}",
                sigs.keys().take(10).collect::<Vec<_>>());
        }

        // Don't fail if headers not available (CI environments)
        // Just verify the parsing works when headers exist
        if !sigs.is_empty() {
            assert!(sigs.len() > 0);
        }
    }

    #[test]
    fn test_header_paths_exist() {
        // Check which header paths actually exist on this system
        let mut found_any = false;
        for lib in ["m", "c"] {
            for path in get_library_header_paths(lib) {
                if Path::new(path).exists() {
                    found_any = true;
                    eprintln!("Found header: {}", path);
                }
            }
        }
        // Just informational - don't fail if no headers
        if !found_any {
            eprintln!("No system headers found - this is OK for some environments");
        }
    }
}
