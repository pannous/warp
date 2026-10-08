//! One `use` naming several modules (card std-use): `use list, text` and `use list text` use each of them
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_use_with_commas_uses_each_module() {
	is!("use list, text; repeat(\"a\", 2)", "aa");
	is!("use list, text; unique([1, 2, 1])", parse("[1 2]"));
	is!("use list, math, text; gcd(4, 6)", 2);
}

#[test]
fn a_use_with_spaces_uses_each_module() {
	is!("use list text; repeat(\"a\", 2)", "aa");
	is!("use list text; unique([1, 2, 1])", parse("[1 2]"));
}

#[test]
fn a_use_with_a_version_stays_one_module() {
	is!("use list version 1.0.0; unique([1, 1])", parse("[1]"));
}
