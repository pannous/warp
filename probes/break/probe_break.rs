use warp::wasm_emitter::eval;

#[test]
fn probe_break() {
	let cases = [("01_while_break", "3"), ("02_while_continue", "25"), ("03_for_range_break", "6"), ("04_for_range_continue", "20"),
		("05_for_list_continue", "8"), ("06_nested_break", "6"), ("07_next_alias", "12"), ("08_classic_for_continue", "13"), ("10_colon_if_break", "3"), ("12_while_do_end", "3"), ("11_python_indented_block_parser_issue", "3"), ("13_break_in_function", "12"), ("14_text_body_continue", "abd"), ("15_next_variable", "15"), ("16_while_true_break", "7"), ("17_nested_continue_outer_break", "23")];
	let mut failed = 0;
	for (name, expected) in cases {
		let path = format!("probes/break/{name}.warp");
		let got = std::panic::catch_unwind(|| eval(&path).to_string()).unwrap_or_else(|_| "PANIC".into());
		let ok = got == expected;
		if !ok { failed += 1 }
		println!("{} {name}: got {got} expected {expected}", if ok { "OK  " } else { "FAIL" });
	}
	let outside = std::panic::catch_unwind(|| eval("probes/break/09_break_outside_loop.warp").to_string());
	println!("09 break outside loop: {:?}", outside);
	assert_eq!(failed, 0);
}

