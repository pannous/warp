use warp::wasm_emitter::eval;

#[test]
fn probe_indent() {
	let cases = [("01_while_tabs", "3"), ("02_while_spaces", "3"), ("03_if_spaces", "11"), ("04_def_spaces", "7"), ("05_for_spaces", "36"),
		("06_nested_spaces", "4202"), ("07_while_do_end", "3"), ("08_while_do_end_lines", "3"), ("09_def_tabs", "7"), ("11_if_then_else_end", "6"), ("10_single_end_ambiguous", "ambiguous"), ("12_do_end_two_statements", "3")];
	let mut failed = 0;
	for (name, expected) in cases {
		let path = format!("probes/indent/{name}.wasp");
		let code = std::fs::read_to_string(&path).unwrap();
		let parsed = std::panic::catch_unwind(|| warp::parse(&code).serialize()).unwrap_or_else(|_| "PANIC".into());
		let got = std::panic::catch_unwind(|| eval(&path).to_string()).unwrap_or_else(|_| "PANIC".into());
		let ok = got == expected || (expected == "ambiguous" && got.contains("ambiguous `end`"));
		if !ok { failed += 1 }
		println!("{} {name}: got {got} expected {expected}\n     parsed: {}", if ok { "OK  " } else { "FAIL" }, parsed.replace('\n', "⏎"));
	}
	assert_eq!(failed, 0);
}
