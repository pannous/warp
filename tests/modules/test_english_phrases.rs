// `set width of p to 12` as library code (`set x to 5` is not: write x = 5): lib/english.warp's rewrite rules (card set-set, decision
// setter-calls), no compiler pass
use crate::is;

#[test]
fn set_phrases_come_from_the_english_module() {
	is!("p = {width: 3}\nset width of p to 12\np.width", 12);
}

// a program defining `set` keeps its own word: the standard rules of a word the program binds stay out
#[test]
fn own_set_shadows_the_english_rule() {
	is!("set = 7\nset + 1", 8);
}

// a quote is code as written: no rewrite rule reaches inside `data(…)`
#[test]
fn a_quoted_set_phrase_stays_as_written() {
	is!("data(set width of p to 12) == parse_code(\"set width of p to 12\")", true);
}
