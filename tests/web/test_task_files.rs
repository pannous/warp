// card task-sound-play: in the browser a go block runs on a task Worker with a fresh instance; it sees the files the
// program wrote before it (a rendered WAV it plays), and the program sees the files it wrote once awaited, as natively
// the tasks of a run share the disk (host-tasks.js startTask, finishedTask; task-worker.js)
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_task_plays_a_wav_rendered_before_it() {
	is!("play C4 for 0.1s\nrender_sound(\"before.wav\")\njob = go { play_file(\"before.wav\"); exists(\"before.wav\") }\nawait job", true);
}

#[test]
fn the_program_reads_what_its_task_wrote() {
	is!("use file\njob = go { write(\"task.txt\", \"from the task\"); 1 }\nawait job\nread(\"task.txt\")", "from the task");
}
