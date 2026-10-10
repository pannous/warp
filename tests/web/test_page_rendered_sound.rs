// card playground-refuses: in the browser render_sound writes its WAV into the page's files (host-files.js
// writtenFiles), where read, play and the playground's download link take it; natively tests/programs/test_sound_render.rs
// (in-process runs share the native host's unrendered sounds, so this runs only in the browser suite)
#![cfg(not(feature = "native"))]
use crate::is;

const SONG: &str = "rendered/song.wav";

fn rendered(then: &str) -> String {
	format!("play C4 for 0.1s\nmelody [C4 E4] each 0.05s\nseconds = render_sound(\"{SONG}\")\nwav = read(\"{SONG}\")\n{then}")
}

#[test]
fn a_rendered_wav_is_a_file_of_the_page() {
	is!(&rendered("round(seconds * 10)"), 2);
	is!(&rendered("byte_slice(wav, 0, 4)"), "RIFF");
	is!(&rendered("byte_slice(wav, 8, 12)"), "WAVE");
	// the samples after the 44 header bytes: 0.2 s of 16-bit samples at 22050 per second
	is!(&rendered("byte_at(wav, 40) + 256 * byte_at(wav, 41)"), 8820);
	is!(&rendered("play_file(\"rendered/song.wav\")\nexists(\"rendered/song.wav\")"), true);
}

#[test]
fn a_render_takes_the_sounds_after_the_last_one() {
	is!("play C4 for 0.2s\nrender_sound(\"first.wav\")\nplay E4 for 0.1s\nround(render_sound(\"second.wav\") * 10)", 1);
	is!("round(render_sound(\"nothing.wav\"))", 0);
}
