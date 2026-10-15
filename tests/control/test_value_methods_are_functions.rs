use crate::is;

// card methods-last-n: methods([1 2]) and xs.methods hold the list module's functions, as methods(list) does, not
// their names as symbols (which also warned "`last_n` names nothing")

#[test]
fn a_list_value_s_methods_are_functions() {
	is!("fs = methods([1 2]); type(fs#1)", "function");
	is!("fs = methods([1 2]); f = fs#4; f([2 3])", 6);
	is!("xs = [1 2]; fs = xs.methods; type(fs#1)", "function");
}
