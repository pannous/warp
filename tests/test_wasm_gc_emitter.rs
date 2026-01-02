use wasp::node::Node;
use wasp::node::Node::*;
use wasp::run::wasmtime_runner::run;
use wasp::wasm_gc_emitter::{eval, WasmGcEmitter};
use wasp::StringExtensions;
use wasp::{eq, is, write_wasm};

fn normalize_blocks(node: &Node) -> Node {
    let node = node.unwrap_meta();
    match node {
        Tag {
            title,
            params,
            body,
        } => Tag {
            title: title.clone(),
            params: Box::new(normalize_blocks(params)),
            body: Box::new(normalize_blocks(body)),
        },
        Block(items, _, _) if items.len() == 1 => normalize_blocks(&items[0]),
        List(items) if items.len() == 1 => normalize_blocks(&items[0]),
        Key(k, v) => Key(k.clone(), Box::new(normalize_blocks(v))),
        Pair(left, right) => Pair(
            Box::new(normalize_blocks(left)),
            Box::new(normalize_blocks(right)),
        ),
        _ => node.clone(),
    }
}
use wasp::type_kinds::NodeKind;
use wasp::Number::Int;

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
    // Normalize original: unwrap single-item blocks like WASM does
    let normalized = normalize_blocks(&node);
    eq!(root, normalized);
}

#[test]
fn test_wasm_roundtrip_via_is() {
    // Parser treats {test=1} as body containing Key, not params
    let x = Key("test".s(), Box::new(Number(Int(1))));
    let _ok: Node = eval("html{test=1}");
    // After single-item block unwrapping, body becomes just the Key
    is!(
        "html{test=1}",
        Tag {
            title: "html".s(),
            params: Box::new(Empty),
            body: Box::new(x),
        }
    );
}

#[test]
fn test_emit_gc_types() {
    // let mut emitter = WasmGcEmitter::new();
    // emitter.emit();
    // Verify unified type indices are valid (can be 0 for first type)
    // private
    // eq!(emitter.node_base_type, 0); // First type
    // eq!(emitter.node_array_type, 1); // Second type
    // assert!(emitter.next_type_idx > 1); // We defined at least 2 types
}

#[test]
fn test_generate_wasm() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    let bytes = emitter.finish();

    // Should have WASM magic number
    eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    // Should have version 1
    eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_node_kind_enum_abi() {
    // ensure enum values match expected ABI
    eq!(NodeKind::Empty as u32, 0);
    eq!(NodeKind::Number as u32, 1);
    eq!(NodeKind::Codepoint as u32, 3);
    eq!(NodeKind::Symbol as u32, 4);
    eq!(NodeKind::Key as u32, 5);
    eq!(NodeKind::Pair as u32, 6);
    eq!(NodeKind::Tag as u32, 7);
    eq!(NodeKind::Block as u32, 8);
    eq!(NodeKind::List as u32, 9);
    eq!(NodeKind::Data as u32, 10);
}
