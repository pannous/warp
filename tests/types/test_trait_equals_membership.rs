// `x in xs` and `xs.has(x)` of an instance whose type overrides equality search by its equals
use crate::is;

const WORDS: &str = "class word{text:string}; equals(a:word, b:word) := lower(a.text) == lower(b.text); ws=[word(\"Hello\"), word(\"World\")]; ";

#[test]
fn membership_uses_the_equals_override() {
	is!(&format!("{WORDS}word(\"world\") in ws"), 2);
	is!(&format!("{WORDS}word(\"nope\") in ws"), 0);
	is!(&format!("{WORDS}ws.has(word(\"HELLO\"))"), 1);
}
