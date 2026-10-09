// str of a computed value is the text of its value, data reads as written (card str-inline)
use crate::is;

#[test]
fn str_of_a_field_read_is_its_value() {
	is!("class Rope{length: m}\nstr(Rope(5 m).length)", "5m");
	is!("class P{x:number}\nstr(P(5).x)", "5");
	is!("str({a:\"x\"}.a)", "x");
	is!("str([\"a\", \"b\"]#2)", "b");
}

#[test]
fn str_of_literal_arithmetic_is_its_value() {
	is!("str(\"a\" + \"b\")", "ab");
	is!("str([1, 2] + [3])", "[1 2 3]");
}

#[test]
fn str_of_data_reads_as_written() {
	is!("str(a:b)", "a:b");
}
