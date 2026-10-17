//! A quoted data list keeps its written brackets and separator when printed at run time.

#[test]
#[cfg(feature = "native")]
fn printed_data_lists_keep_their_shape() {
	let program = include_str!("../../probes/print_list_shapes.warp");
	assert_eq!(crate::common::printed(program), "(a; b)\n[a, b]\n{a; b}\n");
}
