// card cycle-mark-depth: the browser's reader marks a cyclic object where the native reader does, the first object
// met again (reader.js keeps each cell of a chain on its path, as wasm_reader.rs does)
use warp::wasm_emitter::eval;

#[test]
fn a_cycle_is_marked_at_the_object_met_again() {
	let read = eval("class N{name: text; next: N?}\na = N(\"a\", ø)\nb = N(\"b\", a)\na.next = b\na").serialize();
	assert_eq!(read, "N{name:'a' next:N{name:'b' next:…}}");
}
