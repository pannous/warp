//! Card multi-word-class (D5 rule 4, notes/matching.md): `class full name {…}` declares the class `full-name`; the
//! spaced words `full name` name it wherever they stand once declared, and a `to` slot matches it as a whole.

use crate::is;

const FULL_NAME: &str = "class full name { given:text; family:text }; ";

#[test]
fn a_kebab_class_constructs() {
	is!("class full-name{given:text; family:text}; p = full-name{given:\"Ann\", family:\"Lee\"}; p.family", "Lee");
}

#[test]
fn a_class_of_several_words_is_its_kebab_name() {
	is!(&format!("{FULL_NAME}p = full name{{given:\"Ann\", family:\"Lee\"}}; p.family"), "Lee");
	is!(&format!("{FULL_NAME}p = full-name{{given:\"Ann\", family:\"Lee\"}}; p.given"), "Ann");
	is!(&format!("{FULL_NAME}p = full name{{given:\"Ann\", family:\"Lee\"}}; type of p"), "full-name");
}

#[test]
fn a_phrase_slot_matches_the_class_as_a_whole() {
	is!(
		&format!("{FULL_NAME}to greet a full name: \"hi \" + full name.given; greet full name{{given:\"Ann\", family:\"Lee\"}}"),
		"hi Ann"
	);
	is!(
		&format!(
			"{FULL_NAME}greet(n: full name) := \"hi \" + n.family; greet(full name{{given:\"Ann\", family:\"Lee\"}})"
		),
		"hi Lee"
	);
}

#[test]
fn the_words_stay_apart_without_the_class() {
	is!("full = 3; name = 4; [full name]#2", 4);
}
