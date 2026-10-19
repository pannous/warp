// A runtime function only the emitted code turns out to call (print of a list: print_value, which the analysis leaves to
// the emitter) is emitted after main, not by emitting the whole program a second time (card takes-next)
use crate::is;

#[test]
fn an_unforeseen_runtime_function_runs() {
	is!("xs = [1, 2, 3]\nprint xs\ncount(xs)", 3);
}
