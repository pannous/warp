// card playground-drops: in the browser render_sound takes a task's sounds too, mixed at their places along with the
// program's, as natively (src/sound.rs render; host-files.js mixed, host-tasks.js withTaskSounds). Browser only: in-process
// native runs share the host's unrendered sounds (tests/programs/test_sound_voices.rs renders through the CLI)
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_render_mixes_a_tasks_sounds_in() {
	// the task's 0.2 s along with the program's 0.1 s: 0.2 s; dropped it would be 0.1, queued behind 0.3
	is!("job = go { play C4 for 0.2s }\nplay E4 for 0.1s\nawait job\nround(render_sound(\"voices.wav\") * 10)", 2);
}
