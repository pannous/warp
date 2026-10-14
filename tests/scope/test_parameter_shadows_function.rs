// found writing probes/self_gaps/file_bytes.warp: a parameter named like a global function or global value is the
// parameter; `[path, text]` in lib/file.warp's write called the program's `path` with text (path takes 0 arguments)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_list_of_a_parameter_named_like_a_function_lists_it() {
	is!("f(x) := x + 1; g(f, b) := [f, b]; g(1, 2)", parse("[1 2]"));
	is!("f := 3; g(f, b) := [f, b]; g(1, 2)", parse("[1 2]"));
	is!("path := \"x\"; g(path, t) := [path, t]; g(\"ab\", \"cd\")", parse("[\"ab\" \"cd\"]"));
}

#[test]
fn a_global_named_like_a_module_parameter_leaves_the_module_word_alone() {
	let _ = std::fs::create_dir_all("scratch/test_parameter_shadows_function");
	is!("path := \"scratch/test_parameter_shadows_function/a.txt\"; write(path, \"hi\"); read(path)", "hi");
}

#[test]
fn a_function_parameter_is_still_called() {
	is!("apply(f, x, y) := f(x, y); add(a, b) := a + b; apply(add, 1, 2)", 3);
	is!("apply(f, x) := f x; inc(a) := a + 1; apply(inc, 1)", 2);
}
