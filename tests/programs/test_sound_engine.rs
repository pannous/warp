// card native-sound: warp's own output mixes every sound into one stream, each from its frame: back to back to the
// sample, `at` placed to the sample, stopped at once, its gain ramped while it plays (fade_sound). The Mixer is the
// clockwork without a device, so these run silent
#![cfg(feature = "native")]
use warp::sound::sound_engine::Mixer;

const RATE: u32 = 100;
const LOUD: i16 = 16384;
const HALF: f32 = 0.5;

fn mixed(mixer: &mut Mixer, frames: usize) -> Vec<f32> {
	let mut buffer = vec![0.0; frames];
	mixer.fill(&mut buffer);
	buffer
}

#[test]
fn sounds_back_to_back_follow_to_the_sample() {
	let mut mixer = Mixer::new(RATE, 1);
	mixer.add(1, 0, &[LOUD; 10], RATE, 1);
	mixer.add(2, 10, &[-LOUD; 10], RATE, 1);
	let buffer = mixed(&mut mixer, 25);
	assert!(buffer[..10].iter().all(|&sample| sample == HALF), "{buffer:?}");
	assert!(buffer[10..20].iter().all(|&sample| sample == -HALF), "{buffer:?}");
	assert!(buffer[20..].iter().all(|&sample| sample == 0.0), "{buffer:?}");
	assert!(!mixer.busy());
}

#[test]
fn a_sound_starts_at_its_frame_across_buffers() {
	let mut mixer = Mixer::new(RATE, 2);
	mixer.add(1, 7, &[LOUD; 4], RATE, 1);
	let first = mixed(&mut mixer, 2 * 8);
	assert!(first[..14].iter().all(|&sample| sample == 0.0) && first[14..] == [HALF, HALF], "{first:?}");
	assert_eq!(mixed(&mut mixer, 2 * 4), [HALF, HALF, HALF, HALF, HALF, HALF, 0.0, 0.0]);
}

#[test]
fn stop_drops_one_sound_or_all() {
	let mut mixer = Mixer::new(RATE, 1);
	mixer.add(1, 0, &[LOUD; 50], RATE, 1);
	mixer.add(2, 0, &[LOUD; 50], RATE, 1);
	mixer.stop(1);
	assert_eq!(mixed(&mut mixer, 1), [HALF]);
	mixer.stop(0);
	assert!(!mixer.busy());
	assert_eq!(mixed(&mut mixer, 1), [0.0]);
}

#[test]
fn a_sound_of_another_rate_keeps_its_length() {
	let mut mixer = Mixer::new(RATE, 1);
	mixer.add(1, 0, &[LOUD; 20], 2 * RATE, 1);
	let buffer = mixed(&mut mixer, 12);
	assert_eq!(buffer.iter().filter(|&&sample| sample != 0.0).count(), 10, "{buffer:?}");
}

#[test]
fn fade_ramps_a_playing_sound_sample_by_sample() {
	let mut mixer = Mixer::new(RATE, 1);
	mixer.add(1, 0, &[LOUD; 30], RATE, 1);
	assert_eq!(mixed(&mut mixer, 2), [HALF, HALF]);
	mixer.fade(1, 0.0, 0.1); // 10 frames
	let faded = mixed(&mut mixer, 12);
	assert!(faded.windows(2).take(10).all(|pair| pair[1] < pair[0]), "{faded:?}");
	assert_eq!(faded[0], HALF);
	assert!(faded[10..].iter().all(|&sample| sample == 0.0), "{faded:?}");
}

#[test]
fn fade_sound_takes_a_handle_of_the_run() {
	let printed = |program: &str| {
		let run = crate::common::warp_command()
			.args(["--no-ask", "eval", program])
			.env("WARP_NO_WINDOW", "1")
			.output()
			.unwrap();
		String::from_utf8_lossy(&run.stdout).trim().trim_start_matches("» ").to_string()
	};
	assert_eq!(printed("h = play(C4, 5ms)\nfade_sound(h, 0, 0.002)\n7"), "7");
	assert!(printed("play(C4, 5ms)\nfade_sound(9, 0)").contains("no sound 9 in this run"));
}

// card real-time: a signal's blocks (one handle, 512 samples at 44.1 kHz each) on a 48 kHz device join without a gap
// or a jump: each block's start frame rounds on its own, a click 86 times a second when the blocks were mixed apart
#[test]
fn a_signals_blocks_join_seamlessly_at_another_rate() {
	const SIGNAL_RATE: u32 = 44_100;
	const DEVICE_RATE: u32 = 48_000;
	const BLOCK: usize = 512;
	const BLOCKS: usize = 8;
	let sine = |index: usize| {
		(f64::from(LOUD) * (2.0 * std::f64::consts::PI * 440.0 * index as f64 / f64::from(SIGNAL_RATE)).sin()) as i16
	};
	let mut mixer = Mixer::new(DEVICE_RATE, 1);
	for block in 0..BLOCKS {
		let start = (block * BLOCK) as f64 * f64::from(DEVICE_RATE) / f64::from(SIGNAL_RATE);
		let samples: Vec<i16> = (block * BLOCK..(block + 1) * BLOCK).map(sine).collect();
		mixer.add(1, start.round() as u64, &samples, SIGNAL_RATE, 1);
	}
	let buffer = mixed(&mut mixer, (BLOCKS - 1) * BLOCK);
	let largest_step = buffer.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0, f32::max);
	let smooth_step = 2.0 * std::f32::consts::PI * 440.0 / DEVICE_RATE as f32 * HALF;
	assert!(
		largest_step <= 1.1 * smooth_step,
		"a jump of {largest_step} between samples, a smooth sine steps at most {smooth_step}"
	);
}

// card sound-voice-stealing: at most MAX_VOICES sounds sound at once; one more steals the oldest, which fades out in
// STEAL_SECONDS (no click) and leaves the mix
#[test]
fn one_voice_over_the_cap_steals_the_oldest() {
	use warp::sound::sound_engine::MAX_VOICES;
	const QUIET: i16 = 256;
	let mut mixer = Mixer::new(RATE, 1);
	for handle in 1..=MAX_VOICES + 1 {
		mixer.add(handle, 0, &[QUIET; 300], RATE, 1);
	}
	let buffer = mixed(&mut mixer, 200);
	let each = QUIET as f32 / 32_768.0;
	assert!(
		(buffer[199] - MAX_VOICES as f32 * each).abs() < 1e-4,
		"{} voices sound, not {}",
		buffer[199] / each,
		MAX_VOICES
	);
	mixer.stop(1);
	assert!((mixed(&mut mixer, 1)[0] - MAX_VOICES as f32 * each).abs() < 1e-4, "the stolen one is gone already");
}
