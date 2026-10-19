// card real-time (sound layer 8): `play_signal(f, seconds)` plays f(t), t the sample's time in seconds, computed sample
// by sample while it plays, on warp's own output a block ahead of the device; headless the same samples are the run's
// sound, for render_sound
#![cfg(feature = "native")]

const FOLDER: &str = "data/sound";
const WAV_HEADER: usize = 44;
const RATE: f64 = 22_050.0;
const AMPLITUDE: f64 = 32_767.0;

fn rendered_samples(path: &str) -> Vec<i16> {
	std::fs::read(path).expect("rendered wav")[WAV_HEADER..]
		.chunks(2)
		.map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
		.collect()
}

/// The rendered samples of the signal `hum` the definition gives, played for a tenth of a second; in a process of its
/// own, as render_sound takes the sounds of every run in the process (test_sound_render.rs)
fn played(definition: &str, name: &str) -> Vec<i16> {
	rendered(&format!("{definition}\nplay_signal(hum, 0.1)"), name, "rendered", "10")
}

/// The rendered samples of the program's sound, after which `last` (`rendered`: the rendered length in hundredths of a
/// second) is `expected`
fn rendered(program: &str, name: &str, last: &str, expected: &str) -> Vec<i16> {
	std::fs::create_dir_all(FOLDER).expect("data folder");
	let path = format!("{FOLDER}/{name}.wav");
	let program = format!("use sound\n{program}\nrendered = round(save(sound, \"{path}\") * 100)\n{last}");
	let run = crate::common::warp_command().args(["--no-ask", "eval", &program]).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert_eq!(printed.trim().trim_start_matches('»').trim(), expected, "{}", String::from_utf8_lossy(&run.stderr));
	rendered_samples(&path)
}

fn assert_sine(samples: &[i16], gain: f64, frequency: f64) {
	assert_eq!(samples.len(), (RATE * 0.1) as usize);
	for index in [0, 7, 12, 25, 31, 2000] {
		let expected = gain * AMPLITUDE * (2.0 * std::f64::consts::PI * frequency * index as f64 / RATE).sin();
		assert!(
			(samples[index] as f64 - expected).abs() <= 2.0,
			"sample {index}: {} instead of {expected:.0}",
			samples[index]
		);
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
	let samples = played(
		"global level = 0.0 as float\nhum(t) := {\n  level = level + 0.5 * (0.8 - level)\n  level\n}",
		"signal_state",
	);
	assert_eq!(
		&samples[..3],
		[(0.4 * AMPLITUDE).round() as i16, (0.6 * AMPLITUDE).round() as i16, (0.7 * AMPLITUDE).round() as i16]
	);
	assert_eq!(samples.last(), Some(&((0.8 * AMPLITUDE).round() as i16)));
}

#[test]
fn a_signal_louder_than_full_scale_is_clipped() {
	let samples = played("hum(t: float) := 3.0", "signal_clipped");
	assert!(samples.iter().all(|&sample| sample == i16::MAX), "{:?}", &samples[..4]);
}

// block-wise (sound layer 8): `play_blocks(fill, linear_address(block), seconds)` calls fill with the block's time; fill
// writes every cell of the program's linear float array, one call per block instead of one per sample (decision
// music-plain-library: the program owns its block)
const BLOCK_SINE: &str = "linear hum_block = float[512]\nglobal calls = 0\nhum(t) := {\n  calls = calls + 1\n  for i in 1 to #hum_block { hum_block#i = 0.5 * sin(2 * pi * 441 * (t + (i - 1) / sample_rate)) }\n  0\n}\nplay_blocks(hum, linear_address(hum_block), 0.1)";

#[test]
fn a_block_function_fills_the_samples_of_each_block() {
	assert_sine(&rendered(BLOCK_SINE, "signal_blocks", "rendered", "10"), 0.5, 441.0);
}

#[test]
fn a_block_function_is_called_once_a_block() {
	let samples = rendered(BLOCK_SINE, "signal_block_calls", "calls", "5");
	assert_eq!(samples.len(), (RATE * 0.1) as usize, "4 blocks of 512 samples and 157 of the fifth");
}

// stereo (sound layer 8): two functions of the time, left and right, or a block function filling two blocks
fn channel(samples: &[i16], index: usize) -> Vec<i16> {
	samples.iter().skip(index).step_by(2).copied().collect()
}

fn assert_stereo(samples: &[i16]) {
	assert_sine(&channel(samples, 0), 0.5, 441.0);
	assert_sine(&channel(samples, 1), 0.25, 220.5);
}

#[test]
fn a_pair_of_signal_functions_plays_in_stereo() {
	let program = "left(t) := 0.5 * sin(2 * pi * 441 * t)\nright(t) := 0.25 * sin(2 * pi * 220.5 * t)\nplay_signal([left, right], 0.1)";
	assert_stereo(&rendered(program, "signal_stereo", "rendered", "10"));
}

#[test]
fn a_block_function_of_two_blocks_plays_in_stereo() {
	let program = "linear left = float[512]\nlinear right = float[512]\nhum(t) := for i in 1 to #left {\n  time = t + (i - 1) / sample_rate\n  left#i = 0.5 * sin(2 * pi * 441 * time)\n  right#i = 0.25 * sin(2 * pi * 220.5 * time)\n}\nplay_blocks(hum, [linear_address(left), linear_address(right)], 0.1)";
	assert_stereo(&rendered(program, "signal_blocks_stereo", "rendered", "10"));
}

// `play(hum, 2s)` (user, 2026-10-12: no snake case for signals): play of a function of the time is play_signal, of a
// pair of functions stereo; play_blocks takes seconds too
#[test]
fn play_of_a_function_plays_it_as_a_signal() {
	assert_sine(
		&rendered("hum(t) := 0.5 * sin(2 * pi * 441 * t)\nplay(hum, 0.1s)", "play_function", "rendered", "10"),
		0.5,
		441.0,
	);
	assert_sine(&rendered(&BLOCK_SINE.replace("0.1)", "0.1s)"), "play_block_function", "calls", "5"), 0.5, 441.0);
	let pair =
		"left(t) := 0.5 * sin(2 * pi * 441 * t)\nright(t) := 0.25 * sin(2 * pi * 220.5 * t)\nplay([left, right], 0.1s)";
	assert_stereo(&rendered(pair, "play_function_pair", "rendered", "10"));
}
