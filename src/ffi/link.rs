//! Linking C functions into wasmtime: libm, any library through its headers, the hand-linked libc functions and the
//! calls themselves from warp-runtime (c_calls.rs, which executables link too)

use super::*;
pub use warp_runtime::c_calls::{get_or_load_library, text_node, CHandles};

/// Library functions that open a native window: refused under WARP_NO_WINDOW or CI, so no test or agent run pops one up
const WINDOW_OPENERS: [&str; 4] = ["InitWindow", "SDL_CreateWindow", "SDL_CreateWindowAndRenderer", "glfwCreateWindow"];

// Extern C Functions and Hardcoded Signatures (Fallback)

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
	fn asin(x: f64) -> f64;
	fn acos(x: f64) -> f64;
	fn atan(x: f64) -> f64;
	fn atan2(y: f64, x: f64) -> f64;
	fn sinh(x: f64) -> f64;
	fn cosh(x: f64) -> f64;
	fn tanh(x: f64) -> f64;
	fn hypot(x: f64, y: f64) -> f64;
	fn log2(x: f64) -> f64;
	fn trunc(x: f64) -> f64;
	fn log1p(x: f64) -> f64;
	fn expm1(x: f64) -> f64;

}

/// libm functions linked under the import module "m".
/// Deliberately hand-linked FFI examples (user decision 2026-10-03, kept as marked examples): the explicit form of what
/// the header-driven path (get_signatures_from_headers + link_dynamic_library) does by reflection; keep the table.
/// link_libm uses it only when the headers declare nothing for "m" (glibc's __MATHCALL macros): headers first.
const LIBM_UNARY: [(&str, unsafe extern "C" fn(f64) -> f64); 21] = [
	("fabs", fabs),
	("floor", floor),
	("ceil", ceil),
	("round", round),
	("sqrt", sqrt),
	("sin", sin),
	("cos", cos),
	("tan", tan),
	("exp", exp),
	("log", log),
	("log10", log10),
	("asin", asin),
	("acos", acos),
	("atan", atan),
	("sinh", sinh),
	("cosh", cosh),
	("tanh", tanh),
	("log2", log2),
	("trunc", trunc),
	("log1p", log1p),
	("expm1", expm1),
];
const LIBM_BINARY: [(&str, unsafe extern "C" fn(f64, f64) -> f64); 6] =
	[("fmin", fmin), ("fmax", fmax), ("fmod", fmod), ("pow", pow), ("atan2", atan2), ("hypot", hypot)];

pub fn link_ffi_functions(linker: &mut Linker<FfiState>, engine: &Engine) -> Result<()> {
	link_libm(linker, engine, &libm_header_signatures())?;
	Ok(warp_runtime::c_calls::link_libc_functions(linker)?)
}

/// Where the libm functions of a run come from
#[derive(Debug, PartialEq)]
pub enum LibmSource {
	/// that many functions declared by the system headers
	Headers(usize),
	/// the hand-linked LIBM_UNARY/LIBM_BINARY
	Table,
}

/// The libm functions the system headers declare (none from glibc, which declares them through macros)
pub fn libm_header_signatures() -> Vec<FfiHeaderSignature> {
	crate::formats::ffi_parser::find_library_headers(LIBM)
		.iter()
		.flat_map(|header| parse_header_file(header, LIBM))
		.collect()
}

/// libm from its headers first; the hand-linked table only when they declare nothing for "m" (user decision 2026-10-03)
pub fn link_libm(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	header_signatures: &[FfiHeaderSignature],
) -> Result<LibmSource> {
	let linked = match get_or_load_library(LIBM) {
		Some(library) => header_signatures
			.iter()
			.filter(|signature| link_single_function(linker, engine, LIBM, &library, signature).is_ok())
			.count(),
		None => 0,
	};
	if linked > 0 {
		return Ok(LibmSource::Headers(linked));
	}
	link_libm_table(linker)?;
	Ok(LibmSource::Table)
}

fn link_libm_table(linker: &mut Linker<FfiState>) -> Result<()> {
	use warp_runtime::floats::canonical_nan;
	for (name, function) in LIBM_UNARY {
		linker.func_wrap("m", name, move |x: f64| unsafe { function(canonical_nan(x)) })?;
	}
	for (name, function) in LIBM_BINARY {
		linker.func_wrap("m", name, move |x: f64, y: f64| unsafe { function(canonical_nan(x), canonical_nan(y)) })?;
	}
	Ok(())
}

/// Link the functions of a library the module imports, as its headers declare them
pub fn link_dynamic_library(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	lib_name: &str,
	imported: &std::collections::HashSet<String>,
) -> Result<usize> {
	// the module imports under the name the program wrote (`use zlib`), the library and its headers go by their own (z)
	let Some(library) = get_or_load_library(resolve_library_alias(lib_name)) else {
		return Ok(0);
	};
	Ok(header_c_functions(lib_name, imported)
		.into_iter()
		.filter(|function| link_c_function(linker, engine, &library, function.clone()).is_ok())
		.count())
}

/// Link a single function from a parsed header signature, imported from `module`
fn link_single_function(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	module: &str,
	library: &Arc<Library>,
	sig: &FfiHeaderSignature,
) -> Result<()> {
	link_c_function(linker, engine, library, c_function_of(module, sig))
}

fn link_c_function(
	linker: &mut Linker<FfiState>,
	engine: &Engine,
	library: &Library,
	function: CFunction,
) -> Result<()> {
	let pointer = warp_runtime::c_calls::function_pointer(library, &function.name)?;
	Ok(warp_runtime::c_calls::link_c_function(linker, engine, function, pointer)?)
}

/// Link all dynamic libraries required by a WASM module's imports
/// Scans the module for import statements and links matching libraries via reflection
pub fn link_module_libraries(linker: &mut Linker<FfiState>, engine: &Engine, module: &wasmtime::Module) -> Result<()> {
	use std::collections::HashSet;

	// Collect unique library names from imports
	let mut libs_to_link: HashMap<String, HashSet<String>> = HashMap::new();

	for import in module.imports() {
		let module_name = import.module();
		// Skip built-in libraries that are already linked; libc beyond its built-in functions (toupper) is linked here
		let built_in_libc = matches!(module_name, "c" | "libc") && HAND_LINKED_LIBC.contains(&import.name());
		let wasm_module = crate::modules::wasm_modules::is_module_path(module_name); // linked by wasm_modules::link
		if !built_in_libc
			&& !wasm_module
			&& !matches!(module_name, "m" | "libm" | "env" | "wasi_snapshot_preview1" | crate::host::HOST_LIBRARY)
		{
			libs_to_link.entry(module_name.to_string()).or_default().insert(import.name().to_string());
		}
	}

	refuse_headless_windows(&libs_to_link)?;
	// Link each discovered library
	for (lib_name, imported) in libs_to_link {
		match link_dynamic_library(linker, engine, &lib_name, &imported) {
			Ok(count) if count > 0 => {}
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

fn refuse_headless_windows(libs_to_link: &HashMap<String, std::collections::HashSet<String>>) -> Result<()> {
	if !crate::graphics::paint::headless() {
		return Ok(());
	}
	for (lib_name, imported) in libs_to_link {
		if let Some(opener) = WINDOW_OPENERS.iter().find(|opener| imported.contains(**opener)) {
			anyhow::bail!(
				"{opener} from {lib_name} opens a window: refused under {} or CI",
				crate::graphics::paint::NO_WINDOW_VARIABLE
			);
		}
	}
	Ok(())
}
