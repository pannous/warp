//! `x is int` of a foreign value is a type test of its run-time kind, not the runtime's `operator.eq(x, int)` (card int-bool)
use crate::is;

#[test]
fn test_type_test_of_a_python_result() {
	is!("use python math; math.floor(2.5) is int", true);
	is!("use python math; x = math.floor(2.5); x is int", true);
	is!("use python math; math.sqrt(4.0) is float", true);
	is!("use python math; math.sqrt(4.0) is int", false);
	is!("use python math; math.isclose(1.0, 1.0) is bool", true);
	is!("use python math; math.floor(2.5) is a number", true);
}

#[test]
fn test_python_bool_stays_bool() {
	is!("use python math; math.isclose(1.0, 1.0)", true);
}
