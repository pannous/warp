// card yes-stores: a variable declared any holds any value as it is: a bool stays a bool, and a later value of
// another kind replaces it (`y: any = yes; y = "a"` was 97)
use crate::is;

#[test]
fn any_keeps_a_bool() {
	is!("y: any = yes; y", true);
	is!("y: any = yes; y is bool", true);
	is!("y: any = 2 > 1; y", true);
	is!("y: bool = yes; z: any = y; z", true);
}

#[test]
fn any_takes_a_value_of_another_kind() {
	is!("y: any = yes; y = \"a\"; y", "a");
	is!("y: any = 1; y = 2.5; y", 2.5);
	is!("y: any = 1; y + 1", 2);
}
