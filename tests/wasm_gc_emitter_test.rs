use wasp::wasm_gc_emitter::WasmGcEmitter;
use wasp::run::wasmtime_runner::run;
use wasp::{eq, write_wasm};


#[test]
fn test_wasm_roundtrip() {
    use wasp::wasp_parser::WaspParser;

    // Parse WASP input
    let input = "html{test=1}";
    let node = WaspParser::parse(input).expect("Failed to parse WASP");
    println!("Parsed node: {:?}", node);

    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    emitter.emit_node_main(&node); // Emit a main() function that returns the node

    let path = "out/test_wasm_roundtrip.wasm";
    let bytes = emitter.finish();
    assert!(write_wasm(path, &bytes), "Failed to write WASM file");
    println!("✓ Generated {} ({} bytes)", path, bytes.len());



    // let root : GcObject = run_wasm_gc_object(path).expect("Failed to read back WASM");
    // if root. {  }
    let root = run(path); // reconstruct Node from WASM via GcObject
    println!("✓ Read back root node from WASM: {:?}", root);
    eq!(root, node);
}
