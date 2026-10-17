// card sound-pro, step 1: `play` returns at once, its sound queued on the audio clock behind the sounds before it and
// played by a player thread (natively; the playground's WebAudio queue already did). sound_queued() is how many
// seconds the queued sounds still sound, wait_sound waits for them, stop_sound silences them, and the program's end
// waits for what is queued. Headless nothing sounds: the clock still runs, so these tests read it without speakers.
// Each program runs in a process of its own: the clock is the process's
#![cfg(feature = "native")]

const HEADLESS_WAIT_LIMIT_MILLISECONDS: f64 = 1_000.0;

/// What `program` prints, and how long its run took
fn printed(program: &str) -> (String, std::time::Duration) {
	let mut command = crate::common::warp_command();
	let started = std::time::Instant::now();
	let run = command.args(["--no-ask", "eval", program]).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout).into_owned();
	assert!(run.status.success(), "{printed} {}", String::from_utf8_lossy(&run.stderr));
	(printed, started.elapsed())
}

fn last_number(output: &str) -> f64 {
	output
		.split_whitespace()
		.last()
		.and_then(|number| number.parse().ok())
		.unwrap_or_else(|| panic!("no number: {output}"))
}

/// The seconds still queued after `program`, at most `seconds` queued and at least that minus the run's time: the clock
/// runs while the program does, so a loaded machine reads less (card flaky-sound)
fn assert_queued(program: &str, seconds: f64) {
	let (output, took) = printed(&format!("{program}\nsound_queued()"));
	let queued = last_number(&output);
	assert!(queued <= seconds && queued >= seconds - took.as_secs_f64(), "{queued} queued of {seconds} after {took:?}");
}

#[test]
fn sounds_queue_on_the_audio_clock() {
	assert_queued("use sound\nplay(C4, 2s)", 2.0);
	assert_queued("use sound\nplay(C4, 2s)\nplay([E4 G4], 1s)", 3.0);
	assert_queued("use sound\nmelody([C4 E4], 1s)", 2.0);
}

#[test]
fn stop_sound_empties_the_queue() {
	assert!(printed("use sound\nplay(C4, 2s)\nstop_sound\nsound_queued()").0.contains('0'));
}

#[test]
fn waiting_headless_takes_no_time() {
	let (output, _) = printed("sample_rate = 10\nplay(C4, 30s)\nstarted = clock()\nwait_sound\nclock() - started");
	let waited = last_number(&output);
	assert!(waited < HEADLESS_WAIT_LIMIT_MILLISECONDS, "wait_sound took {waited} ms");
}
