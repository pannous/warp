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
fn test_wasm_roundtrip() {
    use wasp::wasp_parser::WaspParser;
    use wasmparser::{Parser, Payload};

    // Parse WASP input
    let input = "html{test=1}";
    let node = WaspParser::parse(input).expect("Failed to parse WASP");
    println!("Parsed node: {:?}", node);

    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    emitter.emit_node_main(&node); // Emit a main() function that returns the node

    let path = "test_wasm_roundtrip.wasm";
    let bytes = emitter.finish();
    let result = File::create(path).and_then(|mut f| f.write_all(&bytes));
    assert!(result.is_ok(), "Failed to write WASM file");
    println!("✓ Generated {} ({} bytes)", path, bytes.len());
    println!("Now verify with: wasm2wat --no-check --enable-all --ignore-custom-section-errors {}", path);

    // Use wasmparser to validate the WASM file
    let bytes = std::fs::read(path).unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);

    // Validate WASM structure using wasmparser with full GC validation
    use wasmparser::{Validator, WasmFeatures};

    let mut features = WasmFeatures::default();
    features.set(WasmFeatures::REFERENCE_TYPES, true);
    features.set(WasmFeatures::GC, true);

    let mut validator = Validator::new_with_features(features);
    match validator.validate_all(&bytes) {
        Ok(_) => println!("✓ WASM validation with GC features passed"),
        Err(e) => panic!("WASM validation failed: {}", e),
    }

    let parser = Parser::new(0);
    let mut has_type_section = false;
    let mut has_function_section = false;
    let mut has_export_section = false;
    let mut has_code_section = false;

    for payload in parser.parse_all(&bytes) {
        match payload.expect("Invalid WASM payload") {
            Payload::Version { .. } => {},
            Payload::TypeSection(types) => {
                has_type_section = true;
                println!("Found type section with {} types", types.count());
            },
            Payload::FunctionSection(funcs) => {
                has_function_section = true;
                println!("Found function section with {} functions", funcs.count());
            },
            Payload::ExportSection(exports) => {
                has_export_section = true;
                let export_names: Vec<_> = exports
                    .into_iter()
                    .filter_map(|e| e.ok().map(|ex| ex.name.to_string()))
                    .collect();
                println!("Exports: {:?}", export_names);
                assert!(export_names.contains(&"main".to_string()), "Should export main function");
                assert!(export_names.contains(&"get_tag".to_string()), "Should export get_tag");
                assert!(export_names.contains(&"get_int_value".to_string()), "Should export get_int_value");
            },
            Payload::CodeSectionEntry(_body) => {
                has_code_section = true;
            },
            Payload::CustomSection(custom) => {
                if custom.name() == "name" {
                    println!("✓ Found name custom section");
                }
            },
            _ => {}
        }
    }

    assert!(has_type_section, "Should have type section");
    assert!(has_function_section, "Should have function section");
    assert!(has_export_section, "Should have export section");
    assert!(has_code_section, "Should have code section");
    println!("✓ WASM validation passed");
}
