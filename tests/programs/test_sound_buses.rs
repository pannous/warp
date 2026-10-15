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
	let program = format!("use sound\n{program}\nrender_sound \"{}\"", path.display());
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
