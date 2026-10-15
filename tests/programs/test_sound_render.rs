// card sound-pro, step 2: render_sound(path) writes the sounds played since the program's start (or the last render)
// one after another into one WAV file and gives its seconds: offline, so it works headless, without speakers
#![cfg(feature = "native")]

const WAV_HEADER: usize = 44;
const SAMPLE_RATE: u32 = 22_050;

/// The file's sample rate and its seconds of 16-bit mono samples
fn rendered(path: &std::path::Path) -> (u32, f64) {
	let bytes = std::fs::read(path).unwrap();
	assert_eq!(&bytes[0..4], b"RIFF");
	assert_eq!(&bytes[8..12], b"WAVE");
	let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
	(rate, (bytes.len() - WAV_HEADER) as f64 / 2.0 / rate as f64)
}

fn run(program: &str) -> String {
	let run = crate::common::warp_command().args(["--no-ask", "eval", program]).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout).into_owned();
	assert!(run.status.success(), "{printed} {}", String::from_utf8_lossy(&run.stderr));
	printed
}

fn wav_path(name: &str) -> std::path::PathBuf {
	std::env::temp_dir().join(format!("warp-render-{name}-{}.wav", std::process::id()))
}

#[test]
fn sounds_render_into_one_wav_file() {
	let path = wav_path("song");
	let printed = run(&format!(
		"use sound\nplay(C4, 0.1s)\nmelody([C4 E4], 0.05s)\nround(save(sound, \"{}\") * 10)",
		path.display()
	));
	assert!(printed.contains('2'), "{printed}");
	let (rate, seconds) = rendered(&path);
	assert_eq!(rate, SAMPLE_RATE);
	assert!((seconds - 0.2).abs() < 0.001, "{seconds}");
}

#[test]
fn a_render_takes_the_sounds_after_the_last_one() {
	let (first, second) = (wav_path("first"), wav_path("second"));
	run(&format!(
		"use sound\nplay(C4, 0.2s)\nsave sound as: \"{}\"\nplay(E4, 0.1s)\nsave sound as: \"{}\"",
		first.display(),
		second.display()
	));
	assert!((rendered(&first).1 - 0.2).abs() < 0.001);
	assert!((rendered(&second).1 - 0.1).abs() < 0.001);
}

// card save-sound: writing audio is the general save, overloaded by the value's type (lib/sound.warp Recording,
// lib/file.warp text); render_sound is the old name, with a note naming save
#[test]
fn save_sound_as_a_file_writes_the_sounds() {
	let (spaced, called) = (wav_path("saved"), wav_path("save-called"));
	let printed = run(&format!(
		"use sound\nplay(C4, 0.2s)\nsave sound as: \"{}\"\nplay(E4, 0.1s)\nseconds = save(sound, \"{}\")\nround(seconds * 10)",
		spaced.display(),
		called.display()
	));
	assert!(printed.contains('1'), "{printed}");
	assert!((rendered(&spaced).1 - 0.2).abs() < 0.001);
	assert!((rendered(&called).1 - 0.1).abs() < 0.001);
}

#[test]
fn save_text_as_a_file_writes_it() {
	let path = std::env::temp_dir().join(format!("warp-save-text-{}.txt", std::process::id()));
	run(&format!("use file\nsave \"hi\" as: \"{}\"", path.display()));
	assert_eq!(std::fs::read_to_string(&path).unwrap(), "hi");
}

#[test]
fn render_sound_is_save_with_a_note() {
	let path = wav_path("old-name");
	let run = crate::common::warp_command()
		.args(["--no-ask", "eval", &format!("use sound\nplay(C4, 0.1s)\nrender_sound(\"{}\")", path.display())])
		.output()
		.unwrap();
	assert!(String::from_utf8_lossy(&run.stderr).contains("save sound as: file"));
	assert!((rendered(&path).1 - 0.1).abs() < 0.001);
}

/// card spectrum-test: save sound mixes the sounds of its own run only, as the test threads run many programs at once
/// in one process (a 0.2 s tone saved 0.3 s long, the sound of another test in it)
#[test]
fn programs_running_at_once_save_their_own_sounds() {
	let runs: Vec<_> = (1..=6)
		.map(|tenths| {
			std::thread::spawn(move || {
				let path = wav_path(&format!("at-once-{tenths}"));
				let program = format!(
					"use sound\ntone(A4, {})\nseconds = save sound as: \"{}\"\nround(seconds * 10)",
					tenths as f64 / 10.0,
					path.display()
				);
				(tenths, warp::wasm_emitter::eval(&program).serialize())
			})
		})
		.collect();
	for run in runs {
		let (tenths, saved) = run.join().unwrap();
		assert_eq!(saved.trim(), tenths.to_string(), "the run of a {tenths}/10 s tone");
	}
}
