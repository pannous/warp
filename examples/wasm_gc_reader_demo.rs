use wasp::wasm_gc_emitter::WasmGcEmitter;
use wasp::wasm_gc_reader::read;
use wasp::node::Node;
use wasp::extensions::numbers::Number;

fn main() {
    println!("=== WASM GC Ergonomic Reading Demo ===\n");

    // Generate a WASM module with a Tag node
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();

    let html_node = Node::Tag(
        "html".to_string(),
        Box::new(Node::Empty),
        Box::new(Node::Text("Hello WASM!".to_string())),
    );
    emitter.emit_node_main(&html_node);

    let bytes = emitter.finish();
    std::fs::write("demo.wasm", &bytes).expect("Failed to write WASM");

    println!("✓ Generated demo.wasm ({} bytes)", bytes.len());

    // Ergonomic reading pattern from rasm:
    // root = read("test.wasm")
    let root = read("demo.wasm").expect("Failed to read WASM");

    println!("\n--- Ergonomic Field Access ---");

    // Access fields ergonomically
    let name = root.name().expect("Failed to get name");
    println!("  root.name() = \"{}\"", name);

    let kind = root.kind().expect("Failed to get kind");
    println!("  root.kind() = {} (NodeKind::Tag)", kind);

    // Pattern: is!(root.name, "html")
    assert_eq!(name, "html");
    println!("\n✓ Pattern works: is!(root.name, \"html\")");

    // Direct field access
    let tag: i32 = root.get("tag").expect("Failed to get tag");
    println!("  root.get(\"tag\") = {}", tag);

    let int_value: i64 = root.get("int_value").expect("Failed to get int_value");
    println!("  root.get(\"int_value\") = {}", int_value);

    // Test with different node types
    println!("\n--- Testing Number Node ---");

    let mut emitter2 = WasmGcEmitter::new();
    emitter2.emit();
    emitter2.emit_node_main(&Node::Number(Number::Int(42)));
    std::fs::write("number.wasm", &emitter2.finish()).expect("Failed to write");

    let num_node = read("number.wasm").expect("Failed to read number WASM");
    let value: i64 = num_node.get("int_value").expect("Failed to get value");
    println!("  Number node value: {}", value);
    assert_eq!(value, 42);

    // Test with Text node
    println!("\n--- Testing Text Node ---");

    let mut emitter3 = WasmGcEmitter::new();
    emitter3.emit();
    emitter3.emit_node_main(&Node::Text("Hello from WASM!".to_string()));
    std::fs::write("text.wasm", &emitter3.finish()).expect("Failed to write");

    let text_node = read("text.wasm").expect("Failed to read text WASM");
    let text = text_node.text().expect("Failed to get text");
    println!("  Text node content: \"{}\"", text);
    assert_eq!(text, "Hello from WASM!");

    println!("\n✓ All ergonomic patterns working!");
    println!("\nKey features:");
    println!("  • root = read(\"file.wasm\") - simple loading");
    println!("  • root.name() - ergonomic method access");
    println!("  • root.get(\"field\") - type-inferred field access");
    println!("  • Store management hidden via Rc<RefCell<Store>>");
    println!("  • Field access by name, not numeric indices");
    println!("  • String reading from linear memory automatic");

    // Clean up
    std::fs::remove_file("demo.wasm").ok();
    std::fs::remove_file("number.wasm").ok();
    std::fs::remove_file("text.wasm").ok();
}
