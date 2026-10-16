// card compute-toys: compute_toy(code) of lib/compute_toys.warp runs a single-pass compute.toys shader, a WGSL kernel
// main_image writing textureStore(screen, id.xy, color), as a fragment shader for paint and gpu_render; and The Book of
// Shaders' $u_time, $u_resolution and $u_mouse are aliases of the built-in holes
#![cfg(feature = "native")]

/// What `warp eval` of the program says, stdout then stderr
fn said(program: &str) -> String {
	let run = crate::common::warp_command()
		.args(["--no-ask", "eval", &format!("use compute_toys\n{program}")])
		.output()
		.unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn a_compute_toy_paints_each_pixel_with_rows_from_the_top() {
	// red where id.x is 0, blue in the top row (compute.toys counts rows from the top), green when the screen is 2 wide
	let program = "s = compute_toy(shader {
	@compute @workgroup_size(16, 16)
	fn main_image(@builtin(global_invocation_id) id: vec3u) {
		let screen_size = textureDimensions(screen);
		if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }
		let wide = select(0.0, 1.0, screen_size.x == 2u);
		textureStore(screen, vec2i(id.xy), float4(select(0.0, 1.0, id.x == 0u), wide, select(0.0, 1.0, id.y == 0u), 1.0));
	}
})
pixels = gpu_render(s, 2, 2)
[pixels#1 == 0xFFFFFFFF, pixels#4 == 0xFF00FF00]";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes]"), "{said}");
}

// the reviewer's case: a kernel storing at an offset cannot run as a fragment, so its pixels are magenta, not wrong
#[test]
fn a_compute_toy_storing_elsewhere_paints_magenta() {
	let program = "s = compute_toy(shader {
	@compute @workgroup_size(16, 16)
	fn main_image(@builtin(global_invocation_id) id: vec3u) {
		textureStore(screen, id.xy + vec2u(1u, 0u), float4(0.0, 1.0, 0.0, 1.0));
	}
})
pixels = gpu_render(s, 2, 2)
[pixels#1 == 0xFFFF00FF, pixels#4 == 0xFFFF00FF]";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes]"), "{said}");
}

#[test]
fn a_compute_toy_with_passes_says_it_needs_their_engine() {
	let said = said("compute_toy(\"#storage particles array<float4>\\n@compute @workgroup_size(16, 16)\\nfn main_image(@builtin(global_invocation_id) id: vec3u) {}\")");
	assert!(said.contains("compute_toy cannot run this shader") && said.contains("#storage"), "{said}");
}

#[test]
fn the_book_of_shaders_names_are_holes() {
	let said = said("s = shader { @fragment fn main() -> @location(0) vec4f { return vec4f($u_time, $u_resolution / $u_mouse, 1.0); } }\n[\"values.time\" in s, \"values.size\" in s, \"values.mouse\" in s]");
	assert!(said.contains("[yes yes yes]"), "{said}");
}
