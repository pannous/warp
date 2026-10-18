//! card field-default-constant: a field's default may name a variable (`tag: text = mark`), it is a value and no
//! type; a field named `result` is a field, not the keyword for the statement before
use crate::is;

const MARK: &str = "mark = \"?\"\n";

#[test]
fn a_default_naming_a_variable_is_its_value() {
	is!(&format!("{MARK}class F {{ name: text; tag: text = mark }}\nf = F(\"a\")\nf.tag"), "?");
	is!(&format!("{MARK}class F {{ name: text; tag = mark }}\nF(\"a\").tag"), "?");
	is!(&format!("{MARK}class F {{ name: text; tag = mark }}\nF(\"a\", \"!\").tag"), "!");
}

#[test]
fn a_field_named_result_is_a_field() {
	is!("class F { result: int }; F(3).result", 3);
	is!(&format!("{MARK}class F {{ name: text; result: text = mark }}\nF(\"a\").result"), "?");
	is!("class F { result: int }\nf = F(3)\n5\nresult + f.result", 8);
}
