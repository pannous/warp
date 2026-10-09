// card sound-library: `play "song.mp3"` plays a music file (WAV, MP3, OGG, FLAC, AIFF, M4A) in the background, by
// the system's player natively (afplay, paplay, ffplay, mpv) and an <audio> in the playground. Headless (tests, CI,
// WARP_NO_WINDOW) nothing plays: the file is checked and a `sound file <format>: <path>` line goes to stderr
#![cfg(feature = "native")]
use crate::common::fails_with;
use crate::is;
use std::path::PathBuf;

/// A file of `bytes` in a temporary folder of the test's own
fn sound_file(folder: &str, name: &str, bytes: &[u8]) -> PathBuf {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(folder);
	std::fs::create_dir_all(&folder).unwrap();
	let path = folder.join(name);
	std::fs::write(&path, bytes).unwrap();
	path
}

/// What `program` prints on stderr, its stdout holding 7
fn errors_of(program: &str) -> String {
	let run = crate::common::warp_command().args(["--no-ask", "eval", program]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr).into_owned();
	assert!(String::from_utf8_lossy(&run.stdout).contains('7'), "{errors}");
	errors
}

#[test]
fn a_music_file_plays_in_the_background() {
	let wav = sound_file("sound-files", "beep.wav", &warp::sound::wav(&[32768, 40000, 32768], 22050));
	assert!(errors_of(&format!("play \"{}\"\n7", wav.display())).contains(&format!("sound file wav: {}", wav.display())));
	let mp3 = sound_file("sound-files", "song.mp3", b"ID3\x04\x00\x00\x00\x00\x00\x00");
	assert!(errors_of(&format!("play_file(\"{}\")\nstop_sound\n7", mp3.display())).contains("sound file mp3: "));
	let ogg = sound_file("sound-files", "song.ogg", b"OggS\x00\x02\x00\x00");
	assert!(errors_of(&format!("play \"{}\"\n7", ogg.display())).contains("sound file ogg: "));
}

#[test]
fn only_a_music_file_plays() {
	let text = sound_file("sound-files-not", "notes.txt", b"C4 E4 G4");
	fails_with(&format!("play \"{}\"", text.display()), "not a sound file");
	fails_with("play \"no/such/song.mp3\"", "no/such/song.mp3");
	is!("play [C4 E4] for 3ms\nstop_sound\n7", 7);
}
