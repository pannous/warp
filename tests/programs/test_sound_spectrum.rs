// card winamp-like: spectrum(samples, seconds, bands) gives how loud each frequency band sounds in the samples at that
// moment, from 0 to 1 (bands spaced evenly in pitch from band_low to band_high Hz), computed in warp, so the same
// natively and in the playground; samples/sound/visualizer.warp paints it as bars while the song plays
use crate::is;

const SOUND: &str = "use sound\n";

/// What a child warp run tells about itself: how it ended (an exit code, or a signal under load), its stdout and stderr
#[cfg(feature = "native")]
fn run_report(run: &std::process::Output) -> String {
	format!(
		"the run ended with status {}\nstdout:\n{}\nstderr:\n{}",
		run.status,
		String::from_utf8_lossy(&run.stdout),
		String::from_utf8_lossy(&run.stderr)
	)
}

#[test]
fn a_tone_fills_its_own_band() {
	is!(&format!("{SOUND}bars = spectrum(chord_samples([A4], 0.1, \"sine\"), 0.02, 8)\nbars[nearest_band(A4, 8)] == max(bars)"), true);
	is!(&format!("{SOUND}bars = spectrum(chord_samples([C3], 0.1, \"sine\"), 0.02, 8)\nbars[nearest_band(C3, 8)] == max(bars)"), true);
	is!(&format!("{SOUND}count(spectrum(chord_samples([A4], 0.1, \"sine\"), 0.02, 12))"), 12);
}

#[test]
fn silence_and_the_end_are_empty() {
	is!(&format!("{SOUND}max(spectrum([sample_offset for i in 0..2000], 0, 4)) == 0"), true);
	is!(&format!("{SOUND}max(spectrum(chord_samples([A4], 0.1, \"sine\"), 5, 4)) == 0"), true);
}

/// A visualizer sample's run, with the arguments of its <name>.args: it plays its song and paints frame after frame;
/// None without a WebGPU adapter
#[cfg(feature = "native")]
fn animates_while_it_plays(sample: &str) -> Option<()> {
	let arguments = crate::common::arguments_beside(&std::path::Path::new(sample).with_extension(""));
	let run = crate::common::warp_command().args(["--no-ask", "run", sample]).args(arguments).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	if errors.contains("no WebGPU adapter") {
		return None;
	}
	assert!(run.status.success(), "{}", run_report(&run));
	assert!(errors.contains("sound "), "{}", run_report(&run));
	// the loop paints a frame per 40 ms of a 4 s song by the wall clock: a second frame shows it animates, however slow
	// a frame gets under load (3 frames once failed in a full gate: card visualizer-test)
	let painted = |output: &[u8]| String::from_utf8_lossy(output).matches("painted").count();
	assert!(painted(&run.stderr).max(painted(&run.stdout)) >= 2, "{}", run_report(&run));
	Some(())
}

#[cfg(feature = "native")]
#[test]
fn a_nonzero_visualizer_child_report_names_its_exit_status() {
	let run = crate::common::warp_command().args(["--no-ask", "eval", "raise \"visualizer probe\""]).output().unwrap();
	assert!(!run.status.success());
	assert!(run_report(&run).contains(&format!("status {}", run.status)));
}

#[cfg(feature = "native")]
#[test]
fn the_visualizer_paints_while_it_plays() {
	animates_while_it_plays(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/sound/visualizer.warp"))
		.expect("the visualizer needs no GPU");
}

// the WebGPU visualizer: a raymarching fragment shader reads the spectrum through $levels each frame
#[cfg(feature = "native")]
#[test]
fn the_gpu_visualizer_raymarches_while_it_plays() {
	if animates_while_it_plays(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/gpu/gpu_visualizer.warp")).is_none() {
		crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
}

// the same tunnel written in warp functions: paint(tunnel, …) compiles them to one fragment shader (card compile-shaders),
// lit by mozart.mp3's spectrum; its first 4 seconds (gpu_visualizer_warp.args), as long as gpu_visualizer.warp's song, not the whole song
#[cfg(feature = "native")]
#[test]
fn the_gpu_visualizer_written_in_warp_raymarches_while_it_plays() {
	if animates_while_it_plays(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/gpu/gpu_visualizer_warp.warp")).is_none() {
		crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
}
