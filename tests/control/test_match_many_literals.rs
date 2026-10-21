//! A match of many literal arms (card pattern-dispatch-speed: tested as a binary search on the literals) picks the
//! same arm as one test after another would: the first of a literal written twice, a codepoint as its text, a symbol or
//! another kind no text arm, a float an equal whole number's arm, a miss the default or the no-case error
use crate::common::fails_with;
use crate::is;

const LETTERS: &str = "f(s) := match s { \"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4, \"e\": 5, \"f\": 6, \"g\": 7, \"h\": 8, \"i\": 9, \"j\": 10, _: 0 }";
const NUMBERS: &str = "n(x) := match x { 1: \"one\", 2: \"two\", 3: \"three\", 4: \"four\", 5: \"five\", 6: \"six\", 7: \"seven\", 8: \"eight\", 9: \"nine\", 10: \"ten\", _: \"many\" }";

#[test]
fn a_match_of_many_texts_picks_each_arm() {
	is!(&format!("{LETTERS}; f(\"j\") * 10000 + f(\"a\") * 1000 + f(\"e\") * 100 + f(\"zz\") * 10 + f(\"\")"), 101500);
	is!(&format!("{LETTERS}; f('c')"), 3);
	is!(&format!("{LETTERS}; f(symbol(\"a\")) + f(3)"), 0);
	is!("o(s) := match s { \"+\": 1, \"-\": 2, \"*\": 3, \"/\": 4, \"==\": 5, \"!=\": 6, \"<=\": 7, \">=\": 8, \"<\": 9, \">\": 10, \"and\": 11, \"or\": 12, _: 0 }; o(\"+\") * 1000000 + o(\">=\") * 10000 + o(\">\") * 100 + o(\"and\") + o(\"x\") + o(symbol(\"+\"))", 1081011);
	is!("g(s) := match s { \"a\": 1, \"b\": 2, \"a\": 99, \"c\": 3, \"d\": 4, \"e\": 5, \"f\": 6, \"g\": 7, \"h\": 8, _: 0 }; g(\"a\")", 1);
	fails_with("h(s) := match s { \"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4, \"e\": 5, \"f\": 6, \"g\": 7, \"h\": 8 }; h(\"q\")", "no case for s = \"q\"");
}

#[test]
fn a_match_of_many_numbers_picks_each_arm() {
	is!(&format!("{NUMBERS}; n(3.0) + n(10) + n(11) + n(\"3\")"), "threetenmanymany");
}

#[test]
fn literal_arms_keep_their_place_among_other_arms() {
	is!("k(s) := match s { \"a\" => 1; \"b\" => 2; \"c\" => 3; \"d\" => 4; \"e\" => 5; \"f\" => 6; \"g\" => 7; \"h\" => 8; t if t > \"x\" => 50; _ => 0 }; k(\"y\") + k(\"h\") * 2 + k(\"m\")", 66);
	is!("p(s) := match s { \"a\" => 1; \"b\" => 2; \"c\" => 3; \"d\" => 4; \"e\" => 5; \"f\" => 6; \"g\" => 7; \"h\" => 8; [x, y] => x + y; n if n is int => n * 100; \"z\" => 26; _ => 0 }; p(\"h\") + p([1, 2]) * 10 + p(3) + p(\"z\") * 10000 + p(\"q\")", 260338);
	is!("k(s) := match s { > \"x\": 50, \"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4, \"e\": 5, \"f\": 6, \"g\": 7, \"h\": 8, _: 0 }; k(\"y\") + k(\"h\") * 2 + k(\"m\")", 66);
}
