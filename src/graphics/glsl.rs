//! GLSL as WGSL (card create-little): twigl(code) in lib/draw.warp runs shaders written for twigl.app and Shadertoy-like
//! sites. naga reads the GLSL fragment shader and writes its functions as WGSL; its entry point is dropped and its
//! `main` renamed, so a WGSL shader around it (lib/draw.warp, with its own `@fragment fn main` and values) calls them.
//! The same natively (std_io draw glsl_wgsl) and in the playground (warp.wasm web_glsl_wgsl, host-gpu.js).

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
	let flags = naga::valid::ValidationFlags::all();
	let info = naga::valid::Validator::new(flags, naga::valid::Capabilities::all())
		.validate(&module)
		.map_err(|error| error.emit_to_string(glsl))?;
	naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
		.map_err(|error| error.to_string())
}
