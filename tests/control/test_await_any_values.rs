// card await-hang: `await any` over tasks of plain expressions, which `go` computes right away, gives the first one's
// value; it used to poll the status of tasks that were never started, forever
use crate::is;

#[test]
fn await_any_of_computed_values_answers() {
	is!("await any [go 1+1, go 2+2]", 2);
	is!("a = go 1+1; b = go 2+2; await any [a, b]", 2);
	is!("f(x) := x * 2; await any [go 5+5, go f(2)]", 10);
}
