//! cards op-div, both-map: floor division is its own operator, `//` and `div` Op::FloorDiv, `//=` Op::FloorDivAssign,
//! not true division's Op::Div and Op::DivAssign under another spelling
use crate::is;
use warp::operators::{spelled_operator, Op};

#[test]
fn floor_division_has_its_own_operators() {
	assert_eq!(spelled_operator("//"), Some(Op::FloorDiv));
	assert_eq!(spelled_operator("div"), Some(Op::FloorDiv));
	assert_eq!(spelled_operator("//="), Some(Op::FloorDivAssign));
	assert_eq!(spelled_operator("/="), Some(Op::DivAssign));
	assert_eq!(Op::FloorDivAssign.base_op(), Op::FloorDiv);
	assert!(Op::FloorDivAssign.is_compound_assign());
	assert_eq!(Op::FloorDiv.as_str(), "//");
	assert_eq!(Op::FloorDivAssign.as_str(), "//=");
}

#[test]
fn floor_division_and_true_division_compute_apart() {
	is!("7//2", 3);
	is!("-7//2", -4);
	is!("7 div 2", 3);
	is!("x = 7; x //= 2; x", 3);
	is!("x = 7.0; x /= 2; x", 3.5);
	is!("2*7//2", 7);
}
