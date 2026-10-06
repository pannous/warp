// A number subscript on a variable that starts as the empty map `{}` keys it (user, P34): `d={}; d[1]="a"` is {1:"a"}
use crate::is;

#[test]
fn a_number_subscript_keys_an_empty_map() {
	is!("d={}; d[1]=\"a\"; d[1]", "a");
	is!("d={}; d[1]=\"a\"; d[5]=\"b\"; count(d)", 2);
	is!("d={}; d[7]=3; d[7] + 1", 4);
}

#[test]
fn a_list_keeps_its_positions() {
	is!("xs=[5,6]; xs[1]", 6);
}
