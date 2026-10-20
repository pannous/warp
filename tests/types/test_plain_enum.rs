// card plain-enum (user decision enum-case B): a case of a plain enum is its own value of the enum's type, as a
// data-less variant of a tagged union is: `Coin.Dime` is Dime, `Dime is Coin`; its position from 0 is `Coin.Dime as
// int`, and an int becomes a case explicitly, `2 as Coin` or `Coin(2)`
use crate::common::fails_with;
use crate::is;
use warp::symbol;

const COIN: &str = "enum Coin { Penny, Nickel, Dime }";

fn with_coin(code: &str) -> String {
	format!("{COIN}; {code}")
}

#[test]
fn a_case_is_its_own_value() {
	is!(&with_coin("Coin.Dime"), symbol("Dime"));
	is!(&with_coin("Coin::Dime"), symbol("Dime"));
	is!(&with_coin("Dime"), symbol("Dime"));
	is!(&with_coin("c = Coin.Dime; c == Dime"), true);
}

#[test]
fn a_case_is_of_its_enum_type() {
	is!(&with_coin("Dime is Coin"), true);
	is!(&with_coin("c = Coin.Nickel; c is Coin"), true);
	is!(&with_coin("f(c: Coin) := c; f(Coin.Dime)"), symbol("Dime"));
	fails_with(&with_coin("f(c: Coin) := c; f(5)"), "needs a Coin");
}

#[test]
fn a_case_has_its_position() {
	is!(&with_coin("Coin.Dime as int"), 2);
	is!(&with_coin("Penny as int"), 0);
	is!(&with_coin("cents(c: Coin) := c as int; cents(Coin.Nickel)"), 1);
}

#[test]
fn an_int_becomes_a_case_explicitly() {
	is!(&with_coin("2 as Coin"), symbol("Dime"));
	is!(&with_coin("Coin(1)"), symbol("Nickel"));
	is!(&with_coin("n = 0; n as Coin"), symbol("Penny"));
	fails_with(&with_coin("9 as Coin"), "9 is no Coin case");
	is!(
		&with_coin(
			"cents(c: Coin) := match c { Penny: 1; Nickel: 5; Dime: 10 }; cents(Coin.Nickel) + cents(2 as Coin)"
		),
		15
	);
}

#[test]
fn other_languages_enums_are_plain_enums_too() {
	is!("enum Direction { case north, south }\nlet d = Direction.south\nd", symbol("south"));
	is!("enum class Color { RED, GREEN }\nColor.GREEN is Color", true);
	is!("enum color {red green blue}; color.blue as int", 2);
}

#[test]
fn a_case_converts_to_its_index_or_position() {
	// user: an enum's int is either an index (from 0, as C counts and `as int` gives) or a position (from 1, as `#` counts)
	is!(&with_coin("Coin.Dime as index"), 2);
	is!(&with_coin("Coin.Dime as position"), 3);
	is!(&with_coin("cents(c: Coin) := c as position; cents(Coin.Penny)"), 1);
	is!(&with_coin("xs = [10 20 30]; xs#(Coin.Dime as position)"), 30);
}

// card enum-case-after-statement: a bare `Coin.Dime` statement keeps the later cases and type tests working
#[test]
fn case_statement_then_type_test() {
	is!("enum Coin { Penny, Nickel, Dime }; Coin.Dime; Dime is Coin", true);
	is!("enum Coin { Penny, Nickel, Dime }; Coin.Nickel; Dime is Coin", true);
	is!("enum Coin { Penny, Nickel, Dime }; Coin.Nickel; 3 is int", true);
}

// a case held in a variable converts as the case itself does
#[test]
fn a_held_case_converts_as_the_case() {
	is!(&with_coin("c = Coin.Dime; c as int"), 2);
	is!(&with_coin("c = Dime; c as position"), 3);
}

// card rust-lane (r1-enum-case-names): a case named like a built-in type is the case wherever the enum declares it
#[test]
fn a_case_named_like_a_type_is_the_case() {
	let kind = "enum Kind { Empty, Int, Float, Text }";
	is!(&format!("{kind}; k = Kind.Float; k == Kind.Float"), true);
	is!(&format!("{kind}; f(k: Kind) := k == Float; f(Kind.Float)"), true);
	is!(&format!("{kind}; name(k: Kind) := match k {{ Kind.Int: 1; Float: 2; _: 3 }}; name(Float)"), 2);
	is!("enum Meaning { Function, Variable }; m = Meaning.Function; m == Meaning.Function", true);
}

// card rust-lane: a map of one entry named after an enum or class, `{Op: {}}`, stays a map (T:{…} is data, D4): a
// second entry joins the map, not the fields of an instance of Op
#[test]
fn an_entry_named_after_a_type_stays_a_map_entry() {
	let empty = "empty(): any := { t = {}; t }; m = empty()";
	is!(&format!("enum Op {{ Colon, Add }}; {empty}; m[\"Op\"] = empty(); m[\"Bracket\"] = 1; #m"), 2);
	is!(&format!("class Point {{ x: int }}; {empty}; m[\"Point\"] = empty(); m[\"y\"] = 2; #m"), 2);
}
