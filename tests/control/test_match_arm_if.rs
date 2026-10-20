//! A match arm giving an inline if, `pattern: if c then a else b`, gives the whole if (cards match-arm, arm-inline-if)
use crate::is;

#[test]
fn an_arm_gives_its_inline_if() {
	is!("f(x) := match x { 1: if x > 0 then \"one\" else \"none\"; _: \"other\" }; f(1)", "one");
	is!("match 2 { 2: if 2 < 1 then 1 else 5; _: 0 }", 5);
	is!("g(n) := match n { Symbol(word): if word == \"a\" then 1 else 2; _: 0 }; 100 * g(symbol(\"a\")) + 10 * g(symbol(\"b\")) + g(3)", 120);
}
