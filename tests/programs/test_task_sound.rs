// card task-sound: a task runs in a fresh instance of the module, whose globals were unset: `go { beep }` played 0.00 s
// (lib/sound.warp's sample_rate was 0); a task's instance now gets the program's globals as they are at its start
use crate::is;

#[test]
fn a_task_reads_the_programs_globals() {
	is!("global k = 5\nf() := k + 1\njob = go { f() }\nawait job", 6);
	is!("use sound\njob = go { sample_rate }\nawait job", 22050);
}

#[cfg(feature = "native")]
#[test]
fn a_sound_in_a_task_plays_its_length() {
	let run = crate::common::warp_command().args(["--no-ask", "eval", "use sound\njob = go { beep }\nawait job"]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(errors.contains("sound 0.15 s"), "{errors}");
	let wav = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("task-sound.wav");
	std::fs::write(&wav, warp::sound::wav(&[32768, 40000, 32768], 22050)).unwrap();
	let played = crate::common::warp_command().args(["--no-ask", "eval", &format!("use sound\njob = go {{ play \"{}\" }}\nawait job", wav.display())]).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&played.stdout).trim(), "» 1", "{}", String::from_utf8_lossy(&played.stderr));
}
