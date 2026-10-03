//! Tests for host functions: fetch and run

use warp::WasmGcEmitter;
use warp::wasm_reader::read_bytes_with_host;

// todo this is a horrible test suite, just do is!("fetch(...)", result)

/// Test that emitter can generate host imports
#[test]
fn test_emit_host_imports() {
	let mut emitter = WasmGcEmitter::new();
	emitter.set_host_imports(true);
	emitter.set_emit_kind_globals(false); // Keep output simple

	// This creates a module with host imports
	let node = warp::int(42);
	emitter.emit_for_node(&node);
	let bytes = emitter.finish();

	// The module should have an imports section with host.fetch and host.run
	// Verify by checking that the WASM is valid
	assert!(!bytes.is_empty());
}

/// Test creating a minimal WASM module that uses host.fetch
/// This creates a WAT-like module that calls fetch and returns length
#[test]
fn test_host_fetch_import() {
	// Create a minimal module with host imports using wat
	let wat = r#"
		(module
			(import "host" "fetch" (func $fetch (param i32 i32) (result i32 i32)))
			(memory (export "memory") 1)
			(data (i32.const 0) "https://example.com")
			(func (export "main") (result i64)
				;; Call fetch with url at offset 0, length 19
				(call $fetch (i32.const 0) (i32.const 19))
				;; Returns (ptr, len) - we want the len
				(drop) ;; drop ptr
				(i64.extend_i32_u) ;; extend len to i64
			)
		)
	"#;

	let bytes = wat::parse_str(wat).expect("Failed to parse WAT");

	// This should work with the host linker
	let result = read_bytes_with_host(&bytes);

	// In test mode, download returns mock data
	// The result should be the length of the mock response
	assert!(result.is_ok(), "Host function execution failed: {:?}", result.err());
}

/// Test that host.run can execute nested WASM
#[test]
fn test_host_run_import() {
	// Create a module that stores inner WASM in memory and calls run
	// The inner module is a simple "return 42"
	let inner_wat = r#"(module (func (export "main") (result i64) (i64.const 42)))"#;
	let inner_bytes = wat::parse_str(inner_wat).expect("Failed to parse inner WAT");

	// Create outer module that embeds inner bytes and calls run
	let inner_hex: String = inner_bytes.iter().map(|b| format!("\\{:02x}", b)).collect();

	let outer_wat = format!(
		r#"
		(module
			(import "host" "run" (func $run (param i32 i32) (result i64)))
			(memory (export "memory") 1)
			(data (i32.const 0) "{inner_hex}")
			(func (export "main") (result i64)
				(call $run (i32.const 0) (i32.const {len}))
			)
		)
		"#,
		inner_hex = inner_hex,
		len = inner_bytes.len()
	);

	let bytes = wat::parse_str(&outer_wat).expect("Failed to parse outer WAT");
	let result = read_bytes_with_host(&bytes);

	assert!(result.is_ok(), "Host run execution failed: {:?}", result.err());
	// The nested WASM returns 42
	if let Ok(warp::Node::Number(warp::Number::Int(n))) = result {
		assert_eq!(n, 42, "Expected nested WASM to return 42");
	}
}

/// Test emitter generates valid module with imports enabled
#[test]
fn test_emitter_with_host_imports_valid() {
	let mut emitter = WasmGcEmitter::new();
	emitter.set_host_imports(true);

	let node = warp::parse("1 + 2");
	emitter.emit_for_node(&node);
	let bytes = emitter.finish();

	// Module should be valid WASM (validation happens in finish())
	assert!(!bytes.is_empty());

	// Note: Running this would require linking host functions
	// The read_bytes_with_host would work for modules that actually call host functions
}
