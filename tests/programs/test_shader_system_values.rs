// card values-number (user): `$mouse_x` in a shader block failed "values.mouse_x is a number or a list of numbers, got
// mouse_x", which read as a suggestion: the hole's entry `mouse_x: mouse_x` kept the word as data (data stays data,
// system_values.rs). A hole naming a system value reads it, as `$mouse.x` reads the built-in hole
const SHADER: &str = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f(select(0.0, 1.0, $mouse_x == 0.0 && $mouse_y == 0.0), 0.0, 0.0, 1.0);\n\t}\n}\n";

#[test]
fn a_hole_naming_a_system_value_reads_it() {
	let lowered = warp::pipeline::lower(&format!("{SHADER}gpu_render(s, 2, 2)")).expect("lowers").serialize();
	assert!(lowered.contains("system_value") && !lowered.contains("mouse_x:mouse_x}"), "{lowered}");
}

#[cfg(feature = "native")]
#[test]
fn the_shader_gets_the_pointer_of_a_system_value_hole() {
	let run = crate::common::warp_command()
		.env("WARP_NO_WINDOW", "1")
		.args(["--no-ask", "eval", &format!("use graphics\n{SHADER}gpu_render(s, 2, 2)[0]")])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains(&(0xFFFF_0000u32).to_string()), "{said}");
}
