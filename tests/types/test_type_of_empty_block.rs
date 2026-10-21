//! card type-class: an empty block `{}` is ø wherever it stands, as `x = {}` holds it: type({}) is empty, not int

use crate::is;

#[test]
fn the_type_of_an_empty_block_is_empty() {
	is!("type({})", "empty");
	is!("type {}", "empty");
	is!("x = {}; type(x)", "empty");
}
