//! An escaped quote leaves its text open: `//` after it is text, no comment hiding a closing bracket (found by card
//! self-serializer: a view of a source with `parse_json("{'a':1, // note}")` in a text)
use crate::is;

#[test]
fn slashes_after_an_escaped_quote_are_text() {
	is!(r#"x = "a \"{1, // b}\" c"; x"#, r#"a "{1, // b}" c"#);
	is!(r#"x = ["\"(", "// )"]; #x"#, 2);
}

// the uniscript entity pass (expand_entities) skips texts, also past an escaped quote: `\\:infinity` stays as written
#[test]
fn entities_after_an_escaped_quote_stay_text() {
	is!(r#"x = "\"a \\:infinity"; #x"#, 13);
}
