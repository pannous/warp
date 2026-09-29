// An unspaced number next to a symbol or `(` multiplies: `2x` is 2*x (wiki/number.md)
use warp::*;

mod common;

fn is_product(node: &Node) -> bool {
	matches!(node, Node::Key(_, Op::Mul, _))
}

#[test]
fn test_number_times_symbol() {
	is!("x=3;2x", 6);
	is!("2π", 2.0 * std::f64::consts::PI);
	is!("x=4;0.5x", 2.0);
	is!("x=3;2x+1", 7);
}

#[test]
fn test_number_times_parenthesis() {
	is!("3(4)", 12);
	is!("x=2;3(x+1)", 9);
}

#[test]
fn test_julia_grouping_division() {
	is!("x=2;8/2x", 2);
	is!("x=2;6/3x", 1);
}

#[test]
fn test_postfix_binds_to_the_symbol() {
	is!("x=3;2x²", 18);
	is!("x=2;3x³", 24);
}

#[test]
fn test_ordinals_are_not_products() {
	for ordinal in ["1st", "2nd", "3rd", "4th"] {
		assert!(!is_product(&parse(ordinal)), "{ordinal}");
	}
}

#[test]
fn test_undefined_symbol_is_the_normal_error() {
	common::fails_with("2q", "q");
}

#[test]
fn test_data_mode_never_multiplies() {
	assert!(!is_product(&parse_data("3x")));
	assert!(!is_product(&parse_data("size: 3px").drop_meta()));
}

#[test]
fn test_spaced_number_and_symbol_is_still_a_list() {
	is!("x=3;[2 x]#2", 3);
}
