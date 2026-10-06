//! P48 (user-decided): an infix operator's body names its operands `left` and `right` (`a` and `b` stay accepted)
use crate::is;

#[test]
fn test_infix_operands_named_left_and_right() {
	is!("infix operator ⊕ := left*10+right; 2 ⊕ 3", 23);
	is!("infix operator ⊕ := a*10+b; 2 ⊕ 3", 23);
}
