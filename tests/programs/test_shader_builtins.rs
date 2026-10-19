// card shader-builtins (user 2026-10-10): $width, $height, $size, $time, $frame, $mouse, $mouse_down and $key are there
// in every shader block without warp variables of those names: the host fills them at each paint or gpu_render
// (src/graphics/gpu.rs with_builtin_values, host-gpu.js); a program's own variable of such a name, or a values map entry, wins
const SHADER: &str = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f($width / 255.0, $height / 255.0, select(0.0, 1.0, $time >= 0.0 && $time < 60.0 && $size.x == $width && $mouse.x == 0.0 && $mouse_down == 0.0 && $key == 0.0 && $frame >= 0.0), 1.0);\n\t}\n}\n";

#[cfg(feature = "native")]
#[test]
fn the_host_fills_the_builtin_holes() {
	let run = crate::common::warp_command()
		.args(["--no-ask", "eval", &format!("use graphics\n{SHADER}gpu_render(s, 4, 2)[0]")])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains(&(0xFF04_02FFu32).to_string()), "{said}");
}

// card shader-read (user wish): $previous(at) is the color the shader left at that pixel in its previous frame of the
// same size, kept on the GPU (black before the first): painting with memory, as a feedback texture. Each render here
// adds a quarter of red to what was there
#[cfg(feature = "native")]
#[test]
fn a_shader_reads_its_previous_frame() {
	let shader = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn $previous(at.xy) + vec4f(0.25, 0.0, 0.0, 1.0);\n\t}\n}\n";
	let run = crate::common::warp_command()
		.args([
			"--no-ask",
			"eval",
			&format!("use graphics\n{shader}gpu_render(s, 2, 2)\ngpu_render(s, 2, 2)\ngpu_render(s, 2, 2)[1]"),
		])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	// kept as 8 bits per channel: 64, then 128, then 192 = 0xC0
	assert!(said.contains(&(0xFFC0_0000u32).to_string()), "{said}");
}

// card shader-read: $last, $feedback and $before are words for $previous (user's Interviewer 2026-10-12, canonical word
// plus hinting aliases): left unassigned, the error names $previous(at)
#[cfg(feature = "native")]
#[test]
fn a_word_for_the_previous_frame_hints_at_previous() {
	let shader = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn $last(at.xy);\n\t}\n}\n";
	let run = crate::common::warp_command()
		.args(["--no-ask", "eval", &format!("use graphics\n{shader}gpu_render(s, 2, 2)")])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	assert!(said.contains("$previous(at)"), "{said}");
}
