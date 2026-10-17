//! Card w0-globals: the type word of `global float y = 2` declares y, as `global y: float = 2` does (it was dropped)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_global_keeps_its_type_word() {
	fails_with("global int y = 2; y = 2.5", "y is declared int");
	fails_with("global float y = 2; y = \"ab\"", "y is declared float");
	is!("global float y = 2; def f() { y = 3.5 }; f(); y", 3.5);
}
