// A cell's value (lowering/nonlocal_cells.rs, signal_values.rs) is a Node of any kind: `not` and conditions test it as
// a Node (0, false, ø are falsy), never by unboxing an Int
use crate::is;

#[test]
fn not_of_a_cell_value() {
	is!("c = cell_new(false); not cell_get(c)", true);
	is!("c = cell_new(1); not cell_get(c)", false);
	is!("c = cell_new(\"text\"); not cell_get(c)", false);
}

#[test]
fn a_cell_value_as_a_condition() {
	is!("c = cell_new(true); if cell_get(c) { 1 } else { 2 }", 1);
	is!("c = cell_new(0); if cell_get(c) { 1 } else { 2 }", 2);
}
