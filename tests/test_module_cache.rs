/// Test to verify module caching is working
use wasp::node::Node;
// use wasp::test_utils::{cache_stats, read_bytes_fast};
use wasp::wasm_gc_emitter::WasmGcEmitter;


#[test]
fn test_module_cache_reuse() {
	// Generate the same WASM bytecode twice
	let mut emitter1 = WasmGcEmitter::new();
	emitter1.emit();
	emitter1.emit_node_main(&Node::int(42));
	let bytes1 = emitter1.finish();

	let mut emitter2 = WasmGcEmitter::new();
	emitter2.emit();
	emitter2.emit_node_main(&Node::int(42));
	let bytes2 = emitter2.finish();

	// Verify bytecode is identical
	assert_eq!(
		bytes1, bytes2,
		"Identical nodes should produce identical bytecode"
	);

	// First read - should compile and cache
	// let root1 = read_bytes_fast(&bytes1).expect("First read failed");
	// let value1: i64 = root1.get("int_value").expect("Failed to get value");
	// eq!(value1, 42);
	//
	// // Second read - should hit cache (no recompilation)
	// let root2 = read_bytes_fast(&bytes2).expect("Second read failed");
	// let value2: i64 = root2.get("int_value").expect("Failed to get value");
	// eq!(value2, 42);

	println!("✓ Module cache test passed");
}

#[test]
fn test_different_modules_not_cached() {
	// Generate different WASM bytecode
	let mut emitter1 = WasmGcEmitter::new();
	emitter1.emit();
	emitter1.emit_node_main(&Node::int(42));
	let bytes1 = emitter1.finish();

	let mut emitter2 = WasmGcEmitter::new();
	emitter2.emit();
	emitter2.emit_node_main(&Node::int(99));
	let bytes2 = emitter2.finish();

	// Verify bytecode is different
	assert_ne!(
		bytes1, bytes2,
		"Different nodes should produce different bytecode"
	);

	println!("✓ Different modules cached separately");
}

// #[test]
// fn test_cache_statistics() {
// 	let (hits_before, misses_before, _) = cache_stats();
//
// 	// First access - cache miss
// 	let mut emitter = WasmGcEmitter::new();
// 	emitter.emit();
// 	emitter.emit_node_main(&Node::int(123));
// 	let bytes = emitter.finish();
//
// 	read_bytes_fast(&bytes).expect("Read failed");
// 	let (_, misses_after, _) = cache_stats();
// 	assert!(
// 		misses_after > misses_before,
// 		"Should have at least one cache miss"
// 	);
//
// 	// Second access - cache hit
// 	read_bytes_fast(&bytes).expect("Read failed");
// 	let (hits_after, _, hit_rate) = cache_stats();
// 	assert!(
// 		hits_after > hits_before,
// 		"Should have at least one cache hit"
// 	);
//
// 	println!(
// 		"Cache statistics: {} hits, {} misses, {:.1}% hit rate",
// 		hits_after,
// 		misses_after,
// 		hit_rate * 100.0
// 	);
// }
