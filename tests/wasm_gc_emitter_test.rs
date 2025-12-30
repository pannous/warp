use wasp::wasm_gc_emitter::{WasmGcEmitter, NodeKind};
use std::fs::File;
use std::io::Write as IoWrite;

#[test]
fn test_wasm_module_generation() {
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let bytes = emitter.finish();

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
    let bytes = emitter.finish();
    let result = File::create(path).and_then(|mut f| f.write_all(&bytes));
    assert!(result.is_ok(), "Failed to write WASM file");

    // Verify file exists and has correct magic number
    let bytes = std::fs::read(path).unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
}

#[test]
fn test_node_tag_values() {
    // Ensure tags match the expected values
    assert_eq!(NodeKind::Empty as u32, 0);
    assert_eq!(NodeKind::Number as u32, 1);
    assert_eq!(NodeKind::Text as u32, 2);
    assert_eq!(NodeKind::Codepoint as u32, 3);
    assert_eq!(NodeKind::Symbol as u32, 4);
    assert_eq!(NodeKind::KeyValue as u32, 5);
    assert_eq!(NodeKind::Pair as u32, 6);
    assert_eq!(NodeKind::Tag as u32, 7);
    assert_eq!(NodeKind::Block as u32, 8);
    assert_eq!(NodeKind::List as u32, 9);
    assert_eq!(NodeKind::Data as u32, 10);
    assert_eq!(NodeKind::WithMeta as u32, 11);
}

#[test]
fn test_node_tag_equality() {
    let tag1 = NodeKind::Empty;
    let tag2 = NodeKind::Empty;
    assert_eq!(tag1, tag2);

    let tag3 = NodeKind::Number;
    assert_ne!(tag1, tag3);
}

#[test]
fn test_multiple_emitters() {
    // Test that we can create multiple emitters independently
    let mut emitter1 = WasmGcEmitter::new();
    let mut emitter2 = WasmGcEmitter::new();

    emitter1.emit();
    emitter2.emit();

    let bytes1 = emitter1.finish();
    let bytes2 = emitter2.finish();

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

    let bytes = emitter.finish();

    // WASM module should have:
    // - Magic number (4 bytes)
    // - Version (4 bytes)
    // - Sections (variable)
    assert!(bytes.len() > 8, "Module too small");

    // Check for section headers after magic+version
    let sections = &bytes[8..];
    assert!(!sections.is_empty(), "No sections found");
}
