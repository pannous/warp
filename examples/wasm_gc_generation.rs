use wasp::wasm_gc_emitter::{WasmGcEmitter, NodeKind};
use std::fs::File;
use std::io::Write;

fn main() {
    println!("=== WebAssembly GC Bytecode Generation Demo ===\n");

    println!("Example 1: Generate WASM module with Node functions\n");

    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let bytes = emitter.finish();
    println!("Generated WASM module: {} bytes", bytes.len());
    println!("Magic number: {:02X} {:02X} {:02X} {:02X}",
             bytes[0], bytes[1], bytes[2], bytes[3]);

    // Save to file
    let mut emitter2 = WasmGcEmitter::new();
    emitter2.emit();
    let bytes2 = emitter2.finish();
    if let Err(e) = File::create("nodes.wasm").and_then(|mut f| f.write_all(&bytes2)) {
        eprintln!("Failed to write WASM file: {}", e);
    } else {
        println!("✓ Generated nodes.wasm\n");
    }

    println!("Example 2: Node variant tags\n");
    println!("Node types are represented by numeric tags:");
    println!("  Empty    = {}", NodeKind::Empty as u32);
    println!("  Number   = {}", NodeKind::Number as u32);
    println!("  Text     = {}", NodeKind::Text as u32);
    println!("  Codepoint = {}", NodeKind::Codepoint as u32);
    println!("  Symbol   = {}", NodeKind::Symbol as u32);
    println!("  KeyValue = {}", NodeKind::KeyValue as u32);
    println!("  Pair     = {}", NodeKind::Pair as u32);
    println!("  Tag      = {}", NodeKind::Tag as u32);
    println!("  Block    = {}", NodeKind::Block as u32);
    println!("  List     = {}", NodeKind::List as u32);
    println!("  Data     = {}", NodeKind::Data as u32);
    println!("  WithMeta = {}", NodeKind::WithMeta as u32);

    println!("\n=== Demo Complete ===");
    println!("\nThe generated WASM module exports:");
    println!("  • make_empty() -> (ref empty)");
    println!("  • make_int(i64) -> (ref number)");
    println!("  • make_float(f64) -> (ref number)");
    println!("  • make_codepoint(i32) -> (ref codepoint)");
    println!("  • get_node_kind(ref node) -> i32");
    println!("\nThese functions use WebAssembly GC types to");
    println!("construct and manipulate Node AST structures.");
}
