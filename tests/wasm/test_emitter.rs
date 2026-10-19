use crate::is;
use warp::*;

#[test]
fn test_compact_empty() {
	parses_to!("ø", Empty);
	parses_to!("null", Empty);
	parses_to!("no", Empty);
	parses_to!("none", Empty);
	parses_to!("nix", Empty);
	parses_to!("empty", Empty);
	is!("ø", Empty);
	is!("null", Empty);
}

#[test]
fn test_compact_float() {
	is!("3.11", 3.11);
}
