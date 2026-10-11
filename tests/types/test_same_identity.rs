//! `same` / `===` is identity on objects (P208): two names holding one instance, map or list are the same, two equal
//! objects made apart are not; `==` compares them field by field. On scalars `===` stays type and value (P196).
use crate::is;

const POINT: &str = "class P { x: int }; ";

#[test]
fn an_alias_is_the_same_object() {
	is!(&format!("{POINT}p = P(1); q = p; p === q"), true);
	is!(&format!("{POINT}p = P(1); q = P(1); p === q"), false);
	is!(&format!("{POINT}p = P(1); q = P(1); p !== q"), true);
	is!(&format!("{POINT}p = P(1); q = P(1); p == q"), true);
	is!(&format!("{POINT}p = P(1); p.copy() === p"), false);
}

#[test]
fn maps_and_lists_are_the_same_only_when_one() {
	is!("m = {a:1}; n = m; m === n", true);
	is!("m = {a:1}; n = {a:1}; m === n", false);
	is!("xs = [1, 2]; ys = [1, 2]; xs === ys", false);
}

#[test]
fn same_is_the_word() {
	is!(&format!("{POINT}p = P(1); q = p; p same q"), true);
	is!(&format!("{POINT}p = P(1); q = P(1); p same as q"), false);
	is!(&format!("{POINT}p = P(1); q = p; p is the same as q"), true);
	is!(&format!("{POINT}p = P(1); q = P(1); p is the same as q"), false);
	is!("same = 3; same + 1", 4);
}

#[test]
fn scalars_stay_type_and_value() {
	is!("0 === no", false);
	is!("1 === 1.0", true);
	is!("\"ab\" === \"ab\"", true);
}
