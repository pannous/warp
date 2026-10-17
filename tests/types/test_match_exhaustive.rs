// card match-exhaustive (user decision match-exhaustive, notes/decisions.md): a match on a known enum or sum type
// without a catch-all that misses a case is the compile error "match on Coin misses Dime"; a subject of unknown type
// keeps the run-time error
use crate::common::fails_with;
use crate::is;

const SHAPE: &str = "enum Shape { Circle(r: float), Square(side: float), Dot }";
const COIN: &str = "enum Coin { Penny, Nickel, Dime }";

fn with(declaration: &str, code: &str) -> String {
	format!("{declaration}; {code}")
}

#[test]
fn a_match_missing_a_variant_fails_to_compile() {
	fails_with(
		&with(SHAPE, "area(s: Shape) := match s { Circle(r) => 3*r*r; Square(a) => a*a }; area(Square(2.0))"),
		"match on Shape misses Dot",
	);
	fails_with(
		&with(SHAPE, "area(s: Shape) := match s { Circle(r): 3*r*r }; area(Circle(1.0))"),
		"match on Shape misses Square, Dot",
	);
}

#[test]
fn a_match_missing_an_enum_case_fails_to_compile() {
	fails_with(
		&with(COIN, "cents(c: Coin) := match c { Coin.Penny: 1; Coin.Nickel: 5 }; cents(Coin.Penny)"),
		"match on Coin misses Dime",
	);
}

#[test]
fn complete_matches_and_catch_alls_compile() {
	is!(
		&with(
			SHAPE,
			"area(s: Shape) := match s { Circle(r) => 3*r*r; Square(a) => a*a; Shape::Dot => 0.0 }; area(Dot)"
		),
		0.0
	);
	is!(&with(SHAPE, "area(s: Shape) := match s { Circle(r): 3*r*r; _: 0.0 }; area(Dot)"), 0.0);
	is!(&with(COIN, "cents(c: Coin) := match c { Coin.Penny: 1; default: 10 }; cents(Coin.Dime)"), 10);
}

#[test]
fn a_subject_of_unknown_type_keeps_the_run_time_error() {
	fails_with(&with(SHAPE, "area(s) := match s { Circle(r) => 3*r*r; Square(a) => a*a }; area(Dot)"), "no case for s");
}
