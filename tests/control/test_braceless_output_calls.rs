// `puti i` without parentheses is a call worth an Int, like `puti(i)`: as a loop's only body statement it is the loop's
// value (it was typed as a list: the loop gave its counter, `x = puti 7; x + 1` was a type error)
use crate::is;

#[test]
fn a_braceless_output_call_is_worth_an_int() {
	is!("for i in 1 to 5 : puti i", 5);
	is!("x = puti 7; x + 1", 8);
}
