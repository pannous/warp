// Overloading by parameter types: each definition is a variant, a call takes the one its arguments fit best
use crate::common::fails_with;
use crate::is;

const COMBINE: &str = "combine(a:float, b:float) := a+b; combine(a:int, b:int) := a*b; ";

#[test]
fn a_call_takes_the_variant_its_arguments_fit() {
	is!(&format!("{COMBINE}combine(2, 3)"), 6);
	is!(&format!("{COMBINE}combine(1.5, 2.5)"), 4.0);
	is!(&format!("{COMBINE}x=2; y=3; combine(x, y)"), 6);
	is!("f(a:int) := 1; f(a:text) := 2; f(\"x\") + f(5)", 3);
}

#[test]
fn an_int_widens_to_a_float_variant() {
	is!("combine(a:float, b:float) := a+b; combine(a:string, b:string) := a; combine(2, 3)", 5.0);
}

#[test]
fn an_argument_of_unknown_type_is_an_error() {
	fails_with(&format!("{COMBINE}def g(p){{ combine(p, p) }}; g(2)"), "fits no single variant");
}

const COMBINE_WITH: &str = "combine number with number = number#1 + number#2; combine int with int = $0 * $1; ";

// Card paren-overload: a phrase call in parentheses dispatches as the bare call
#[test]
#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
fn a_phrase_call_in_parentheses_takes_its_variant() {
	is!(&format!("{COMBINE_WITH}(combine 1.1 with 2.2)"), 3.3);
	is!(&format!("{COMBINE_WITH}x = (combine 2 with 3); x"), 6);
	is!(&format!("{COMBINE_WITH}(combine 2 with 3) + 1"), 7);
}

// Card similar-vs: ≈ and ~ end a phrase call's arguments as == does
#[test]
#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
fn a_similarity_compares_the_phrase_calls_value() {
	is!(&format!("{COMBINE_WITH}combine 1.1 with 2.2 ≈ 3.3"), true);
	is!(&format!("{COMBINE_WITH}check combine 1.1 with 2.2 ≈ 3.3; 7"), 7);
	is!(&format!("{COMBINE_WITH}combine 2 with 3 ~ 6"), true);
}

// card untyped-variant-fallback: an untyped variant takes only what no typed variant fits
#[test]
fn an_untyped_variant_is_the_fallback() {
	let variants = "foo(x:int) := x+x; foo(x:text) := x.length; foo(x) := 1; ";
	is!(&format!("{variants}foo(\"abc\")"), 3);
	is!(&format!("{variants}foo(2)"), 4);
	is!(&format!("{variants}foo(2.5)"), 1);
	is!("foo(x:int) := x+x; foo(x: any) := 1; foo('c') + foo(2)", 5);
}

// card of-any-variant: `foo of any` is the fallback variant, as wiki/polymorphism.md shows
#[test]
fn of_any_is_the_fallback_variant() {
	is!("foo of int = it+it; foo of text = it.length; foo of any = 1; foo(\"abc\") + foo(2) + foo(2.5)", 8);
	is!("foo of any = 1; foo(3)", 1);
}

// card text-list: a list parameter overloads a text one instead of the last definition silently winning
#[test]
fn a_list_parameter_overloads_a_text_parameter() {
	let variants = "f(c: text, x) := c.length + x; f(b: list, x) := b.count * x; ";
	is!(&format!("{variants}f(\"abc\", 1)"), 4);
	is!(&format!("{variants}f([1, 2], 10)"), 20);
	is!(&format!("{variants}xs = [1, 2, 3]; f(xs, 2) + f(\"ab\", 0)"), 8);
}
