//! GLSL as WGSL (card create-little): glsl_shader in lib/draw.warp runs shaders written for twigl.app and shadertoy.com.
//! naga reads the GLSL fragment shader and writes its functions as WGSL; its entry point is dropped and its `main`
//! renamed, so a WGSL shader around it (lib/draw.warp, with its own `@fragment fn main` and values) calls them.
//! Natively warp calls wgsl_functions (src/modules/std_adapters.rs draw); the playground loads this crate as glsl.wasm
//! (web/playground/build.sh, host-compiler.js glslWgsl) through the exports below.

/// What GLSL's own `main` is called in the WGSL, which keeps `main` for the shader's entry point
const GLSL_MAIN: &str = "warp_glsl_main";

/// How a block's hole `$name` (written `values.name`, src/graphics/shader_holes.rs) is named in GLSL, which has no
/// `values`: a float global of this prefix, which the WGSL reads from `values` again
const HOLE_PREFIX: &str = "warp_values_";
const VALUES: &str = "values.";
const VERSION_LINE: &str = "#version 450\n";

/// The functions of a GLSL 450 fragment shader as WGSL, or naga's errors pointing into the GLSL. A hole `values.name`
/// (`twigl { … $name … }`) is a float
pub fn wgsl_functions(glsl: &str) -> Result<String, String> {
	let (glsl, holes) = with_hole_globals(glsl);
	Ok(read_from_values(&translated(&glsl)?, &holes))
}

/// The GLSL with each `values.name` as the float global `warp_values_name`, declared after its #version line; the names
fn with_hole_globals(glsl: &str) -> (String, Vec<String>) {
	let mut names: Vec<String> = vec![];
	let mut text = String::new();
	let mut rest = glsl;
	while let Some(at) = rest.find(VALUES) {
		let after = &rest[at + VALUES.len()..];
		let length = after.find(|ch: char| !(ch.is_alphanumeric() || ch == '_')).unwrap_or(after.len());
		let is_word_start = !rest[..at].ends_with(|ch: char| ch.is_alphanumeric() || ch == '_' || ch == '.');
		text.push_str(&rest[..at]);
		match is_word_start && length > 0 {
			true => {
				let name = &after[..length];
				text.push_str(&format!("{HOLE_PREFIX}{name}"));
				if !names.iter().any(|known| known == name) {
					names.push(name.to_string());
				}
				rest = &after[length..];
			}
			false => {
				text.push_str(VALUES);
				rest = after;
			}
		}
	}
	text.push_str(rest);
	let declarations: String = names.iter().map(|name| format!("float {HOLE_PREFIX}{name};\n")).collect();
	let declared = match text.strip_prefix(VERSION_LINE) {
		Some(body) => format!("{VERSION_LINE}{declarations}{body}"),
		None => format!("{declarations}{text}"),
	};
	(declared, names)
}

/// The WGSL reading each hole's global from the shader's `values` instead, the globals' declarations dropped
fn read_from_values(wgsl: &str, holes: &[String]) -> String {
	if holes.is_empty() {
		return wgsl.to_string();
	}
	let kept: Vec<&str> =
		wgsl.lines().filter(|line| !line.starts_with(&format!("var<private> {HOLE_PREFIX}"))).collect();
	holes
		.iter()
		.fold(kept.join("\n"), |wgsl, name| wgsl.replace(&format!("{HOLE_PREFIX}{name}"), &format!("{VALUES}{name}")))
}

fn translated(glsl: &str) -> Result<String, String> {
	let options = naga::front::glsl::Options::from(naga::ShaderStage::Fragment);
	let mut module =
		naga::front::glsl::Frontend::default().parse(&options, glsl).map_err(|errors| errors.emit_to_string(glsl))?;
	module.entry_points.clear();
	for (_, function) in module.functions.iter_mut() {
		if function.name.as_deref() == Some("main") {
			function.name = Some(GLSL_MAIN.into());
		}
	}
	let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
		.validate(&module)
		.map_err(|error| error.emit_to_string(glsl))?;
	naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
		.map_err(|error| error.to_string())
}

/// glsl.wasm's exports: the page writes the GLSL into memory from `glsl_buffer(length)`, calls `glsl_wgsl(length)` and
/// reads that many bytes at `glsl_buffer(0)`: the WGSL, or the error after a leading NUL
#[cfg(target_arch = "wasm32")]
mod exports {
	use std::cell::RefCell;

	thread_local! {
		static BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
	}

	/// The buffer's address, at least `length` bytes long
	#[no_mangle]
	pub extern "C" fn glsl_buffer(length: usize) -> *mut u8 {
		BUFFER.with(|buffer| {
			let mut buffer = buffer.borrow_mut();
			if buffer.len() < length {
				buffer.resize(length, 0);
			}
			buffer.as_mut_ptr()
		})
	}

	/// The GLSL of the buffer's first `length` bytes translated into it; the byte length of the answer
	#[no_mangle]
	pub extern "C" fn glsl_wgsl(length: usize) -> usize {
		BUFFER.with(|buffer| {
			let mut buffer = buffer.borrow_mut();
			let glsl = String::from_utf8_lossy(&buffer[..length]).into_owned();
			let answer = match super::wgsl_functions(&glsl) {
				Ok(wgsl) => wgsl.into_bytes(),
				Err(error) => [&[0u8][..], error.as_bytes()].concat(),
			};
			*buffer = answer;
			buffer.len()
		})
	}
}
