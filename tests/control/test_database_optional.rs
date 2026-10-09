// card orm-optional: an optional field (`email: text?`) is a nullable column; a row without a value reads as ø
use crate::is;
use warp::wasm_emitter::eval;

const USER: &str = "class User{name: text; email: text?; age: int?}\nusers: [User] = database.users_optional";

#[test]
fn an_optional_field_is_a_nullable_column() {
	eval(&format!("{USER}\nusers.add(User(\"Al\", \"al@b.c\", 40))\nusers.add(User(\"Bo\", ø, ø))"));
	is!(&format!("{USER}\nusers#1.email"), "al@b.c");
	is!(&format!("{USER}\nusers#2.email == ø"), true);
	is!(&format!("{USER}\nusers#2.age == ø"), true);
	is!(&format!("{USER}\ncount(users where it.email == ø)"), 1);
}

#[test]
fn an_optional_field_added_later_starts_empty() {
	eval("class Pen{color: text}\npens: [Pen] = database.pens_optional\npens.add(Pen(\"red\"))");
	is!("class Pen{color: text; owner: text?}\npens: [Pen] = database.pens_optional\npens#1.owner == ø", true);
}

#[test]
fn a_field_made_optional_keeps_its_values() {
	eval("class Cup{label: text}\ncups: [Cup] = database.cups_optional\ncups.add(Cup(\"big\"))");
	is!("class Cup{label: text?}\ncups: [Cup] = database.cups_optional\ncups#1.label", "big");
}
