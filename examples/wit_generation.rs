use wasp::wit_emitter::{WitEmitter, node_to_wit_value};
use wasp::wasp_parser::WaspParser;
use wasp::node::Node;

fn main() {
    println!("=== WebAssembly Interface Types (WIT) Generation Demo ===\n");

    // Example 1: Generate WIT interface definition
    println!("Example 1: Generating WIT interface definition");
    let mut emitter = WitEmitter::new();
    emitter.emit_interface("wasp", "ast");

    let wit_output = emitter.get_output();
    println!("{}", wit_output);

    // Save to file
    if let Err(e) = emitter.emit_to_file("wasp-ast.wit") {
        eprintln!("Failed to write WIT file: {}", e);
    } else {
        println!("✓ Generated wasp-ast.wit\n");
    }

    // Example 2: Convert Node instances to WIT values
    println!("Example 2: Converting Node instances to WIT values\n");

    let simple_node = Node::int(42);
    println!("Simple number:");
    println!("  Rust:  {:?}", simple_node);
    println!("  WIT:   {}\n", node_to_wit_value(&simple_node));

    let text_node = Node::text("Hello, WASM!");
    println!("Text node:");
    println!("  Rust:  {:?}", text_node);
    println!("  WIT:   {}\n", node_to_wit_value(&text_node));

    let kv_node = Node::keys("name", "Alice");
    println!("Key-value node:");
    println!("  Rust:  {:?}", kv_node);
    println!("  WIT:   {}\n", node_to_wit_value(&kv_node));

    // Example 3: Parse WASP and convert to WIT
    println!("Example 3: Parse WASP to Node to WIT\n");

    let wasp = r#"config {
        server {
            host: "localhost"
            port: 8080
        }
        database {
            url: "postgresql://..."
        }
    }"#;

    println!("WASP input:");
    println!("{}\n", wasp);

    let node = WaspParser::parse(wasp).unwrap();
    let wit_value = node_to_wit_value(&node);

    println!("WIT representation:");
    println!("{}\n", wit_value);

    // Example 4: Node with metadata
    println!("Example 4: Node with metadata (position tracking)\n");

    let wasp_with_pos = "port: 8080";
    let node_with_meta = WaspParser::parse(wasp_with_pos).unwrap();

    println!("WASP: {}", wasp_with_pos);
    println!("WIT:  {}\n", node_to_wit_value(&node_with_meta));

    // Example 5: Complex nested structure
    println!("Example 5: Complex nested structure\n");

    let complex_node = Node::tag(
        "html",
        Node::list(vec![
            Node::keys("h1", "Welcome"),
            Node::keys("p", "Hello World"),
        ]),
    );

    println!("Complex structure:");
    println!("  Debug: {:?}", complex_node);
    println!("  WIT:   {}\n", node_to_wit_value(&complex_node));

    // Example 6: List of mixed types
    println!("Example 6: List of mixed types\n");

    let list_node = Node::list(vec![
        Node::int(1),
        Node::float(3.14),
        Node::text("mixed"),
        Node::symbol("types"),
    ]);

    println!("Mixed list:");
    println!("  WIT: {}\n", node_to_wit_value(&list_node));

    println!("=== Demo Complete ===");
    println!("\nThe WIT interface can be used to:");
    println!("  • Define component interfaces for WebAssembly Component Model");
    println!("  • Generate bindings for multiple languages (Rust, JS, Python, etc.)");
    println!("  • Enable interoperability between WASM components");
    println!("  • Serialize/deserialize Node AST across language boundaries");
}
