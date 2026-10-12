// card real-time (sound layer 8): `play_signal(f, seconds)` plays f(t), t the sample's time in seconds, computed sample
// by sample while it plays, on warp's own output a block ahead of the device; headless the same samples are the run's
// sound, for render_sound
#![cfg(feature = "native")]

const FOLDER: &str = "data/sound";
const WAV_HEADER: usize = 44;
const RATE: f64 = 22_050.0;
const AMPLITUDE: f64 = 32_767.0;

fn rendered_samples(path: &str) -> Vec<i16> {
	std::fs::read(path).expect("rendered wav")[WAV_HEADER..].chunks(2).map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]])).collect()
}

/// The rendered samples of the signal `hum` the definition gives, played for a tenth of a second; in a process of its
/// own, as render_sound takes the sounds of every run in the process (test_sound_render.rs)
fn played(definition: &str, name: &str) -> Vec<i16> {
	std::fs::create_dir_all(FOLDER).expect("data folder");
	let path = format!("{FOLDER}/{name}.wav");
	let program = format!("use sound\n{definition}\nplay_signal(hum, 0.1)\nround(render_sound(\"{path}\") * 100)");
	let run = crate::common::warp_command().args(["--no-ask", "eval", &program]).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert_eq!(printed.trim().trim_start_matches('»').trim(), "10","{}", String::from_utf8_lossy(&run.stderr));
	rendered_samples(&path)
}

fn assert_sine(samples: &[i16], gain: f64, frequency: f64) {
	assert_eq!(samples.len(), (RATE * 0.1) as usize);
	for index in [0, 7, 12, 25, 31, 2000] {
		let expected = gain * AMPLITUDE * (2.0 * std::f64::consts::PI * frequency * index as f64 / RATE).sin();
		assert!((samples[index] as f64 - expected).abs() <= 2.0, "sample {index}: {} instead of {expected:.0}", samples[index]);
	}
}

#[test]
fn a_signal_function_plays_sample_by_sample() {
	assert_sine(&played("hum(t: float) := 0.5 * sin(2 * pi * 441 * t)", "signal_typed"), 0.5, 441.0);
}

#[test]
fn a_signal_parameter_is_the_time_in_seconds_untyped_too() {
	assert_sine(&played("hum(t) := 0.25 * sin(2 * pi * 220.5 * t)", "signal_untyped"), 0.25, 220.5);
}

// DSP keeps state between samples: a one-pole filter gliding to 0.8 (its global a float: exact decimals would grow)
#[test]
fn a_signal_keeps_its_state_between_samples() {
	let samples = played("global level = 0.0 as float\nhum(t) := {\n  level = level + 0.5 * (0.8 - level)\n  level\n}", "signal_state");
	assert_eq!(&samples[..3], [(0.4 * AMPLITUDE).round() as i16, (0.6 * AMPLITUDE).round() as i16, (0.7 * AMPLITUDE).round() as i16]);
	assert_eq!(samples.last(), Some(&((0.8 * AMPLITUDE).round() as i16)));
}

#[test]
fn a_signal_louder_than_full_scale_is_clipped() {
	let samples = played("hum(t: float) := 3.0", "signal_clipped");
	assert!(samples.iter().all(|&sample| sample == i16::MAX), "{:?}", &samples[..4]);
}
