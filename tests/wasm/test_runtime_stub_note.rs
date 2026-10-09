// card runtime-stub-note: the note 'building the runtime stub for executables once' comes before the stub's first
// build only, not with every program whose executable is (re)written
#![cfg(feature = "native")]

const NOTE: &str = "building the runtime stub";

/// What `warp <file>` says on stderr for a fresh program file `name`
fn stderr_of_fresh_program(name: &str) -> String {
	let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("runtime_stub_note");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join(format!("{name}.warp"));
	let _ = std::fs::remove_file(program.with_extension(std::env::consts::EXE_EXTENSION));
	std::fs::write(&program, format!("// {name}\n6 * 7")).unwrap();
	let output = crate::common::warp_command().arg("--no-ask").arg(&program).output().unwrap();
	String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn the_note_comes_only_before_the_first_build() {
	stderr_of_fresh_program("first"); // builds the stub if there is none yet
	let second = stderr_of_fresh_program("second");
	assert!(!second.contains(NOTE), "{second}");
}
