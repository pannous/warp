// A runtime function only the emitted code turns out to call (print of a list: print_value, which the analysis leaves to
// the emitter) is emitted after main, not by emitting the whole program a second time (card takes-next)
use crate::common::function_index;
use crate::is;

#[test]
fn an_unforeseen_runtime_function_follows_main() {
	let code = "xs = [1, 2, 3]\nprint xs";
	let main = function_index(code, "main").expect("main");
	let print_value = function_index(code, "print_value").expect("print of a list calls print_value");
	assert!(print_value > main, "print_value {print_value} is emitted after main {main}");
}

#[test]
fn an_unforeseen_runtime_function_runs() {
	is!("xs = [1, 2, 3]\nprint xs\ncount(xs)", 3);
}
