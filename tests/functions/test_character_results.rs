// A function returning a character returns that character, not its code point (was 98 for 'b', and `f('b') == 'b'`
// was false)
use crate::is;

#[test]
fn a_returned_character_stays_a_character() {
	is!("def f(){ 'b' }; f()", 'b');
	is!("def f(x){ x }; f('b')", 'b');
	is!("def f(x){ x }; f(\"b\")", 'b'); // a one-character text lexes as a character
	is!("def f(x:char){ x }; f('b')", 'b');
	is!("def f(x){ x }; f('b') == 'b'", 1);
	is!("def up(c){ c }; up('x') + 'y'", "xy");
}
