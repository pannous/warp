// card playground-go: a task (`go { }`) is a voice, natively (src/sound.rs Voice) and in the browser (host-tasks.js
// voicePlaced): its sounds start where its starter's voice stands and sound along with the starter's later ones,
// instead of queueing behind them. sound_queued() is the asking voice's; a busy machine only lowers it (the clock goes
// on, a cold start of the task takes seconds), so each test leaves seconds of room between the two outcomes
use crate::is;

#[test]
fn the_starter_does_not_queue_behind_its_task() {
	is!("job = go { play E4 for 2s }\nawait job\nplay C4 for 0.1s\nsound_queued() < 1", true); // queued behind: 2.1
}

#[test]
fn a_task_starts_where_its_starter_stands() {
	is!("play C4 for 5s\njob = go { play E4 for 5s\nsound_queued() > 6 }\nawait job", true); // voiced: 10, from now: 5
}
