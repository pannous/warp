// card cwd-artifacts: running or compiling a program leaves nothing in the current directory; the last module, kept for
// inspection, goes to one fixed path (warp::wasm_emitter::debug_module_path)
use std::path::PathBuf;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

#[test]
fn eval_leaves_no_module_in_the_current_directory() {
	let directory = PathBuf::from(OUTPUT_DIRECTORY).join("cwd_debris");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let run = crate::common::warp_command().current_dir(&directory).args(["eval", "1+2"]).output().unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	let left: Vec<_> = std::fs::read_dir(&directory).unwrap().map(|entry| entry.unwrap().file_name()).collect();
	assert!(left.is_empty(), "left in the current directory: {left:?}");
}

#[test]
fn the_last_module_is_kept_at_its_fixed_path() {
	let path = warp::wasm_emitter::debug_module_path().expect("HOME is set");
	assert!(path.ends_with("last.wasm"), "{}", path.display());
	assert!(!path.starts_with(std::env::current_dir().unwrap()), "{}", path.display());
}
