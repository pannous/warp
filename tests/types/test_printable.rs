// The Printable operation (user, P31): `text(p:person) := …`, the one function a type word may name, gives the text of
// a person for `as text`, str, text, print and interpolation; other values keep their own text
use warp::is;

const PERSON: &str = "class person{name:text}; text(p:person) := \"P \" + p.name; x = person{name:\"a\"}; ";

#[test]
fn as_text_str_and_text_call_the_printable_operation() {
	is!(&format!("{PERSON}x as text"), "P a");
	is!(&format!("{PERSON}str(x)"), "P a");
	is!(&format!("{PERSON}text(x)"), "P a");
	is!(&format!("{PERSON}3 as text"), "3");
}

#[test]
fn print_and_interpolation_call_the_printable_operation() {
	is!(&format!("{PERSON}print x"), "P a");
	is!(&format!("{PERSON}\"is \\(x)\""), "is P a");
	is!(&format!("{PERSON}xs=[x]; print xs#1"), "P a");
}
