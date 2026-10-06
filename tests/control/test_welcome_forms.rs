//! Forms from other languages, lowered to wasp's own (notes/welcoming.md): `match` cases written with `=>`, `_` the
//! default case (Rust, Scala), and `loop { … }`, the endless loop left by `break` (Rust)

use crate::is;

#[test]
fn match_cases_may_be_written_with_fat_arrows() {
	is!("x=2; match x { 1 => \"one\"; 2 => \"two\"; _ => \"other\" }", "two");
	is!("match 7 { 1 => 10; _ => 0 }", 0);
	is!("switch 1 { 1 => 10; 2 => 20 }", 10);
}

#[test]
fn loop_runs_until_break() {
	is!("i=0; loop { i = i + 1; if i > 4 { break } }; i", 5);
}
