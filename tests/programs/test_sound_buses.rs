// card shared-buses: a bus is one effect chain that many sounds go through together, from the program and its tasks,
// processed once on their mix (src/sound/effects.rs): live in the Mixer, the same in render_sound (headless here)
#![cfg(feature = "native")]
use warp::sound::effects::Effect;
use warp::sound::sound_engine::Mixer;

const WAV_HEADER: usize = 44;
const SAMPLE_RATE: usize = 22_050;
const QUIET: i16 = 50;

/// The rendered mono samples of the program, which ends in render_sound to the path
fn rendered_samples(program: &str, name: &str) -> Vec<i16> {
	let path = std::env::temp_dir().join(format!("warp-buses-{name}-{}.wav", std::process::id()));
	let program = format!("use sound\n{program}\nsave sound as \"{}\"", path.display());
	let run = crate::common::warp_command().args(["--no-ask", "eval", &program]).output().unwrap();
	assert!(run.status.success(), "{} {}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	let bytes = std::fs::read(&path).unwrap();
	bytes[WAV_HEADER..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect()
}

/// The loudest sample between the seconds (silence past the end)
fn loudest(samples: &[i16], from: f64, to: f64) -> i16 {
	let at = |seconds: f64| ((seconds * SAMPLE_RATE as f64) as usize).min(samples.len());
	samples[at(from)..at(to)].iter().map(|sample| sample.saturating_abs()).max().unwrap_or(0)
}

#[test]
fn a_sound_sent_to_a_delay_bus_echoes_where_it_was_silent() {
	let samples = rendered_samples("room = bus() |> delay(0.3, 0.5, 1)\nout(osc(A4, 0.1s), room)", "delay");
	assert!(loudest(&samples, 0.0, 0.1) > 1000, "the dry sound");
	assert!(loudest(&samples, 0.15, 0.28) < QUIET, "silent between");
	assert!(loudest(&samples, 0.31, 0.39) > 500, "its echo 0.3 s later");
}

#[test]
fn a_sound_outside_the_bus_stays_dry() {
	let samples = rendered_samples("room = bus() |> delay(0.3, 0.5, 1)\nout(osc(A4, 0.1s))", "dry");
	assert!(loudest(&samples, 0.15, 0.5) < QUIET, "no echo: {}", loudest(&samples, 0.15, 0.5));
}

/// a task's sound and the program's go through the same bus: both echo
#[test]
fn a_task_and_the_program_share_a_bus() {
	let program = "room = bus() |> delay(0.3, 0.5, 1)\njob = go { out(osc(E5, 0.1s), room); 1 }\nawait job\nout(osc(A4, 0.1s), room)";
	let samples = rendered_samples(program, "shared");
	assert!(loudest(&samples, 0.31, 0.39) > 500, "the echo of the sounds 0.3 s later");
}

#[test]
fn a_reverb_bus_rings_after_its_last_sound() {
	let samples = rendered_samples("hall = bus() |> reverb(0.5)\nout(osc(A4, 0.1s), hall)", "reverb");
	assert!(loudest(&samples, 0.12, 0.3) > QUIET, "the room's tail after the sound");
}

/// The Mixer itself, as the device's callback drives it: a bus with a delay echoes the sound mixed into it
#[test]
fn the_mixer_runs_a_bus_on_its_sounds() {
	let rate = 1000;
	let mut mixer = Mixer::new(rate, 1);
	// each echo at half the level before (lib/sound.warp echoed: y[i] = x[i] + feedback · y[i - gap])
	mixer.set_bus(1, vec![Effect::Delay { seconds: 0.1, feedback: 0.5, mix: 1.0 }]);
	mixer.add_to_bus(7, 1, 0, &[16_384; 10], rate, 1);
	let mut buffer = vec![0.0; 300];
	mixer.fill(&mut buffer);
	let near = |at: usize, level: f32| (buffer[at] - level).abs() < 0.01;
	assert!(near(5, 0.5), "the dry sound: {}", buffer[5]);
	assert!(near(50, 0.0), "silent between: {}", buffer[50]);
	assert!(near(105, 0.25), "the echo 0.1 s later: {}", buffer[105]);
	assert!(near(205, 0.125), "and the next: {}", buffer[205]);
}

// card shared-buses stage 3: a bus's effect applied again takes its new settings at the voice's moment, keeping its
// place in the chain and what still rings in it

#[test]
fn a_changed_effect_applies_from_the_voices_moment() {
	let program = "room = bus() |> gain(1)\nout(osc(A4, 0.2s), room)\nroom = room |> gain(0)\nout(osc(A4, 0.2s), room)";
	let samples = rendered_samples(program, "change");
	assert!(loudest(&samples, 0.05, 0.15) > 1000, "the first sound at gain 1");
	assert!(loudest(&samples, 0.25, 0.4) < QUIET, "the second at gain 0: {}", loudest(&samples, 0.25, 0.4));
}

#[test]
fn an_effect_set_again_keeps_its_echoes() {
	let program = "room = bus() |> delay(0.3, 0.5, 1)\nout(osc(A4, 0.1s), room)\nroom = room |> delay(0.3, 0.5, 1)";
	let samples = rendered_samples(program, "kept");
	assert!(loudest(&samples, 0.31, 0.39) > 500, "the echo rings on through the change");
}

#[test]
fn an_effect_set_again_keeps_its_place_in_the_chain() {
	crate::is!(
		"use sound\nroom = bus() |> gain(1) |> reverb(0.3)\nroom = room |> gain(0.5)\nroom.chain",
		warp::warp_parser::parse("[[\"gain\" 0.5] [\"reverb\" 0.3]]")
	);
}

/// out restores the bus its thread sent to before, so out(…, room) inside another bus's sound leaves that bus on
#[test]
fn out_restores_the_bus_before() {
	crate::is!(
		"use sound\nroom = bus()\nstd_io(\"sound\", \"send\", [7])\nout([0.0], room)\nstd_io(\"sound\", \"send\", [0])",
		7
	);
}

#[test]
fn the_mixer_changes_a_bus_at_its_frame() {
	let rate = 1000;
	let mut mixer = Mixer::new(rate, 1);
	mixer.set_bus(1, vec![Effect::Gain(1.0)]);
	mixer.add_to_bus(7, 1, 0, &[16_384; 10], rate, 1);
	mixer.change_bus(1, 5, vec![Effect::Gain(0.0)]);
	let mut buffer = vec![0.0; 20];
	mixer.fill(&mut buffer);
	assert!((buffer[3] - 0.5).abs() < 0.01, "before the change: {}", buffer[3]);
	assert!(buffer[7].abs() < 0.01, "after it: {}", buffer[7]);
}

// card shared-buses stage 4: a compressor and a peaking and shelving EQ as bus effects

/// The loudest sample of the processed mono sine at the frequency (Hz), after its first tenth of a second
fn processed_sine_peak(frequency: f32, effects: &[Effect]) -> f32 {
	let rate = 22_050;
	let sine: Vec<f32> =
		(0..rate).map(|i| 0.1 * (std::f32::consts::TAU * frequency * i as f32 / rate as f32).sin()).collect();
	let output = warp::sound::effects::processed(&sine, effects, &[], rate as u32, 1);
	output[rate / 10..rate].iter().fold(0.0, |loudest, sample| sample.abs().max(loudest))
}

#[test]
fn a_compressor_turns_loud_sounds_down_and_leaves_quiet_ones() {
	let compressor = [Effect::named("compress", &[-20.0, 4.0, 0.001, 0.1]).unwrap()];
	let rate = 1000;
	let loud = warp::sound::effects::processed(&[0.9; 1000], &compressor, &[], rate, 1);
	assert!((0.12..0.25).contains(&loud[900]), "0.9 (-1 dB) is 14 dB down: {}", loud[900]);
	let quiet = warp::sound::effects::processed(&[0.05; 1000], &compressor, &[], rate, 1);
	assert!((quiet[900] - 0.05).abs() < 0.001, "-26 dB stays under the threshold: {}", quiet[900]);
}

#[test]
fn a_peak_eq_boosts_its_frequency_only() {
	let peak = [Effect::named("peak", &[1000.0, 12.0, 1.0]).unwrap()];
	assert!((0.35..0.45).contains(&processed_sine_peak(1000.0, &peak)), "12 dB up: ×4");
	assert!((0.09..0.12).contains(&processed_sine_peak(50.0, &peak)), "far below it stays");
}

#[test]
fn shelves_boost_below_or_above_their_frequency() {
	let low = [Effect::named("low_shelf", &[200.0, 12.0, 0.707]).unwrap()];
	assert!(
		(0.35..0.45).contains(&processed_sine_peak(30.0, &low)),
		"the lows up: {}",
		processed_sine_peak(30.0, &low)
	);
	assert!((0.09..0.12).contains(&processed_sine_peak(5000.0, &low)), "the highs stay");
	let high = [Effect::named("high_shelf", &[2000.0, -12.0, 0.707]).unwrap()];
	assert!(processed_sine_peak(9000.0, &high) < 0.03, "the highs down: {}", processed_sine_peak(9000.0, &high));
	assert!((0.09..0.12).contains(&processed_sine_peak(100.0, &high)), "the lows stay");
}

#[test]
fn a_bus_takes_a_compressor_and_an_eq() {
	crate::is!(
		"use sound\nroom = bus() |> compress(-30, 8) |> low_shelf(200Hz, 6) |> peak(3kHz, -4, 2)\nroom.chain",
		warp::warp_parser::parse("[[\"compress\" -30 8 0.01 0.1] [\"low_shelf\" 200 6 0.707] [\"peak\" 3000 -4 2]]")
	);
}

/// EQ bands of one kind at different frequencies are several bands; one at the same frequency is set again (card bus-peak)
#[test]
fn eq_bands_at_other_frequencies_add_up() {
	crate::is!(
		"use sound\nroom = bus() |> peak(200Hz, 3) |> peak(3kHz, -2)\nroom = room |> peak(200Hz, 5)\nroom.chain",
		warp::warp_parser::parse("[[\"peak\" 200 5 1] [\"peak\" 3000 -2 1]]")
	);
}

/// a compressor follows the louder channel on both, so a loud left side keeps the stereo image (review of stage 4)
#[test]
fn a_compressor_ducks_both_channels_together() {
	let compressor = [Effect::named("compress", &[-20.0, 4.0, 0.001, 0.1]).unwrap()];
	let frames: Vec<f32> = (0..1000).flat_map(|_| [0.9, 0.05]).collect();
	let output = warp::sound::effects::processed(&frames, &compressor, &[], 1000, 2);
	let (left, right) = (output[1800], output[1801]);
	assert!((right / left - 0.05 / 0.9).abs() < 0.001, "the same gain on both: {left} {right}");
}
