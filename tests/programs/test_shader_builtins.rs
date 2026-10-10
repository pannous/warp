// card shader-builtins (user 2026-10-10): $width, $height, $size, $time, $frame, $mouse, $mouse_down and $key are there
// in every shader block without warp variables of those names: the host fills them at each paint or gpu_render
// (src/gpu.rs with_builtin_values, host-gpu.js); a program's own variable of such a name, or a values map entry, wins
const SHADER: &str = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f($width / 255.0, $height / 255.0, select(0.0, 1.0, $time >= 0.0 && $time < 60.0 && $size.x == $width && $mouse.x == 0.0 && $mouse_down == 0.0 && $key == 0.0 && $frame >= 0.0), 1.0);\n\t}\n}\n";

#[test]
fn builtin_holes_are_left_to_the_host() {
	let lowered = warp::pipeline::lower(&format!("{SHADER}paint(s, 4, 2)")).expect("lowers").serialize();
	assert!(lowered.contains("(paint s 4 2)"), "{lowered}");
	let own = warp::pipeline::lower(&format!("width = 9.0\n{SHADER}paint(s, 4, 2)")).expect("lowers").serialize();
	assert!(own.contains("(paint s 4 2 {width:width})"), "{own}");
}

#[cfg(feature = "native")]
#[test]
fn the_host_fills_the_builtin_holes() {
	let run = crate::common::warp_command().args(["--no-ask", "eval", &format!("use graphics\n{SHADER}gpu_render(s, 4, 2)[0]")]).output().unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains(&(0xFF04_02FFu32).to_string()), "{said}");
}
