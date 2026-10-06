//! The elements of split and chars are texts: `+` concatenates them, it does not add numbers
use crate::is;

#[test]
fn split_elements_concatenate() {
	is!("p = split(\"a,b\", \",\"); p[0] + 3", "a3");
	is!("split(\"a{}b\", \"{}\")[1] + 3", "b3");
	is!("chars(\"ab\")[0] + 3", "a3");
	is!("r = \"\"; for w in split(\"a b\", \" \") { r = r + w + 1 }; r", "a1b1");
}
