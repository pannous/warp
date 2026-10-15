// card sound-gain-ramps: fade_sound(handle, gain, seconds) ramps a sound's gain from where the speaker stands when it
// is called; render_sound writes the ramp too, so a fade is heard the same live and offline (headless here)
#![cfg(feature = "native")]

const WAV_HEADER: usize = 44;
const SAMPLE_RATE: usize = 22_050;
const QUIET: i16 = 50;

/// The rendered mono samples of the program, which ends in render_sound to the path
fn rendered_samples(program: &str, name: &str) -> Vec<i16> {
	let path = std::env::temp_dir().join(format!("warp-fades-{name}-{}.wav", std::process::id()));
	let program = format!("use sound\n{program}\nsave sound as: \"{}\"", path.display());
	let run = crate::common::warp_command().args(["--no-ask", "eval", &program]).output().unwrap();
	assert!(run.status.success(), "{} {}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	let bytes = std::fs::read(&path).unwrap();
	bytes[WAV_HEADER..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect()
}

/// The loudest sample between the seconds
fn loudest(samples: &[i16], from: f64, to: f64) -> i16 {
	samples[(from * SAMPLE_RATE as f64) as usize..(to * SAMPLE_RATE as f64) as usize]
		.iter()
		.map(|sample| sample.saturating_abs())
		.max()
		.unwrap_or(0)
}

#[test]
fn fade_sound_ramps_a_sound_down_in_the_render() {
	let samples = rendered_samples("h = tone(440Hz, 1s)\nfade_sound(h, 0, 0.4)", "out");
	assert!(loudest(&samples, 0.02, 0.1) > 3 * loudest(&samples, 0.32, 0.38), "the ramp goes down");
	assert!(loudest(&samples, 0.5, 0.95) < QUIET, "silent after the ramp");
}

/// Headless nothing plays, so the speaker stays at the start: however long the program takes, its fade begins there
#[test]
fn a_headless_fade_waits_for_no_clock() {
	let samples = rendered_samples("a = tone(440Hz, 1s)\nsleep 500ms\nfade_sound(a, 0, 0.1)", "slow");
	assert!(loudest(&samples, 0.3, 0.9) < QUIET, "a faded out from the start");
}

#[test]
fn fade_sound_of_one_handle_leaves_the_others() {
	let samples = rendered_samples("a = tone(440Hz, 1s)\nb = tone(440Hz, 1s)\nfade_sound(a, 0, 0.1)", "one");
	assert!(loudest(&samples, 0.3, 0.9) < QUIET, "a faded out");
	assert!(loudest(&samples, 1.1, 1.9) > 1000, "b plays on");
	let samples = rendered_samples("a = tone(440Hz, 1s)\nb = tone(440Hz, 1s)\nfade_sound(0, 0, 0.1)", "all");
	assert!(loudest(&samples, 0.3, 1.9) < QUIET, "handle 0 fades all");
}
