// card sound-pro, step 3: play gives the sound's handle, its number in the run (tones, melodies and music files counted
// together), and stop_sound(handle) stops that one alone: dropped from the queue, its player ended; stop_sound() all
#![cfg(feature = "native")]

/// What `program` prints, run as its own process: the handles count from the run's start (the test process plays too)
fn printed(program: &str) -> String {
	let run = crate::common::warp_command().args(["--no-ask", "eval", program]).output().unwrap();
	String::from_utf8_lossy(&run.stdout).trim().trim_start_matches("» ").to_string()
}

#[test]
fn play_gives_the_sounds_handle() {
	assert_eq!(printed("a = play C4 for 5ms\nb = melody [C4 E4] each 5ms\nc = tone 440Hz for 5ms\n[a b c]"), "[1 2 3]");
	let wav = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sound-handles.wav");
	std::fs::write(&wav, warp::sound::wav(&[32768, 40000, 32768], 22050)).unwrap();
	assert_eq!(printed(&format!("play C4 for 5ms\nplay \"{}\"", wav.display())), "2");
}

#[test]
fn stop_sound_stops_one_sound() {
	assert_eq!(printed("a = play C4 for 5ms\nb = play E4 for 5ms\nstop_sound(a)\nstop_sound()\n7"), "7");
	assert!(printed("play C4 for 5ms\nstop_sound(9)").contains("no sound 9 in this run"));
}
