//! Card union-return (user 2026-10-12: `or` spells a union in every position, return types too): `as int or text`
//! and `f(x): int | text := …` check each returned value as a variable declared of the union is checked; a union
//! converts nothing, unlike `as int`. `as` is the canonical result type spelling, `->` works with a hint (return-as)
use crate::common::fails_with;
use crate::is;

const SHAPES: &str = "class Circle{r:int}; class Square{side:int}; class Point{x:int}; ";

#[test]
fn a_union_result_gives_back_each_part() {
	is!("def f(x:int) as int or text { \"a\" }; f(2)", "a");
	is!("def f(x:int) as int | text { x * 2 }; f(2)", 4);
	is!("f(x:int): int or text := \"a\"; f(2)", "a");
	is!("def f(x:int) as int or text { if x > 1 then return \"big\"; x }; f(2)", "big");
	is!("def f(x:int) as int or text { if x > 1 then return \"big\"; x }; f(1)", 1);
	is!(&format!("{SHAPES}def f(x:int) as Circle or Square {{ Square(x) }}; f(2).side"), 2);
}

#[test]
fn a_union_result_refuses_a_value_fitting_no_part() {
	fails_with(
		"def f(x:int) as int or text { 2.5 }; f(2)",
		"the result is declared int or text, cannot assign float 2.5",
	);
	fails_with("f(x:int): int | text := [x]; f(2)", "the result is declared int or text");
	fails_with(&format!("{SHAPES}def f(x:int) as Circle or Square {{ Point(x) }}; f(2)"), "Point is none of them");
	fails_with("def f(x:int) as int or zork { x }; f(2)", "zork is neither");
}

#[test]
fn a_union_mismatch_offers_the_wider_union() {
	fails_with("x: int or text = 2.5", "declare x:int or text or float");
}

#[test]
fn as_spells_a_union_result_with_or_without_a_block() {
	is!("def f(x:int) as int or text = \"a\"; f(2)", "a");
	is!("f(x:int) as int or text := x * 2; f(2)", 4);
	is!("def f(x:int) as int or text or float { 2.5 }; f(2)", 2.5);
	is!("f(x:int) as int | text | float := 2.5; f(2)", 2.5);
	fails_with("f(x:int) as int or text := 2.5; f(2)", "the result is declared int or text");
	is!("def f(x:int) as int { x }; f(2)", 2);
	is!("def f(x:int) -> int or text { \"a\" }; f(2)", "a"); // works, with the hint "prefer `def f(x) as int or text`"
}

#[test]
fn a_union_place_checks_a_value_known_only_at_run_time() {
	fails_with("f(x): int or text := x; f(2.5)", "is declared int or text");
	fails_with("g(v) := v; x: int or text = g(2.5); x", "is declared int or text");
	is!("f(x): int or text := x; f(\"a\")", "a");
	is!("g(v) := v; x: int or text = g(3); x", 3);
}
