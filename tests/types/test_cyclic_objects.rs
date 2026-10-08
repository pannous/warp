// Objects may point to each other (a team's players and each player's team, card orm step 4): reading such a value
// back into a Node, a tree, marks where it meets an object it is already inside of (wasm_reader.rs and reader.js
// CYCLE_MARK) instead of overflowing the stack
use warp::wasm_emitter::eval;

#[test]
fn an_object_pointing_back_reads_with_a_cycle_mark() {
	let read = eval("class N{name: text; next: N?}\na = N(\"a\", ø)\nb = N(\"b\", a)\na.next = b\na").serialize();
	// natively the mark stands for a again (`N{name:'a' next:N{name:'b' next:…}}`); the browser's result is a copy of a's
	// first cell, so its mark comes one object later
	assert!(read.starts_with("N{name:'a' next:N{name:'b' next:") && read.trim_end_matches('}').ends_with('…'), "{read}");
}
