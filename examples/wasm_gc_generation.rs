use wasp::wasm_gc_emitter::{WasmGcEmitter, NodeTag};

fn main() {
    println!("=== WebAssembly GC Bytecode Generation Demo ===\n");

    println!("Example 1: Generate WASM module with Node functions\n");

    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let bytes = emitter.build();
    println!("Generated WASM module: {} bytes", bytes.len());
    println!("Magic number: {:02X} {:02X} {:02X} {:02X}",
             bytes[0], bytes[1], bytes[2], bytes[3]);

    // Save to file
    let mut emitter2 = WasmGcEmitter::new();
    emitter2.emit();
    if let Err(e) = emitter2.emit_to_file("nodes.wasm") {
        eprintln!("Failed to write WASM file: {}", e);
    } else {
        println!("✓ Generated nodes.wasm\n");
    }

    println!("Example 2: Node variant tags\n");
    println!("Node types are represented by numeric tags:");
    println!("  Empty    = {}", NodeTag::Empty as u32);
    println!("  Number   = {}", NodeTag::Number as u32);
    println!("  Text     = {}", NodeTag::Text as u32);
    println!("  Symbol   = {}", NodeTag::Symbol as u32);
    println!("  KeyValue = {}", NodeTag::KeyValue as u32);
    println!("  Pair     = {}", NodeTag::Pair as u32);
    println!("  Tag      = {}", NodeTag::Tag as u32);
    println!("  Block    = {}", NodeTag::Block as u32);
    println!("  List     = {}", NodeTag::List as u32);
    println!("  Data     = {}", NodeTag::Data as u32);
    println!("  WithMeta = {}", NodeTag::WithMeta as u32);

    println!("\n=== Demo Complete ===");
    println!("\nThe generated WASM module exports:");
    println!("  • make_empty() -> i32");
    println!("  • make_int(i64) -> i32");
    println!("  • make_float(f64) -> i32");
    println!("\nThese functions return node type tags that can be used");
    println!("to construct and manipulate Node AST in WebAssembly.");
}
