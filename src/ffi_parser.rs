// FFI header parser - discovers function signatures from system header files
// Uses unified Kind and Signature types from function.rs

use crate::function::{Arg, Signature};
use crate::type_kinds::Kind;

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
const LIBC_HEADERS: [&str; 8] = [
	"string.h",
	"_string.h",
	"stdlib.h",
	"_stdlib.h",
	"stdio.h",
	"_stdio.h",
	"ctype.h",
	"_ctype.h",
];
/// the page's libc is libc.wasm, its functions declared in one header
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
const LIBC_HEADERS: [&str; 1] = ["libc.h"];
/// Libraries whose header is not named after them: `use z` reads zlib.h
const LIBRARY_HEADERS: [(&str, &str); 1] = [("z", "zlib.h")];

pub(crate) fn include_dirs() -> Vec<String> {
	match std::env::var(INCLUDE_VARIABLE) {
		Ok(list) => list
			.split(':')
			.filter(|dir| !dir.is_empty())
			.map(str::to_string)
			.collect(),
		#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
		Err(_) => vec![PAGE_INCLUDE.to_string()],
		#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
		Err(_) => INCLUDE_DIRS.iter().map(|dir| dir.to_string()).collect(),
	}
}

/// The path of `header` (`math.h`, `SDL2/SDL_render.h`) in the first of `dirs` that holds it
pub fn find_header_in(header: &str, dirs: &[impl AsRef<str>]) -> Option<String> {
	dirs.iter()
		.map(|dir| format!("{}/{}", dir.as_ref(), header))
		.find(|path| crate::web::file_exists(path))
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
	headers
		.iter()
		.filter_map(|header| find_header_in(header, &dirs))
		.collect()
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

/// A C declaration as warp kinds; ffi/header.rs reads it, the one C header parser (card cleanup-ffi-headers)
pub fn parse_declaration(decl: &str, library: &str) -> Option<FfiFunction> {
	crate::ffi::extract_function_signature(decl, library).map(|declared| function_of(&declared))
}

/// The functions a header file declares, as warp kinds
pub fn parse_header_file(path: &str, library: &str) -> Vec<FfiFunction> {
	crate::ffi::parse_header_file(path, library)
		.iter()
		.map(function_of)
		.collect()
}

fn function_of(declared: &crate::ffi::FfiHeaderSignature) -> FfiFunction {
	let mut signature = Signature::new();
	let return_kind = parse_c_type(&declared.return_type);
	if return_kind != Kind::Empty {
		signature.return_types.push(return_kind);
	}
	for (index, c_type) in declared.param_types.iter().enumerate() {
		let name = declared
			.param_names
			.get(index)
			.cloned()
			.unwrap_or_else(|| format!("p{index}"));
		signature.parameters.push(Arg::new(name, parse_c_type(c_type)));
	}
	FfiFunction {
		name: declared.name.clone(),
		signature,
		library: declared.library.clone(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	/// libc's and libm's functions by name, as their headers declare them
	fn get_all_signatures() -> std::collections::HashMap<String, FfiFunction> {
		["c", "m"]
			.into_iter()
			.flat_map(|library| {
				find_library_headers(library)
					.into_iter()
					.map(move |path| (path, library))
			})
			.flat_map(|(path, library)| parse_header_file(&path, library))
			.map(|function| (function.name.clone(), function))
			.collect()
	}

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
		assert_eq!(init.signature.parameters[2].kind, Kind::Text); // title (char*)
		eprintln!("✓ InitWindow: void(i32, i32, string)");
	}
}
