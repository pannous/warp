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
	let printed = run(&format!("play C4 for 0.1s\nmelody [C4 E4] each 0.05s\nround(render_sound(\"{}\") * 10)", path.display()));
	assert!(printed.contains('2'), "{printed}");
	let (rate, seconds) = rendered(&path);
	assert_eq!(rate, SAMPLE_RATE);
	assert!((seconds - 0.2).abs() < 0.001, "{seconds}");
}

#[test]
fn a_render_takes_the_sounds_after_the_last_one() {
	let (first, second) = (wav_path("first"), wav_path("second"));
	run(&format!("play C4 for 0.2s\nrender_sound(\"{}\")\nplay E4 for 0.1s\nrender_sound(\"{}\")", first.display(), second.display()));
	assert!((rendered(&first).1 - 0.2).abs() < 0.001);
	assert!((rendered(&second).1 - 0.1).abs() < 0.001);
}
