// samples/neural_net.warp: a function declares several main-level variables global, Python's `global a, b` or one
// `global` line each (two of them in a block were the "duplicate key 'global'" of an object)
use crate::is;

#[test]
fn global_names_several_variables() {
	is!("a = 1; b = 2; f() := { global a, b; a = 5; b = 6 }; f(); a + b", 11);
	is!("a = 1; b = 2; def f() {\n global a, b\n a = 5\n b = 6 }\nf(); a + b", 11);
}

#[test]
fn a_block_declares_several_globals() {
	is!("a = 1; b = 2; def f() { global a; global b; a = 5; b = 6 }; f(); a + b", 11);
	is!("a = 1; b = 2; def f() {\n global a\n global b\n a = 5\n b = 6 }\nf(); a + b", 11);
}
