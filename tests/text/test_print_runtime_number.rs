// A number whose kind shows only at run time (arithmetic on a cell's value, a message's payload) prints as its text
// (card print-value)
#[test]
#[cfg(feature = "native")]
fn a_runtime_number_prints() {
	let printed = crate::common::printed("c = cell_new(21)\nprint cell_get(c) * 2\nd = cell_new(1.5)\nprint cell_get(d) + 1");
	assert!(printed.starts_with("42\n2.5\n"), "printed {printed:?}");
}
