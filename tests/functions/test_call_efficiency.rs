// Calls stay as cheap as a plain call whatever form the function is declared in: a declared result type of the kind
// the body has already adds no box and no conversion (probes/call_benchmark.sh measures the forms)
use crate::common::calls_to;
use crate::is;

const UNTYPED: &str = "def f(a, b){ a + b }; f(2, 3)";
const TYPED: &str = "def f(a:int, b:int) -> int { a + b }; f(2, 3)";

#[test]
fn a_declared_int_result_adds_no_box() {
	is!(TYPED, 5);
	for runtime in ["new_int", "get_int_value"] {
		assert_eq!(calls_to(TYPED, runtime), calls_to(UNTYPED, runtime), "{runtime}");
	}
}

const RETURNED: &str = "def mk(k){ x => x + k }; h = mk(1); total = 0; for i in 1..3 { total = total + h(i) }; total";

#[test]
fn a_closure_of_one_known_target_is_called_directly() {
	is!(RETURNED, 5); // 1..3 leaves 3 out
	assert_eq!(calls_to(RETURNED, "closure_call_1"), 0);
	// a variable that may hold either of two closures still dispatches
	is!("def mk(k){ x => x + k }; def mul(k){ x => x * k }; h = mk(1); if 1 > 2 { h = mul(3) }; h(4)", 5);
	// a parameter is never taken for a variable of the same name
	is!("def mk(k){ x => x + k }; h = mk(1); def twice(h, x){ h(h(x)) }; twice(y => y * 3, 2)", 18);
}
