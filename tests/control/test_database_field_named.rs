// card orm-size-field: a stored row's field named like a builtin word (size, count, length) is the field, as of a
// declared list (field-named-size): `cups#1` reads through the table's generated cups·at, which keeps the row's class
use crate::is;
use warp::wasm_emitter::eval;

const CUPS: &str = "class Cup{size: text; count: int}\ncups: [Cup] = database.cups_field_named";

fn program(rest: &str) -> String {
	format!("{CUPS}\n{rest}")
}

#[test]
fn a_stored_row_reads_its_size_field() {
	eval(&program("cups.add(Cup(\"big\", 3))"));
	is!(&program("cups#1.size"), "big");
	is!(&program("cups#1.count"), 3);
	is!(&program("first = cups#1\nfirst.size"), "big");
	is!(&program("for cup in cups { cup.size }"), "big");
}

// a one-statement loop body over a table is one call, not the statements `print` and `cup.size`
#[cfg(feature = "native")]
#[test]
fn a_loop_over_a_table_prints_each_row() {
	let printed = crate::common::printed(&program("cups.add(Cup(\"small\", 1))\nfor cup in cups { print cup.size }"));
	assert!(printed.contains("small"), "{printed}");
}
