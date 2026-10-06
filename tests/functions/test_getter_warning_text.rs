// card g-_cHQ: the warning of an effectful getter quotes a short body (`t runs clock() at every read`) but names a
// long one only as its code, instead of spelling out the whole block
fn getter_warning(code: &str) -> String {
	warp::diagnostic::take_warnings();
	warp::wasm_emitter::eval(code);
	let warnings = warp::diagnostic::take_warnings();
	warnings.into_iter().map(|warning| warning.message).find(|message| message.contains("at every read")).unwrap_or_else(|| panic!("no getter warning for {code}"))
}

#[test]
fn a_long_getter_body_is_not_spelled_out_in_its_warning() {
	let message = getter_warning("pos = 0; peek := if pos < 3 then clock() + pos * 1000 else clock() - pos; peek");
	assert!(message.starts_with("peek runs its code at every read"), "{message}");
	assert!(!message.contains("if pos"), "{message}");
	assert!(getter_warning("t := clock(); t").contains("t runs clock() at every read; write t = clock() for one value"));
}
