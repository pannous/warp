//! card matches-call: a standard module calling another module's word operator by its word, shell's
//! `matches(line, pattern)` of its own `use regex`; the program, without `use regex`, still doesn't see matches
use warp::wasm_emitter::eval;

#[test]
fn a_standard_module_calls_a_used_modules_operator_by_its_word() {
	let lines = eval("use shell; grep(\"^use regex\", \"lib/shell.warp\")");
	assert_eq!(lines.serialize().trim(), "[\"use regex\"]");
	assert!(matches!(eval("use shell; matches(\"a1\", \"[0-9]\")"), warp::Node::Error(_)));
}
