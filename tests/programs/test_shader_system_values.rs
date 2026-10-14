// card values-number (user): `$mouse_x` in a shader block failed "values.mouse_x is a number or a list of numbers, got
// mouse_x", which read as a suggestion. `$mouse.x` is the pointer's x (the built-in hole $mouse, a vec2f); `$mouse_x`
// works as its alias with a hint to write `$mouse.x` (user 2026-10-14: "mouse_x could just give a hint mouse.x")
use warp::normalize::{capture_hints, set_hint_mode, HintMode};

const SHADER: &str = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f(select(0.0, 1.0, $mouse.x == 0.0 && $mouse.y == 0.0), 0.0, 0.0, 1.0);\n\t}\n}\n";

#[test]
fn mouse_x_is_an_alias_of_mouse_dot_x_with_a_hint() {
	set_hint_mode(HintMode::Always);
	let aliased = SHADER.replace("$mouse.x", "$mouse_x");
	let (lowered, hints) = capture_hints(|| warp::pipeline::lower(&format!("{aliased}gpu_render(s, 2, 2)")));
	let lowered = lowered.expect("lowers").serialize();
	assert!(lowered.contains("values.mouse.x") && !lowered.contains("mouse_x"), "{lowered}");
	assert!(hints.iter().any(|hint| hint.original == "$mouse_x" && hint.canonical == "$mouse.x"), "{hints:?}");
}

#[cfg(feature = "native")]
#[test]
fn the_shader_gets_the_pointer() {
	for shader in [SHADER.to_string(), SHADER.replace("$mouse.x", "$mouse_x").replace("$mouse.y", "$mouse_y")] {
		let run = crate::common::warp_command()
			.env("WARP_NO_WINDOW", "1")
			.args(["--no-ask", "eval", &format!("use graphics\n{shader}gpu_render(s, 2, 2)[0]")])
			.output()
			.unwrap();
		let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
		if said.contains("no WebGPU adapter") {
			return crate::common::announce_skip("a WebGPU adapter", module_path!());
		}
		assert!(said.contains(&(0xFFFF_0000u32).to_string()), "{said}");
	}
}
