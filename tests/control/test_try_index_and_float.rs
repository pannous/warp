// `try X else Y` around an index of a value that may be no list, and around a float value; an if with a float branch
use warp::*;

#[test]
fn indexing_what_may_be_no_list_falls_back() {
	is!("xs = [[1] 2]; try xs#2#1 else 7", 7);
	is!("n = 5; try n#1 else 7", 7);
	is!("xs = [1 2]; try xs#9 else 7", 7);
	is!("xs = [1 2]; try xs#2 else 7", 2);
	is!("try [1 2]#3 else 7", 7);
}

#[test]
fn a_float_value_passes_through_try_and_if() {
	is!("x = 1.5 as float; if 1 then {x} else {0}", 1.5);
	is!("try 1.5 as float else 0", 1.5);
	is!("f(c) := { x = 1.5 as float; if c then {x} else {0} }; f(1)", 1.5);
}
