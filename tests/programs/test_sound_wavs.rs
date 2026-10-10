// card sound-wavs: every warp process writes its sounds under names of its own in $TMPDIR/warp-sound, so a second run
// cannot overwrite a WAV the first run's player has queued but not played yet
#![cfg(feature = "native")]

/// The WAV paths a run of `program` reports on stderr (headless: `sound <seconds> s: <path>`)
fn sound_paths(tmpdir: &std::path::Path, program: &str) -> Vec<String> {
	let run = crate::common::warp_command().env("TMPDIR", tmpdir).args(["--no-ask", "eval", program]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr).into_owned();
	errors.lines().filter_map(|line| line.split_once(" s: ").map(|(_, path)| path.to_string())).collect()
}

#[test]
fn each_run_writes_wavs_of_its_own() {
	let tmpdir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sound-wavs");
	std::fs::create_dir_all(&tmpdir).unwrap();
	let program = "play C4 for 5ms\nplay E4 for 5ms\n7";
	let (first, second) = (sound_paths(&tmpdir, program), sound_paths(&tmpdir, program));
	assert_eq!((first.len(), second.len()), (2, 2), "{first:?} {second:?}");
	assert_ne!(first[0], first[1]);
	assert!(first.iter().all(|path| !second.contains(path)), "{first:?} {second:?}");
	assert!(first.iter().chain(&second).all(|path| std::path::Path::new(path).exists()));
}
