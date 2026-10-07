// card empty-parens-getter: `w() := random()` and `def w() := random()` say by their parentheses that they run at every
// call: no "runs … at every read" warning, which only the bare getter `w := random()` gets; both still read the
// current value of a variable changed after them (P71)
use crate::is;

fn every_read_warnings(code: &str) -> Vec<String> {
	warp::diagnostic::take_warnings();
	warp::wasm_emitter::eval(code);
	warp::diagnostic::take_warnings().into_iter().map(|warning| warning.message).filter(|message| message.contains("at every read")).collect()
}

#[test]
fn written_parentheses_need_no_every_read_warning() {
	assert!(every_read_warnings("w() := clock(); w").is_empty());
	assert!(every_read_warnings("def w() := clock(); w").is_empty());
	assert!(!every_read_warnings("w := clock(); w").is_empty());
}

#[test]
fn a_getter_with_parentheses_still_reads_the_current_value() {
	is!("y=1; w() := y*2; y = 5; w", 10);
	is!("y=1; def w() := y*2; y = 5; w", 10);
}
