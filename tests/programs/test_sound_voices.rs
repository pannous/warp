// card sound-pro, layer 1: voices overlap. A task (`go { … }`) is a voice of its own: its sounds start where the
// voice that started it stands on the audio clock and follow each other, while the program's own sounds go on, so
// the two sound together; render_sound mixes them as they sound
#![cfg(feature = "native")]

const WAV_HEADER: usize = 44;

/// The samples of the rendered WAV
fn rendered_samples(path: &std::path::Path) -> Vec<i16> {
	let bytes = std::fs::read(path).unwrap();
	bytes[WAV_HEADER..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect()
}

/// The program's printed value; it renders its sounds into the file named `name`
fn rendered(name: &str, program: &str) -> (String, Vec<i16>) {
	let path = std::env::temp_dir().join(format!("warp-voices-{name}-{}.wav", std::process::id()));
	let program = program.replace("SONG", &path.display().to_string());
	let run = crate::common::warp_command().args(["--no-ask", "eval", &program]).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout).trim().trim_start_matches("» ").to_string();
	assert!(run.status.success(), "{printed} {}", String::from_utf8_lossy(&run.stderr));
	(printed, rendered_samples(&path))
}

#[test]
fn a_task_is_a_voice_that_sounds_along() {
	let (seconds, samples) = rendered("along", "job = go { melody [C4 E4] each 0.1s }\nmelody [G4 C5] each 0.1s\nawait job\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "0.2");
	assert_eq!(samples.len(), 4410);
}

#[test]
fn a_voice_starts_where_its_starter_stands() {
	let program = "play C4 for 0.1s\njob = go { play E4 for 0.1s }\nplay G4 for 0.1s\nawait job\nrender_sound(\"SONG\")";
	let (seconds, samples) = rendered("starts", program);
	assert_eq!(seconds, "0.2");
	assert!(samples[..2205].iter().any(|&sample| sample > 3000), "the first note sounds first");
}

#[test]
fn voices_add_up() {
	let (_, alone) = rendered("alone", "play 440Hz for 0.1s\nrender_sound(\"SONG\")");
	let (_, both) = rendered("both", "job = go { play 440Hz for 0.1s }\nplay 440Hz for 0.1s\nawait job\nrender_sound(\"SONG\")");
	assert_eq!(alone.len(), both.len());
	let loudest = |samples: &[i16]| samples.iter().map(|sample| u32::from(sample.unsigned_abs())).max().unwrap();
	assert!(loudest(&both) > loudest(&alone) * 19 / 10, "{} {}", loudest(&both), loudest(&alone));
}
