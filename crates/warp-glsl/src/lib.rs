//! GLSL as WGSL (card create-little): glsl_shader in lib/draw.warp runs shaders written for twigl.app and shadertoy.com.
//! naga reads the GLSL fragment shader and writes its functions as WGSL; its entry point is dropped and its `main`
//! renamed, so a WGSL shader around it (lib/draw.warp, with its own `@fragment fn main` and values) calls them.
//! Natively warp calls wgsl_functions (src/modules/std_adapters.rs draw); the playground loads this crate as glsl.wasm
//! (web/playground/build.sh, host-compiler.js glslWgsl) through the exports below.

/// What GLSL's own `main` is called in the WGSL, which keeps `main` for the shader's entry point
const GLSL_MAIN: &str = "warp_glsl_main";

/// The functions of a GLSL 450 fragment shader as WGSL, or naga's errors pointing into the GLSL
pub fn wgsl_functions(glsl: &str) -> Result<String, String> {
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
