//! card field-before-library-word: a declared field named like a library word (first, last, reverse) wins over the
//! word when the value's class is known; `xs.first` of a list stays the library word
use crate::is;

const PERSON: &str = "class person { first: text; last: text }";

#[test]
fn a_known_instance_reads_its_field() {
	is!(&format!("{PERSON}\nn = person(\"Ada\", \"Lovelace\")\nn.first"), "Ada");
	is!(&format!("{PERSON}\nn = person(\"Ada\", \"Lovelace\")\nn.first + \" \" + n.last"), "Ada Lovelace");
	is!("class full name { first: text; last: text }\nn = full name(\"Ada\", \"Lovelace\")\nn.last", "Lovelace");
}

#[test]
fn a_list_keeps_the_library_word() {
	is!(&format!("{PERSON}\nxs = [3, 4]\nxs.first"), 3);
}
