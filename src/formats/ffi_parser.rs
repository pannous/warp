// FFI header discovery: where the headers of a library are

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
const LIBC_HEADERS: [&str; 8] =
	["string.h", "_string.h", "stdlib.h", "_stdlib.h", "stdio.h", "_stdio.h", "ctype.h", "_ctype.h"];
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

/// The one C header parser is ffi/header.rs (card cleanup-ffi-headers)
pub use crate::ffi::{extract_function_signature as parse_declaration, parse_header_file};
