// `set x to 5`, `set width of p to 12` as library code: lib/english.warp's rewrite rules (card set-set, decision
// setter-calls), no compiler pass
use crate::is;

#[test]
fn set_phrases_come_from_the_english_module() {
	is!("x = 1\nset x to 5\nx", 5);
	is!("p = {width: 3}\nset width of p to 12\np.width", 12);
	is!("p = {width: 3}\nset p.width to 7\np.width", 7);
	is!("f() := { n = 1; set n to 5; n }\nf()", 5);
}

// a program defining `set` keeps its own word: the standard rules of a word the program binds stay out
#[test]
fn own_set_shadows_the_english_rule() {
	is!("set(a, b) := a * b\nset(3, 4)", 12);
}
