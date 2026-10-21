// card match-arm-whose: an arm whose value is an unbraced match, `1: match n {…}`, keeps the outer match's next arms;
// the inner match took them as more of its own words
use crate::is;

const NUMBER_ARMS: &str = "f(n) := match n {\n\t1: match n + 1 { 2: \"two\"; _: \"never\" }\n\t_: \"other\"\n}\n";
const NODE_ARMS: &str = "f(n: Node): any := match n {\n\tKey(v, Op.Else, f): match v { Key(c, Op.Then, y): \"then\"; _: \"plain\" }\n\t_: \"other\"\n}\n";

#[test]
fn an_unbraced_inner_match_keeps_the_outer_arms() {
	is!(&format!("{NUMBER_ARMS}f(1)"), "two");
	is!(&format!("{NUMBER_ARMS}f(5)"), "other");
	is!("n := 3; match n { 3: switch n * 2 { 6: \"six\"; _: \"never\" }; _: \"other\" }", "six");
}

#[test]
fn an_unbraced_inner_match_over_a_node() {
	is!(&format!("{NODE_ARMS}f(symbol(\"x\"))"), "other");
	is!(&format!("{NODE_ARMS}f(Key(symbol(\"a\"), Op.Else, symbol(\"b\")))"), "plain");
	is!(&format!("{NODE_ARMS}f(Key(Key(ø, Op.Then, symbol(\"y\")), Op.Else, symbol(\"b\")))"), "then");
}
