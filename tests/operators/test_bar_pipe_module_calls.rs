// card pipes-after (warp-reviewer on unix-words): `ls "/tmp" | count` ran ls("/tmp" | count) and `print cat f | count`
// trapped: a `|` after a braceless call pipes the call, also of a used module's word (which joins the program after the
// pipes are read) and inside print
#![cfg(feature = "native")]
use crate::common::{printed, scratch_directory};

#[test]
fn a_bar_after_a_module_call_pipes_the_call() {
	let folder = scratch_directory("bar_pipe_module_calls");
	std::fs::create_dir_all(&folder).unwrap();
	std::fs::write(folder.join("a.txt"), "hello").unwrap();
	std::fs::write(folder.join("b.txt"), "").unwrap();
	let (folder, file) = (folder.display(), folder.join("a.txt").display().to_string());
	assert_eq!(printed(&format!("use script; print ls \"{folder}\" | count")).trim(), "2");
	assert_eq!(printed(&format!("use script; print cat \"{file}\" | count")).trim(), "5");
	assert_eq!(printed(&format!("use script; n = cat \"{file}\" | count; print n")).trim(), "5");
}
