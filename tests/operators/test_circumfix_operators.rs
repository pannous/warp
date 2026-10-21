// `circumfix operator ⌊ ⌋ := floor` (lib/math.warp): a pair of glyphs bracketing an expression applies a function to
// it, like parentheses around its argument; a program declares its own pairs the same way (card circumfix-operators-user)
use crate::is;

#[test]
fn math_circumfix_operators() {
	is!("⌊3.7⌋", 3);
	is!("⌈3.2⌉", 4);
	is!("⌊-3.7⌋+1", -3);
	is!("‖-5‖+1", 6);
	is!("‖3-5‖*2", 4);
	is!("⌈‖-2.5‖⌉", 3);
}

#[test]
fn a_program_declares_its_circumfix_operator() {
	is!("circumfix operator ⟦ ⟧ := round; ⟦2.6⟧", 3);
	is!("half x := x/2; circumfix operator ⟦ ⟧ := half; ⟦3+5⟧ + 1", 5);
}
