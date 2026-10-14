//! `use os; args` (card std-args, notes/stdlib.md): the command line arguments after the program file, `warp run
//! prog.warp a b` gives ["a" "b"]; without any, and in the browser, the empty list
use crate::is;

#[test]
fn args_without_arguments_is_the_empty_list() {
	is!("use os; count(args)", 0);
}

#[cfg(feature = "native")]
#[test]
fn args_are_the_words_after_the_program_file() {
	let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_std_args");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("prog.warp");
	std::fs::write(&program, "use os; print(count(args)); print(args#2)").unwrap();
	let output = crate::common::warp_command().arg("run").arg(&program).args(["a", "bee"]).output().unwrap();
	let printed = String::from_utf8_lossy(&output.stdout);
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert!(printed.contains("2\nbee"), "{printed}");
}
