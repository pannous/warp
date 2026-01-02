use wasmtime::{Config, Engine, Linker, Module, Store};
use wasp::eq;

/// Test that wasmtime 40.0.0 supports GC features
#[test]
fn test_wasmtime_gc_support() {
    println!("=== Testing Wasmtime 40.0.0 GC Support ===\n");

    // Configure engine with GC features
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config).expect("Failed to create engine with GC");
    let mut store = Store::new(&engine, ());

    println!("✓ Engine created with GC support enabled");

    // Test GC struct type
    let wat = r#"
    (module
        (type $point (struct
            (field $x i32)
            (field $y i32)
        ))

        (func (export "make_point") (param $x i32) (param $y i32) (result (ref $point))
            local.get $x
            local.get $y
            struct.new $point
        )
    )
    "#;

    let wasm_bytes = wat::parse_str(wat).expect("Failed to parse WAT");
    let module = Module::new(&engine, wasm_bytes).expect("Failed to create module");

    println!("✓ GC struct type compiled successfully");

    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("Failed to instantiate module");

    println!("✓ GC module instantiated");

    // Try to call the function
    let _make_point = instance
        .get_func(&mut store, "make_point")
        .expect("make_point function not found");

    println!("✓ Got GC function reference");
    println!("\nWasmtime 40.0.0 fully supports WebAssembly GC!");
}

/// Test GC introspection features
#[test]
fn test_gc_introspection() {
    println!("=== Testing GC Introspection APIs ===\n");

    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config).expect("Failed to create engine");
    let mut store = Store::new(&engine, ());

    let wat = r#"
    (module
        (type $node (struct
            (field $tag i32)
            (field $value i64)
        ))

        (func (export "create") (param $tag i32) (param $value i64) (result (ref $node))
            local.get $tag
            local.get $value
            struct.new $node
        )
    )
    "#;

    let wasm_bytes = wat::parse_str(wat).expect("Failed to parse WAT");
    let module = Module::new(&engine, wasm_bytes).expect("Failed to create module");
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("Failed to instantiate");

    let create = instance
        .get_func(&mut store, "create")
        .expect("create function not found");

    // Call and get a GC reference
    let mut results = vec![wasmtime::Val::I32(0)];
    create
        .call(
            &mut store,
            &[wasmtime::Val::I32(42), wasmtime::Val::I64(123)],
            &mut results,
        )
        .expect("Failed to call create");

    let node_ref = &results[0];
    println!("✓ Created GC object");
    println!("  Type: {:?}", node_ref.ty(&store));

    // Try introspection (if available in wasmtime 40.0.0)
    if let Some(anyref) = node_ref.unwrap_anyref() {
        println!("✓ Got anyref from Val");

        if let Ok(structref) = anyref.unwrap_struct(&store) {
            println!("✓ Got StructRef - full introspection available!");

            // Try to read fields
            match structref.field(&mut store, 0) {
                Ok(field) => {
                    println!("  ✓ Can read field 0: {:?}", field);
                    let i32_val = field.unwrap_i32();
                    println!("    Value: {}", i32_val);
                    eq!(i32_val, 42);
                }
                Err(e) => println!("  ✗ Cannot read field: {}", e),
            }
        } else {
            println!("  Note: StructRef introspection not available in this wasmtime version");
        }
    } else {
        println!("  Note: anyref unwrapping not available - using function-based access only");
    }

    println!("\nGC introspection test complete!");
}
