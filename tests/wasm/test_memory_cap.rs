//! Every run's store refuses to grow a memory or the GC heap past the memory cap (WARP_MEMORY_CAP_MB), loudly: a
//! runaway program stops with an error instead of taking the machine down (154 GB, 2026-10-09)
use wasmtime::{Instance, Module};
use warp::util::{fueled_store, gc_engine};

const GROWN_PAST_THE_CAP: &str = "(module
	(memory 1)
	(type $bytes (array (mut i8)))
	(func (export \"grow_linear_memory\") (result i32) (memory.grow (i32.const 60000)))
	(func (export \"allocate_gc_array\") (result i32) (array.len (array.new_default $bytes (i32.const 0x30000000)))))";

fn run(function: &str) -> String {
	let engine = gc_engine();
	let module = Module::new(&engine, wat::parse_str(GROWN_PAST_THE_CAP).unwrap()).unwrap();
	let mut store = fueled_store(&engine, ());
	let instance = Instance::new(&mut store, &module, &[]).unwrap();
	let call = instance.get_typed_func::<(), i32>(&mut store, function).unwrap().call(&mut store, ());
	format!("{:#}", call.expect_err("grown past the cap"))
}

#[test]
fn linear_memory_stops_at_the_memory_cap() {
	assert!(run("grow_linear_memory").contains("memory cap"), "{}", run("grow_linear_memory"));
}

/// 1.6 GB: the copying collector uses half its heap, so this fits the 4 GB wasmtime allows a GC heap, not the cap's
/// 2 GB; wasmtime drops the limiter's error here (it is on stderr) and traps with its own
#[test]
fn the_gc_heap_stops_at_the_memory_cap() {
	let failure = run("allocate_gc_array");
	assert!(failure.contains("GC heap out of memory"), "{failure}");
}
