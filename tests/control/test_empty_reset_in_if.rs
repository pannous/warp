// card empty-reset-in-if: an assignment of ø (`f = []`) is the Node a variable holding it is, so an `if` statement ending
// in one is no number (it was read as one: 'not an int')
use crate::is;

#[test]
fn an_if_resetting_a_list_to_empty_runs() {
	is!("f = []; if 2 > 1 { f = [] }; 7", 7);
	is!("f = ø; if 2 > 1 { f = ø }; 7", 7);
	is!("f = []; if 2 > 1 { f = [] } else { 0 }; 7", 7);
	is!("x = 1; f = []; on change x { f = [] }; x = 2; 7", 7);
	is!("f = [1]; if 2 > 1 { f = [] }; count(f)", 0);
}
