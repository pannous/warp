// card wgsl-error: a WGSL error in a `shader { … }` block is a warp error at its line and column in the program, with
// the line underlined, not the line and column inside the shader the GPU reports
#![cfg(feature = "native")]

const BROKEN_PROGRAM: &str = "size = 8
broken = shader {
	@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
		return vec4f(level(at.x), 0.0, 0.0, 1.0);
	}
	fn level(band: f32) : f32 {
		return band / 8.0;
	}
}
paint(broken, size, size)";

#[test]
fn a_wgsl_error_names_its_place_in_the_program() {
	let run = crate::common::warp_command()
		.env("WARP_NO_WINDOW", "1")
		.args(["--no-ask", "eval", BROKEN_PROGRAM])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("at 6:22"), "{said}");
	assert!(said.contains("  6 | \tfn level(band: f32) : f32 {"), "{said}");
}

#[test]
fn a_shader_error_after_a_hole_counts_the_hole_as_written() {
	// the shader's line 1 is `let a = values.x; let b = c`, its column 27 the `c`
	let place =
		warp::graphics::shader_errors::place_in_program("x = 2\ns = shader {\n\t\tlet a = $x; let b = c\n\t}", 1, 27);
	assert_eq!(place, Some((3, 23)));
}

#[test]
fn a_shader_error_the_browser_wrapped_is_placed_too() {
	let code = "s = shader {\n\tfn f() : f32 { return 1.0; }\n}";
	assert_eq!(
		warp::graphics::shader_errors::relocated_message(
			"could not run the program: gpu_render: 1:8: expected '{' for function body",
			code
		)
		.as_deref(),
		Some("could not run the program: gpu_render: expected '{' for function body at 2:9")
	);
}
