//! Card parameter-named-number (user decision B): `f x as T` casts the argument, f(x as T), wherever it stands, as
//! `f 2 + 1` is f(2 + 1); a cast of the result is written `(f x) as T`
use crate::is;

const FORMATS: &str = "format(number: int) := \"Integer \\(number)\"; format(number: float) := \"Float \\(number)\"; ";

#[test]
fn the_argument_is_cast_in_every_position() {
	is!(&format!("{FORMATS}format 2.2 as int"), "Integer 2");
	is!(&format!("{FORMATS}o = format 2.2 as int; o"), "Integer 2");
	is!(&format!("{FORMATS}g() := format 2.2 as int; g()"), "Integer 2");
	is!(&format!("{FORMATS}x: format 2.2 as int; x"), "Integer 2");
	is!(&format!("{FORMATS}if 1 then format 2.2 as int else 0"), "Integer 2");
}

#[test]
fn arithmetic_after_the_cast_stays_in_the_argument() {
	is!("f(x: int) := x * 10; o = f 2.2 as int + 1; o", 30);
	is!("f(x: int) := x * 10; f 2.2 as int + 1", 30);
}

#[test]
fn parentheses_cast_the_result() {
	is!("f(x) := x * 2; o = (f 2.5) as int; o", 5);
	is!(&format!("{FORMATS}o = format(2.2) as text; o"), "Float 2.2");
}
