// card sound-pro, step 1: `play` returns at once, its sound queued on the audio clock behind the sounds before it and
// played by a player thread (natively; the playground's WebAudio queue already did). sound_queued() is how many
// seconds the queued sounds still sound, wait_sound waits for them, stop_sound silences them, and the program's end
// waits for what is queued. Headless nothing sounds: the clock still runs, so these tests read it without speakers.
// Each program runs in a process of its own: the clock is the process's
#![cfg(feature = "native")]

/// What `program` prints, and how long its run took
fn printed(program: &str) -> (String, std::time::Duration) {
	let started = std::time::Instant::now();
	let run = crate::common::warp_command().args(["--no-ask", "eval", program]).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout).into_owned();
	assert!(run.status.success(), "{printed} {}", String::from_utf8_lossy(&run.stderr));
	(printed, started.elapsed())
}

#[test]
fn sounds_queue_on_the_audio_clock() {
	assert!(printed("play C4 for 2s\nround(sound_queued())").0.contains('2'));
	assert!(printed("play C4 for 2s\nplay [E4 G4] for 1s\nround(sound_queued())").0.contains('3'));
	assert!(printed("melody [C4 E4] each 1s\nround(sound_queued())").0.contains('2'));
}

#[test]
fn stop_sound_empties_the_queue() {
	assert!(printed("play C4 for 2s\nstop_sound\nsound_queued()").0.contains('0'));
}

#[test]
fn waiting_headless_takes_no_time() {
	let (output, took) = printed("play C4 for 3s\nwait_sound\n7");
	assert!(output.contains('7') && took < std::time::Duration::from_secs(3), "{took:?}");
}
