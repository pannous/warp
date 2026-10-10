//! card derive-hard: every library word of one argument takes the name after it inside an operand, as sort and reverse
//! did from a hard-coded list (closed-lists rule, notes/cleanup.md)
use crate::is;

#[test]
fn a_one_argument_library_word_takes_the_name_after_it() {
	is!("xs=[1 2]; 1 == first xs", true);
	is!("xs=[1 2]; 2 == last xs", true);
	is!("xs=[1 2]; 1.5 == mean xs", true);
	is!("c='a'; 97 == ord c", true);
	is!("c='7'; yes == is_digit c", true);
	is!("m={a:1}; [\"a\"] == keys m", true);
	is!("m={a:1}; [1] == values m", true);
	is!("w=\"ab\"; \"ba\" == reverse w", true);
}
