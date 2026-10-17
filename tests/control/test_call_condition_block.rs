//! card if-exists: in a condition a braceless call's variable argument takes no block, the block is the body:
//! `if exists profile {…}` is `if exists(profile) {…}`, never exists(profile {…})

use crate::is;

#[test]
fn the_block_after_a_call_on_a_variable_is_the_body() {
	is!("profile = \"Cargo.toml\"; if exists profile {1} else {2}", 1);
	is!("profile = \"no such file\"; if exists profile {1} else {2}", 2);
	is!("f(x) := x > 1; y = 3; while f y {y -= 1}; y", 1);
}
