use wasp::wasm_gc_emitter::{WasmGcEmitter, NodeTag};

#[test]
fn test_wasm_module_generation() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let bytes = emitter.build();

    // Check WASM magic number: 0x00 0x61 0x73 0x6D
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);

    // Check version: 0x01 0x00 0x00 0x00
    assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_wasm_file_output() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let path = "/tmp/test-wasm-gc-nodes.wasm";
    let result = emitter.emit_to_file(path);
    assert!(result.is_ok(), "Failed to write WASM file");

    // Verify file exists and has correct magic number
    let bytes = std::fs::read(path).unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
}

#[test]
fn test_node_tag_values() {
    // Ensure tags are sequential for efficient matching
    assert_eq!(NodeTag::Empty as u32, 0);
    assert_eq!(NodeTag::Number as u32, 1);
    assert_eq!(NodeTag::Text as u32, 2);
    assert_eq!(NodeTag::Symbol as u32, 3);
    assert_eq!(NodeTag::KeyValue as u32, 4);
    assert_eq!(NodeTag::Pair as u32, 5);
    assert_eq!(NodeTag::Tag as u32, 6);
    assert_eq!(NodeTag::Block as u32, 7);
    assert_eq!(NodeTag::List as u32, 8);
    assert_eq!(NodeTag::Data as u32, 9);
    assert_eq!(NodeTag::WithMeta as u32, 10);
}

#[test]
fn test_node_tag_equality() {
    let tag1 = NodeTag::Empty;
    let tag2 = NodeTag::Empty;
    assert_eq!(tag1, tag2);

    let tag3 = NodeTag::Number;
    assert_ne!(tag1, tag3);
}

#[test]
fn test_multiple_emitters() {
    // Test that we can create multiple emitters independently
    let mut emitter1 = WasmGcEmitter::new();
    let mut emitter2 = WasmGcEmitter::new();

    emitter1.emit();
    emitter2.emit();

    let bytes1 = emitter1.build();
    let bytes2 = emitter2.build();

    // Both should produce valid WASM
    assert_eq!(&bytes1[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    assert_eq!(&bytes2[0..4], &[0x00, 0x61, 0x73, 0x6D]);

    // Should be identical
    assert_eq!(bytes1, bytes2);
}

#[test]
fn test_wasm_module_structure() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let bytes = emitter.build();

    // WASM module should have:
    // - Magic number (4 bytes)
    // - Version (4 bytes)
    // - Sections (variable)
    assert!(bytes.len() > 8, "Module too small");

    // Check for section headers after magic+version
    let sections = &bytes[8..];
    assert!(!sections.is_empty(), "No sections found");
}

#[test]
fn test_default_constructor() {
    let mut emitter = WasmGcEmitter::default();
    emitter.emit();

    let bytes = emitter.build();
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
}
