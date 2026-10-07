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

/// `use { memory, table } from "env"`: the module imports its memory and a function table instead of defining them;
/// warp gives a run of its own a fresh memory and table
#[cfg(feature = "native")]
#[test]
fn imported_memory_and_table() {
	let code = "use { memory, table } from \"env\"\nuse memory\nmemory.copy([7, 8], 100)\nmemory.slice(100, 102)";
	let module = warp::pipeline::compile(code).expect("compiles");
	let imports: Vec<(String, String)> = wasmparser::Parser::new(0).parse_all(&module.bytes).filter_map(Result::ok)
		.filter_map(|payload| match payload { wasmparser::Payload::ImportSection(section) => Some(section), _ => None })
		.flat_map(|section| section.into_imports().filter_map(Result::ok).map(|import| (import.module.to_string(), import.name.to_string())).collect::<Vec<_>>())
		.collect();
	assert!(imports.contains(&("env".to_string(), "memory".to_string())), "{imports:?}");
	assert!(imports.contains(&("env".to_string(), "table".to_string())), "{imports:?}");
}

#[test]
fn imported_memory_runs() {
	is!("use { memory, table } from \"env\"\nuse memory\nmemory.copy([7, 8], 100)\nmemory.slice(100, 102)", parse("[7 8]"));
}

#[test]
fn a_row_of_fields_among_lines() {
	is!("type Node: gc struct {\n    tag: i32\n    left: ref Node? right: ref Node?\n}\nn = Node { tag: 1 left: Node { tag: 2 left: None right: None } right: None }\nn.left.tag", 2);
}

/// the whole sample: imported memory, memory.copy and memory.slice, gc structs, an interface
#[cfg(feature = "native")]
#[test]
fn wasm_interop_sample_runs() {
	let sample = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/wasm_interop.wasp")).unwrap();
	let printed = crate::common::printed(&sample);
	assert!(printed.contains("\"wasm ✓\" 50]"), "{printed}");
}
