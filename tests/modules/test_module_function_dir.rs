// card use-math: the reflection words of a used module's function, `use math; dir(is_square)`, as of the program's own
// (tests/control/test_function_dir.rs); the module's functions join the program only when its use is resolved
use crate::is;

#[test]
fn a_used_module_function_reflects_like_the_program_own() {
	is!("use math\ncount(dir(is_square))", 4);
	is!("use math\nis_square.params#1", "n");
	is!("use math\nstr(type(is_square))", "function");
	is!("use math\nis_square(16)", true);
}
