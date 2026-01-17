// Minimal C header parser for FFI function signatures
// Parses simplified C function declarations and maps to WASM types

use std::collections::HashMap;
use wasm_encoder::ValType;

/// Parsed C function signature
#[derive(Clone, Debug)]
pub struct CSignature {
    pub name: String,
    pub return_type: CType,
    pub params: Vec<CParam>,
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
    CharPtr,  // char* or const char*
    IntPtr,   // int*
}

impl CType {
    /// Convert C type to WASM ValType
    pub fn to_wasm(&self) -> Option<ValType> {
        match self {
            CType::Void => None,
            CType::Int => Some(ValType::I32),
            CType::Long => Some(ValType::I64),
            CType::Float => Some(ValType::F32),
            CType::Double => Some(ValType::F64),
            CType::SizeT => Some(ValType::I64),
            CType::CharPtr => Some(ValType::I32),
            CType::IntPtr => Some(ValType::I32),
        }
    }

    /// Convert C type to Rust type string for extern "C"
    pub fn to_rust(&self) -> &'static str {
        match self {
            CType::Void => "()",
            CType::Int => "i32",
            CType::Long => "i64",
            CType::Float => "f32",
            CType::Double => "f64",
            CType::SizeT => "usize",
            CType::CharPtr => "*const i8",
            CType::IntPtr => "*const i32",
        }
    }
}

/// Parse a C type string
fn parse_type(s: &str) -> Option<CType> {
    let s = s.trim();

    if s.contains('*') {
        if s.contains("char") {
            return Some(CType::CharPtr);
        }
        return Some(CType::IntPtr);
    }

    let s = s.strip_prefix("const").map(str::trim).unwrap_or(s);

    match s {
        "void" => Some(CType::Void),
        "int" => Some(CType::Int),
        "long" | "long int" => Some(CType::Long),
        "float" => Some(CType::Float),
        "double" => Some(CType::Double),
        "size_t" => Some(CType::SizeT),
        _ => None,
    }
}

/// Parse a single function declaration
pub fn parse_declaration(decl: &str) -> Option<CSignature> {
    let decl = decl.trim().trim_end_matches(';').trim();
    let paren_pos = decl.find('(')?;
    let close_paren = decl.rfind(')')?;

    let before_paren = &decl[..paren_pos];
    let params_str = &decl[paren_pos + 1..close_paren];

    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let name = parts.last()?.to_string();
    let return_type_str = parts[..parts.len() - 1].join(" ");
    let return_type = parse_type(&return_type_str)?;

    let params = if params_str.trim() == "void" || params_str.trim().is_empty() {
        vec![]
    } else {
        params_str
            .split(',')
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    return None;
                }

                let parts: Vec<&str> = p.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let last = *parts.last()?;
                let is_not_name = last.starts_with('*') ||
                    last.chars().next().map(|c| !c.is_alphabetic()).unwrap_or(true);

                let (type_parts, name_part) = if is_not_name {
                    (parts.as_slice(), None)
                } else if parts.len() > 1 {
                    (&parts[..parts.len() - 1], Some(last.to_string()))
                } else {
                    (parts.as_slice(), None)
                };

                let type_str = type_parts.join(" ");
                let ctype = parse_type(&type_str)?;

                Some(CParam { ctype, name: name_part })
            })
            .collect()
    };

    Some(CSignature {
        name,
        return_type,
        params,
    })
}

/// Parse multiple declarations from a string
pub fn parse_header(content: &str) -> Vec<CSignature> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                return None;
            }
            parse_declaration(line)
        })
        .collect()
}

/// Built-in header definitions
pub mod headers {
    pub const LIBM: &str = r#"
double fmin(double x, double y);
double fmax(double x, double y);
double fabs(double x);
double floor(double x);
double ceil(double x);
double round(double x);
double sqrt(double x);
double sin(double x);
double cos(double x);
double tan(double x);
double asin(double x);
double acos(double x);
double atan(double x);
double atan2(double y, double x);
double sinh(double x);
double cosh(double x);
double tanh(double x);
double fmod(double x, double y);
double pow(double base, double exp);
double exp(double x);
double log(double x);
double log10(double x);
double log2(double x);
double cbrt(double x);
double hypot(double x, double y);
"#;

    pub const LIBC: &str = r#"
int abs(int x);
long labs(long x);
size_t strlen(const char *s);
int atoi(const char *s);
long atol(const char *s);
double atof(const char *s);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
int rand(void);
void srand(int seed);
"#;
}

/// Get all FFI signatures from built-in headers
pub fn get_all_signatures() -> HashMap<String, (CSignature, &'static str)> {
    let mut sigs = HashMap::new();
    for sig in parse_header(headers::LIBM) {
        sigs.insert(sig.name.clone(), (sig, "m"));
    }
    for sig in parse_header(headers::LIBC) {
        sigs.insert(sig.name.clone(), (sig, "c"));
    }
    sigs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let sig = parse_declaration("double sin(double x);").unwrap();
        assert_eq!(sig.name, "sin");
        assert_eq!(sig.return_type, CType::Double);
        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].ctype, CType::Double);
    }

    #[test]
    fn test_parse_two_params() {
        let sig = parse_declaration("double pow(double base, double exp);").unwrap();
        assert_eq!(sig.name, "pow");
        assert_eq!(sig.params.len(), 2);
    }

    #[test]
    fn test_parse_void_params() {
        let sig = parse_declaration("int rand(void);").unwrap();
        assert_eq!(sig.name, "rand");
        assert_eq!(sig.params.len(), 0);
    }

    #[test]
    fn test_parse_pointer_param() {
        let sig = parse_declaration("size_t strlen(const char *s);").unwrap();
        assert_eq!(sig.name, "strlen");
        assert_eq!(sig.params[0].ctype, CType::CharPtr);
    }

    #[test]
    fn test_parse_header() {
        let sigs = parse_header(headers::LIBM);
        assert!(sigs.len() > 10);
        assert!(sigs.iter().any(|s| s.name == "sin"));
        assert!(sigs.iter().any(|s| s.name == "pow"));
    }
}
