// card playground-go: a task (`go { }`) is a voice, natively (src/sound.rs Voice) and in the browser (host-tasks.js
// voicePlaced): its sounds start where its starter's voice stands and sound along with the starter's later ones,
// instead of queueing behind them. sound_queued() is the asking voice's
use crate::is;

#[test]
fn the_starter_does_not_queue_behind_its_task() {
	is!("job = go { play E4 for 1s }\nawait job\nplay C4 for 0.1s\nround(sound_queued() * 10)", 1);
}

#[test]
fn a_task_starts_where_its_starter_stands() {
	is!("play C4 for 1s\njob = go { play E4 for 0.1s\nround(sound_queued() * 10) }\nawait job", 11);
}
