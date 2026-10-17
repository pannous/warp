// card host-word: a host word's failure inside a try is the program's to catch, as raise is; outside a try it stays a
// loud error. The browser host catches only gpuJob's failures so far (card host-word)
#![cfg(feature = "native")]
use crate::common::fails_with;
use crate::is;

#[test]
fn a_failed_save_is_caught() {
	is!(
		"use draw; try { save \"/nonexistent/dir/x.png\" } catch e { \"caught \" + e }",
		"caught save: cannot write /nonexistent/dir/x.png: No such file or directory (os error 2)"
	);
	fails_with("use draw; save \"/nonexistent/dir/x.png\"", "cannot write /nonexistent/dir/x.png");
}

#[test]
fn a_failed_eval_is_caught() {
	is!("try { eval(\"1 +\") } catch e { is_error(e) }", 1);
	fails_with("eval(\"1 +\")", "not a number");
}
