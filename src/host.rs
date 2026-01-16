//! Host functions for WASM modules
//! Provides `fetch(url) -> string` and `run(wasm_bytes) -> Node`

use crate::extensions::utils::download;
use crate::node::Node;
use crate::util::gc_engine;
use anyhow::{anyhow, Result};
use log::trace;
use wasmtime::{Caller, Engine, Extern, Linker, Memory, Module, Store, Val};

/// Memory allocator state for host functions
pub struct HostState {
	/// Next free offset in linear memory for string allocation
	next_alloc: u32,
	/// Pending result node from `run` call (stored until it can be returned)
	pending_result: Option<Node>,
}

impl Default for HostState {
	fn default() -> Self {
		Self::new()
	}
}

impl HostState {
	pub fn new() -> Self {
		HostState {
			next_alloc: 65536, // Start allocation after initial memory region
			pending_result: None,
		}
	}

	pub fn alloc(&mut self, size: u32) -> u32 {
		let ptr = self.next_alloc;
		self.next_alloc += size;
		// Align to 8 bytes
		self.next_alloc = (self.next_alloc + 7) & !7;
		ptr
	}
}

/// Read a string from WASM linear memory
pub fn read_string_from_memory(memory: &Memory, store: &impl wasmtime::AsContext, ptr: u32, len: u32) -> Result<String> {
	let mut buf = vec![0u8; len as usize];
	memory.read(store, ptr as usize, &mut buf)?;
	String::from_utf8(buf).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

/// Write a string to WASM linear memory using Caller, returns (ptr, len)
fn write_string_to_caller(
	memory: &Memory,
	caller: &mut Caller<'_, HostState>,
	s: &str,
) -> Result<(u32, u32)> {
	let bytes = s.as_bytes();
	let len = bytes.len() as u32;
	let ptr = caller.data_mut().alloc(len);

	// Grow memory if needed
	let pages_needed = ((ptr + len) as usize).div_ceil(65536);
	let current_pages = memory.size(&*caller) as usize;
	if pages_needed > current_pages {
		let grow = (pages_needed - current_pages) as u64;
		memory.grow(&mut *caller, grow)?;
	}

	memory.write(&mut *caller, ptr as usize, bytes)?;
	Ok((ptr, len))
}

/// Run WASM bytes and return i64 result (for simple modules returning i64)
fn run_wasm_simple(bytes: &[u8]) -> Result<i64> {
	let engine = gc_engine();
	let mut store = Store::new(&engine, ());
	let module = Module::new(&engine, bytes)?;
	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module)?;

	let main = instance
		.get_func(&mut store, "main")
		.ok_or_else(|| anyhow!("No main function"))?;

	let mut results = vec![Val::I64(0)];
	main.call(&mut store, &[], &mut results)?;

	match &results[0] {
		Val::I64(n) => Ok(*n),
		Val::I32(n) => Ok(*n as i64),
		_ => Err(anyhow!("Expected i64 result")),
	}
}

/// Link host functions into a wasmtime Linker
pub fn link_host_functions(linker: &mut Linker<HostState>, _engine: &Engine) -> Result<()> {
	// host.fetch(url_ptr: i32, url_len: i32) -> (result_ptr: i32, result_len: i32)
	// Returns string result via two i32 values (multivalue return)
	linker.func_wrap(
		"host",
		"fetch",
		|mut caller: Caller<'_, HostState>, url_ptr: i32, url_len: i32| -> (i32, i32) {
			let memory = match caller.get_export("memory") {
				Some(Extern::Memory(m)) => m,
				_ => {
					trace!("host.fetch: no memory export");
					return (0, 0);
				}
			};

			// Read URL from WASM memory
			let url = match read_string_from_memory(&memory, &caller, url_ptr as u32, url_len as u32) {
				Ok(s) => s,
				Err(e) => {
					trace!("host.fetch: failed to read URL: {}", e);
					return (0, 0);
				}
			};

			trace!("host.fetch: fetching {}", url);

			// Fetch the URL content
			let mut content = download(&url);
			// Add trailing newline if not present (wasp convention)
			if !content.ends_with('\n') {
				content.push('\n');
			}

			// Write result back to WASM memory
			match write_string_to_caller(&memory, &mut caller, &content) {
				Ok((ptr, len)) => (ptr as i32, len as i32),
				Err(e) => {
					trace!("host.fetch: failed to write result: {}", e);
					(0, 0)
				}
			}
		},
	)?;

	// host.run(wasm_ptr: i32, wasm_len: i32) -> i64
	// Runs WASM bytes and returns result as an i64 (simplified for now)
	// Full GC object return requires more complex setup
	linker.func_wrap(
		"host",
		"run",
		|mut caller: Caller<'_, HostState>, wasm_ptr: i32, wasm_len: i32| -> i64 {
			let memory = match caller.get_export("memory") {
				Some(Extern::Memory(m)) => m,
				_ => {
					trace!("host.run: no memory export");
					return -1;
				}
			};

			// Read WASM bytes from memory
			let mut wasm_bytes = vec![0u8; wasm_len as usize];
			if memory.read(&caller, wasm_ptr as usize, &mut wasm_bytes).is_err() {
				trace!("host.run: failed to read WASM bytes");
				return -1;
			}

			trace!("host.run: executing {} bytes of WASM", wasm_len);

			// Execute the WASM and get result directly
			match run_wasm_simple(&wasm_bytes) {
				Ok(result) => {
					caller.data_mut().pending_result = Some(Node::Number(
						crate::extensions::numbers::Number::Int(result),
					));
					result
				}
				Err(e) => {
					trace!("host.run: execution failed: {}", e);
					-1
				}
			}
		},
	)?;

	// host.get_result_ptr() -> i32
	// Get pointer to last run result (serialized as text)
	linker.func_wrap(
		"host",
		"get_result_ptr",
		|_caller: Caller<'_, HostState>| -> i32 {
			// Return 0 for now - full implementation requires memory allocation
			0
		},
	)?;

	// host.get_result_len() -> i32
	linker.func_wrap(
		"host",
		"get_result_len",
		|_caller: Caller<'_, HostState>| -> i32 {
			0
		},
	)?;

	Ok(())
}

/// Create a linker with host functions pre-linked
pub fn create_host_linker(engine: &Engine) -> Result<Linker<HostState>> {
	let mut linker = Linker::new(engine);
	link_host_functions(&mut linker, engine)?;
	Ok(linker)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_host_state() {
		let mut state = HostState::new();
		assert_eq!(state.alloc(10), 65536);
		assert_eq!(state.alloc(5), 65552); // 65536 + 10, aligned to 8 = 65544? No, 65536+10=65546, aligned = 65552
	}
}
