// `x += y` means `x = x + y` for every kind of x: a list concatenates, a number variable refuses a text
use crate::common::fails_with;
use warp::*;

#[test]
fn a_list_variable_concatenates_with_plus_assign() {
	is!("xs=[1 2]; xs+=[3]; count(xs)", 3);
	is!("xs=[\"ab\"]; xs+=[\"cd\"]; xs#2", "cd");
	is!("xs=[1 2]; for i in 0..2 { xs+=[i] }; xs", ints(vec![1, 2, 0, 1]));
}

#[test]
fn a_number_variable_refuses_a_text_in_a_loop_too() {
	fails_with("s=0; for x in [\"a\" \"b\"] { s+=x }; s", "type error: int + text");
	fails_with("s=0; for i in 0..1 { s+=\"ab\" }; s", "type error: int + text");
	is!("s=\"\"; for x in [\"a\" \"b\"] { s+=x }; s", "ab");
}
