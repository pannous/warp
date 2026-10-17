// card shader-wheel: the mouse wheel is an input next to the pointer: the system value mouse_wheel and the shader's
// built-in hole $mouse_wheel are the steps turned over the paint window (or the page's canvas) since the start, up
// adding; `$wheel` is its alias with a hint, and compute_toy's mouse.zoom follows it, 5% a step, as compute.toys zooms
use warp::normalize::{capture_hints, set_hint_mode, HintMode};

const READS_THE_WHEEL: &str = "s = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f(select(0.0, 1.0, $mouse_wheel == 2.0), 0.0, 0.0, 1.0);\n\t}\n}\n";

#[test]
fn wheel_is_an_alias_of_mouse_wheel_with_a_hint() {
	set_hint_mode(HintMode::Always);
	let aliased = READS_THE_WHEEL.replace("$mouse_wheel", "$wheel");
	let (lowered, hints) = capture_hints(|| warp::pipeline::lower(&format!("{aliased}gpu_render(s, 2, 2)")));
	let lowered = lowered.expect("lowers").serialize();
	assert!(lowered.contains("values.mouse_wheel"), "{lowered}");
	assert!(hints.iter().any(|hint| hint.original == "$wheel" && hint.canonical == "$mouse_wheel"), "{hints:?}");
}

// the viewer tells the steps turned with a `wheel` line (src/graphics/paint_window.rs follow_input)
#[cfg(feature = "native")]
#[test]
fn the_wheel_over_the_paint_window_is_mouse_wheel() {
	let _window = crate::common::window_input_held();
	warp::graphics::paint_window::follow_input("wheel 3\nwheel -2\n".as_bytes());
	crate::is!("mouse_wheel", -2);
	warp_runtime::system_values::tell_wheel(0);
}

/// What `warp eval` of the program says, stdout then stderr; None without a WebGPU adapter
#[cfg(feature = "native")]
fn rendered(program: &str) -> Option<String> {
	let run = crate::common::warp_command()
		.env("WARP_NO_WINDOW", "1")
		.args(["--no-ask", "eval", &format!("use graphics\nuse compute_toys\n{program}")])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	match said.contains("no WebGPU adapter") {
		true => None,
		false => Some(said),
	}
}

#[cfg(feature = "native")]
#[test]
fn a_shader_reads_the_wheel() {
	let red = 0xFFFF_0000u32.to_string();
	let Some(given) = rendered(&format!("{READS_THE_WHEEL}gpu_render(s, 2, 2, {{mouse_wheel: 2}})[0]")) else {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	};
	assert!(given.contains(&red), "{given}");
	// no window, no wheel turned: the host fills 0
	let unturned = rendered(&format!("{READS_THE_WHEEL}gpu_render(s, 2, 2)[0]")).unwrap();
	assert!(unturned.contains(&0xFF00_0000u32.to_string()), "{unturned}");
}

#[cfg(feature = "native")]
#[test]
fn compute_toys_zoom_follows_the_wheel() {
	let toy = "s = compute_toy(shader {
	@compute @workgroup_size(16, 16)
	fn main_image(@builtin(global_invocation_id) id: vec3u) {
		let zoomed = abs(mouse.zoom - pow(1.05, 4.0)) < 0.001;
		textureStore(screen, id.xy, float4(select(0.0, 1.0, zoomed), 0.0, 0.0, 1.0));
	}
})
gpu_render(s, 2, 2, {mouse_wheel: 4})[0]";
	let Some(said) = rendered(toy) else {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	};
	assert!(said.contains(&0xFFFF_0000u32.to_string()), "{said}");
}
