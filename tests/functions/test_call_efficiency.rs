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
