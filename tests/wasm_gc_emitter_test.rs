use wasp::node::Node;
use wasp::node::Node::*;
use wasp::run::wasmtime_runner::run;
use wasp::wasm_gc_emitter::{NodeKind, WasmGcEmitter,eval};
use wasp::{eq, is, write_wasm};

#[test]
fn test_wasm_roundtrip() {
    // same as eval() but shows explicit parsing
    use wasp::wasp_parser::WaspParser;

    // Parse WASP input
    let input = "html{test=1}";
    let node = WaspParser::parse(input);
    println!("Parsed node: {:?}", node);

    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    emitter.emit_node_main(&node); // Emit a main() function that returns the node

    let path = "out/test_wasm_roundtrip.wasm";
    let bytes = emitter.finish();
    assert!(write_wasm(path, &bytes), "Failed to write WASM file");
    println!("✓ Generated {} ({} bytes)", path, bytes.len());

    // let root : GcObject = run_wasm_gc_object(path).expect("Failed to read back WASM");
    let root = run(path); // reconstruct Node from WASM via GcObject
    println!("✓ Read back root node from WASM: {:?}", root);
    eq!(root, node);
}

#[test]
fn test_wasm_roundtrip_via_is() {
    let x = Tag {
        title: "html".to_string(),
        params: Box::new(KeyValue(
            "test".to_string(),
            Box::new(Number(wasp::extensions::numbers::Number::Int(1))),
        )),
        body: Box::new(Node::Empty),
    };
    let ok:Node = eval("html{test=1}");
    is!("html{test=1}", x);
}


#[test]
fn test_emit_gc_types() {
    // let mut emitter = WasmGcEmitter::new();
    // emitter.emit();
    // Verify unified type indices are valid (can be 0 for first type)
    // private
    // assert_eq!(emitter.node_base_type, 0); // First type
    // assert_eq!(emitter.node_array_type, 1); // Second type
    // assert!(emitter.next_type_idx > 1); // We defined at least 2 types
}

#[test]
fn test_generate_wasm() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    let bytes = emitter.finish();

    // Should have WASM magic number
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    // Should have version 1
    assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_node_kind_enum_abi() { // ensure enum values match expected ABI
    assert_eq!(NodeKind::Empty as u32, 0);
    assert_eq!(NodeKind::Number as u32, 1);
    assert_eq!(NodeKind::Codepoint as u32, 3);
    assert_eq!(NodeKind::Symbol as u32, 4);
    assert_eq!(NodeKind::KeyValue as u32, 5);
    assert_eq!(NodeKind::Pair as u32, 6);
    assert_eq!(NodeKind::Tag as u32, 7);
    assert_eq!(NodeKind::Block as u32, 8);
    assert_eq!(NodeKind::List as u32, 9);
    assert_eq!(NodeKind::Data as u32, 10);
}
