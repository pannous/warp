// card print-oldest (user, 2026-10-10): a text joins anything in its str() form, `"oldest first: " + list` needs no
// str(list); the one exception, a text spelling a number plus a number (`"3"+3`), warns ("a"+3 does not)
use warp::diagnostic::take_warnings;
use crate::is;

fn warns_of_spelled_number(code: &str) -> bool {
	take_warnings();
	warp::wasm_emitter::eval(code);
	take_warnings().iter().any(|warning| warning.message.contains("spells a number"))
}

#[test]
fn a_text_joins_anything_in_its_text_form() {
	is!("\"oldest: \" + [1, 2]", "oldest: [1 2]");
	is!("[1 2] + \"z\"", "[1 2]z");
	is!("class P{name: text}\n\"p: \" + P(\"Bo\")", "p: P{name:\"Bo\"}");
	is!("\"k: \" + {a:1}", "k: {a:1}");
	is!("\"x\" + ø", "xø");
	is!("s = \"a\"\ns += [1 2]\ns", "a[1 2]");
	is!("class P{name: text; age: int}\npeople = [P(\"Al\", 30), P(\"Bo\", 50)]\n\"oldest first: \" + (people sorted by -it.age).map(p => p.name)", "oldest first: [\"Bo\" \"Al\"]");
}

#[test]
fn a_text_spelling_a_number_plus_a_number_warns() {
	assert!(warns_of_spelled_number("\"3\"+3"));
	assert!(warns_of_spelled_number("3 + \"4.5\""));
	assert!(!warns_of_spelled_number("\"a\"+3"));
	is!("\"a\"+3", "a3");
}
