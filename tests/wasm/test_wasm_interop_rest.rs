// What samples/wasm_interop.wasp kept as comments (card wasm-interop-rest): optional fields on one line, linear memory
// read and written by its bytes
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn optional_fields_on_one_line() {
	is!("type P: { a: int? b: int? }; P{a: 1, b: 2}.b", 2);
	is!("type Node: gc struct { tag: i32 left: ref Node? right: ref Node? }; n = Node{tag: 1 left: Node{tag: 2 left: None right: None} right: None}; n.left.tag", 2);
	is!("ok = 1; ok? 7: 8", 7); // outside a type a `?` before `name:` stays the ternary
}

#[test]
fn linear_memory_by_its_bytes() {
	is!("use memory; memory.copy([104, 105], 60000); memory.slice(60000, 60002)", parse("[104 105]"));
	is!("use memory; use text; memory.copy(to_utf8(\"wasm ✓\"), 60000); from_utf8(memory.slice(60000, 60008))", "wasm ✓");
}
