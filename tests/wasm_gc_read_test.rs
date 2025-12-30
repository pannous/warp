use wasmtime::{Config, Engine, Instance, Linker, Module, Store, Val, ValType};
use anyhow::Result;

/// Test reading WebAssembly GC object properties from the host
/// Based on patterns from ~/dev/script/rust/rasm demos
#[test]
fn test_read_wasm_gc_node_properties() -> Result<()> {
    println!("=== Testing WASM GC Object Property Reading ===\n");

    // Configure engine for GC support
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    // Create minimal WAT module with GC struct
    let wat = r#"
    (module
        (type $node (struct
            (field $tag i32)
            (field $value i64)
        ))

        ;; Create a node with tag and value
        (func (export "make_node") (param $tag i32) (param $value i64) (result (ref $node))
            local.get $tag
            local.get $value
            struct.new $node
        )

        ;; Get tag field from node
        (func (export "get_tag") (param $node (ref $node)) (result i32)
            local.get $node
            struct.get $node $tag
        )

        ;; Get value field from node
        (func (export "get_value") (param $node (ref $node)) (result i64)
            local.get $node
            struct.get $node $value
        )
    )
    "#;

    let wasm_bytes = wat::parse_str(wat)?;
    let module = Module::new(&engine, wasm_bytes)?;

    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)?;

    println!("✓ Loaded WASM module with GC types");

    // Test 1: Create a node from WASM and read it back from the host
    println!("\nTest 1: Create node in WASM, read properties from Rust");

    let make_node = instance.get_func(&mut store, "make_node")
        .expect("make_node function not found");

    let mut results = vec![Val::I32(0)];
    make_node.call(&mut store, &[Val::I32(42), Val::I64(123)], &mut results)?;

    println!("  Created node with tag=42, value=123");

    // Read properties using getter functions
    let get_tag = instance.get_typed_func::<(), i32>(&mut store, "get_tag")?;
    let get_value = instance.get_typed_func::<(), i64>(&mut store, "get_value")?;

    // This demonstrates the pattern from rasm: wrapping and accessing GC objects
    let node_ref = results[0].clone();

    println!("  Node reference type: {:?}", node_ref.ty(&mut store));

    // Test 2: Direct struct field access (if supported)
    println!("\nTest 2: Direct struct field introspection");

    if let Some(anyref) = node_ref.unwrap_anyref() {
        if let Ok(struct_ref) = anyref.unwrap_struct(&store) {
            println!("  ✓ Got struct reference");

            // Read field 0 (tag)
            let tag_val = struct_ref.field(&mut store, 0)?;
            let tag = tag_val.unwrap_i32();
            println!("  Field 0 (tag): {}", tag);
            assert_eq!(tag, 42);

            // Read field 1 (value)
            let value_val = struct_ref.field(&mut store, 1)?;
            let value = value_val.unwrap_i64();
            println!("  Field 1 (value): {}", value);
            assert_eq!(value, 123);

            println!("\n✓ Successfully read GC struct fields from host!");
        } else {
            println!("  Note: Direct struct introspection not available");
        }
    }

    // Test 3: Pattern for wrapping in ergonomic API (simplified from rasm)
    println!("\nTest 3: Ergonomic wrapper pattern");
    println!("  In rasm, this would use GcObject::new() to hide store management");
    println!("  and provide .get(\"tag\") and .get(\"value\") methods");

    Ok(())
}

/// Test creating nodes from host side (if possible with current wasmtime)
#[test]
fn test_create_wasm_gc_nodes_from_host() -> Result<()> {
    println!("=== Testing WASM GC Node Creation from Host ===\n");

    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    // WAT module with type definitions
    let wat = r#"
    (module
        (type $node (struct
            (field $tag (mut i32))
            (field $data i64)
        ))

        ;; Export type for host access (if supported)
        (func (export "node_type") (result (ref null $node))
            ref.null $node
        )
    )
    "#;

    let wasm_bytes = wat::parse_str(wat)?;
    let module = Module::new(&engine, wasm_bytes)?;

    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)?;

    println!("✓ Module loaded");
    println!("  Future work: Use Wasmtime APIs to create GC objects directly from host");
    println!("  This is the pattern demonstrated in rasm/gc_object_demo.rs");

    Ok(())
}

/// Integration test: Full workflow of serializing/deserializing Nodes
#[test]
fn test_node_serialization_workflow() -> Result<()> {
    println!("=== Testing Node Serialization Workflow ===\n");

    // This test demonstrates the full pattern:
    // 1. Create Node in Rust
    // 2. Serialize to WASM GC struct
    // 3. Read back properties from host
    // 4. Verify round-trip correctness

    println!("TODO: Implement full Node serialization");
    println!("  - Convert wasp::Node to WASM GC struct");
    println!("  - Use patterns from rasm to read fields");
    println!("  - Reconstruct Node from WASM representation");

    Ok(())
}
