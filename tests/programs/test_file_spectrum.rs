// card gpu-visualizer-mp3: spectrum("song.mp3", seconds, bands) measures an audio file at a moment (decoded once by the
// host, src/sound/spectrum.rs), sound_seconds(file) is how long it sounds; samples/gpu_visualizer_warp.warp lights its
// bars with it while the file plays. shader_of(f) is the WGSL paint(f, …) compiles f to
#![cfg(feature = "native")]
use crate::is;

/// A program that renders an A4 of 0.2 s to a WAV, then runs `then` on it, the file's path as `file`
fn of_a_rendered_tone(then: &str) -> String {
	let folder = crate::common::scratch_directory("file_spectrum");
	std::fs::create_dir_all(&folder).unwrap();
	let file = folder.join("a4.wav");
	format!("use sound\nfile = \"{}\"\ntone(A4, 0.2)\nrender_sound(file)\n{then}", file.display())
}

#[test]
fn a_files_tone_fills_its_own_band() {
	is!(&of_a_rendered_tone("bars = spectrum(file, 0.05, 8)\nbars[nearest_band(A4, 8)] == max(bars)"), true);
	is!(&of_a_rendered_tone("count(spectrum(file, 0.05, 12))"), 12);
	is!(&of_a_rendered_tone("max(spectrum(file, 5, 4)) == 0"), true);
}

#[test]
fn a_file_says_how_long_it_sounds() {
	is!(&of_a_rendered_tone("round(sound_seconds(file) * 10)"), 2);
}

#[test]
fn shader_of_is_the_wgsl_a_function_paints_with() {
	let printed = crate::common::printed(
		"width = 8\nheight = 8\nf(x, y) := { s = 0; for i in 1 to 3 { if x > i { s = s + 1 } }; [s / 3, y / height, 0.5] }\nprint shader_of(f)",
	);
	assert!(
		printed.contains("@fragment fn main") && printed.contains("fn warp_f_color(a_x: f32, a_y: f32)"),
		"{printed}"
	);
	// the statements indented by their braces
	assert!(
		printed.contains("\n\tfor (") && printed.contains("\n\t\tif (") && printed.contains("\n\t\t\tv_s = "),
		"{printed}"
	);
	crate::common::fails_with("g(x, y) := \"a\"; shader_of(g)", "WGSL cannot compute g");
}

#[test]
fn the_compiled_shader_demo_compiles_to_one_fragment_shader() {
	let demo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/warp_compiled_wgsl_shader.warp"));
	let lowered = warp::pipeline::lower(&demo.unwrap()).expect("a program").serialize();
	assert!(lowered.contains("@fragment") && lowered.contains("fn warp_swirl_color("), "{lowered}");
}

#[test]
fn a_file_written_again_is_measured_again() {
	// render_sound writes the sounds since the last one: the file holds the A4 of 0.2 s, then only a C3 of 0.4 s
	let again = "before = sound_seconds(file)\ntone(C3, 0.4)\nrender_sound(file)\nbars = spectrum(file, 0.05, 8)";
	is!(
		&of_a_rendered_tone(&format!("{again}\n[round(before * 10), round(sound_seconds(file) * 10)] == [2, 4]")),
		true
	);
	is!(&of_a_rendered_tone(&format!("{again}\nbars[nearest_band(C3, 8)] == max(bars) and max(bars) > 0")), true);
}
