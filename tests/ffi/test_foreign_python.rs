//! `use python "math"`: a module of the Python runtime through the host word foreign_call (notes/stdlib_connectors.md)
use crate::common::fails_with;
use crate::is;

#[test]
fn test_python_calls_and_reads() {
	is!("use python \"math\"; math.sqrt(2)", 2f64.sqrt());
	is!("use python math; math.pi", std::f64::consts::PI);
	is!("use python math; math.floor(2.5) + 1", 3);
	is!("use python \"statistics\"; statistics.mean([1, 2, 3, 4])", 2.5);
	is!("use python \"os.path\" as path; path.join(\"a\", \"b\")", "a/b");
}

#[test]
fn test_python_values_cross_exactly() {
	is!("use python math; math.factorial(25) / math.factorial(24)", 25);
	is!("use python \"sys\"; count sys.version_info", 5);
}

#[test]
fn test_a_python_error_is_loud() {
	fails_with("use python math; math.nope(1)", "AttributeError");
}

#[test]
fn test_untrusted_code_gets_no_python() {
	// P88 (user 2026-10-05): untrusted code gets every capability for now, foreign ones (P89) included
	assert_eq!(warp::pipeline::eval_untrusted("use python math; math.pi"), std::f64::consts::PI);
}
