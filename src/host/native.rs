//! The host's native side: wasmtime host functions, linear-memory helpers and the linker (feature native)

use super::*;

const PAGE_BITS: u32 = 16;
const I64_BYTES: usize = 8;

/// guarded_call(name, arguments): the node wrapper `name` called with the argument list in the caller's own instance;
/// a stack overflow inside is the Error "call stack exhausted" (what the aborted call wrote to globals and memory stays),
/// any other failure, running out of fuel included, ends the run as before
fn guarded_call(mut caller: Caller<'_, HostState>, name: i32, arguments: Option<wasmtime::Rooted<wasmtime::AnyRef>>) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	let message = |failure: anyhow::Error| wasmtime::Error::msg(failure.to_string());
	let name = crate::tasks::c_string(&mut caller, name).map_err(message)?;
	let function = caller.get_export(&name).and_then(Extern::into_func).ok_or_else(|| wasmtime::Error::msg(format!("no exported function {name}")))?;
	// the wrapper gives back its function's result: a Node, an Int or a Float
	let mut result = [match function.ty(&caller).results().next() {
		Some(wasmtime::ValType::I64) => Val::I64(0),
		Some(wasmtime::ValType::F64) => Val::F64(0),
		_ => Val::AnyRef(None),
	}];
	let value = match function.call(&mut caller, &[Val::AnyRef(arguments)], &mut result) {
		Ok(()) => match result[0] {
			Val::I64(n) => crate::tasks::TaskValue::Int(n),
			Val::F64(bits) => crate::tasks::TaskValue::Float(warp_runtime::floats::canonical_nan(f64::from_bits(bits))),
			node => return Ok(node.unwrap_anyref().copied()),
		},
		Err(failure) if failure.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::StackOverflow) => return error_in_program(&mut caller, STACK_EXHAUSTED),
		Err(failure) => return Err(failure),
	};
	let builders = crate::tasks::Builders::of(&mut |export| caller.get_export(export)).map_err(message)?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(message)?;
	Ok(built.unwrap_anyref().copied())
}

/// The Error of `reason`, built with the module's own error_of: a failure the program's `try` catches
fn error_in_program(caller: &mut Caller<'_, HostState>, reason: &str) -> wasmtime::Result<HostNode> {
	let text = built_in_program(caller, &Node::Text(reason.to_string()), crate::wasm_emitter::text_builtins::ERROR_OF)?;
	let error_of = caller.get_export(crate::wasm_emitter::text_builtins::ERROR_OF).and_then(Extern::into_func).ok_or_else(|| wasmtime::Error::msg("no exported error_of"))?;
	let mut error = [Val::AnyRef(None)];
	error_of.call(&mut *caller, &[Val::AnyRef(text)], &mut error)?;
	Ok(error[0].unwrap_anyref().copied())
}

use crate::tasks::task_failure;
use crate::util::gc_engine;
use anyhow::anyhow;
use log::trace;
use wasmtime::{AsContextMut, Caller, Engine, Extern, Linker, Memory, Val};

/// The state of a running program, one for all its imports: host functions, WASI and FFI share one linker, so a program
/// can print and fetch, or print and call C
pub struct HostState {
	/// Next free offset in linear memory for string allocation
	next_alloc: u32,
	/// WASI preview 1 (fd_write …), writing to the process's stdout and stderr
	pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
	/// Whether this store runs a task's instance (tasks.rs): a raise there goes to the starting thread's handlers
	pub in_task: bool,
	/// The C pointers this run got (sqlite3 *, FILE *), handed to warp as ids (notes/ffi_handles.md)
	pub c_handles: crate::ffi::CHandles,
	/// The WebAssembly modules the program imports, instantiated at their first call (wasm_modules.rs), by path
	pub wasm_modules: std::collections::HashMap<String, wasmtime::Instance>,
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
			// the environment as `use os; env(name)` reads it natively, for wasi_environment()
			wasi: wasmtime_wasi::WasiCtxBuilder::new().inherit_stdout().inherit_stderr().inherit_env().build_p1(),
			c_handles: Default::default(),
			wasm_modules: Default::default(),
			in_task: false,
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


/// Copy bytes into the module's memory: from its text heap when it exports one (the same rule as its own
/// emit_text_allocation: fresh pages past the current memory when the heap is unset or full), else from HostState
fn write_bytes_to_caller(memory: &Memory, caller: &mut Caller<'_, HostState>, bytes: &[u8]) -> Result<(u32, u32)> {
	let heap = match caller.get_export(TEXT_HEAP_EXPORT) {
		Some(Extern::Global(global)) => Some(global),
		_ => None,
	};
	write_bytes(memory, heap, &mut caller.as_context_mut(), bytes)
}

/// Copy bytes into a module's memory at its text heap `heap` (or from HostState without one): from a host function
/// (its caller) or into an instance the host made (a task, tasks.rs)
pub fn write_bytes(memory: &Memory, heap: Option<wasmtime::Global>, store: &mut wasmtime::StoreContextMut<'_, HostState>, bytes: &[u8]) -> Result<(u32, u32)> {
	match heap {
		Some(heap) => write_to_heap(memory, heap, store, bytes),
		None => {
			let ptr = store.data_mut().alloc(bytes.len() as u32);
			write_at(memory, store, ptr, bytes)
		}
	}
}

/// Copy bytes into a module's memory at its text heap, for any store (an FFI call's too, ffi.rs): fresh pages past the
/// current memory when the heap is unset or full, the module's own emit_text_allocation rule
pub fn write_to_heap<T>(memory: &Memory, heap: wasmtime::Global, store: &mut wasmtime::StoreContextMut<'_, T>, bytes: &[u8]) -> Result<(u32, u32)> {
	let len = bytes.len() as u32;
	let top = heap.get(&mut *store).i32().unwrap_or(0) as u32;
	let memory_end = (memory.size(&*store) as u32) << PAGE_BITS;
	let ptr = if top == 0 || top + len > memory_end {
		memory.grow(&mut *store, ((len >> PAGE_BITS) + 1) as u64)?;
		memory_end
	} else {
		top
	};
	let written = write_at(memory, store, ptr, bytes)?;
	heap.set(&mut *store, Val::I32((ptr + len) as i32))?;
	Ok(written)
}

/// The bytes at `ptr`, the memory grown to hold them
fn write_at<T>(memory: &Memory, store: &mut wasmtime::StoreContextMut<'_, T>, ptr: u32, bytes: &[u8]) -> Result<(u32, u32)> {
	let len = bytes.len() as u32;
	let pages_needed = ((ptr + len) as usize).div_ceil(1 << PAGE_BITS);
	let current_pages = memory.size(&*store) as usize;
	if pages_needed > current_pages {
		memory.grow(&mut *store, (pages_needed - current_pages) as u64)?;
	}
	memory.write(&mut *store, ptr as usize, bytes)?;
	Ok((ptr, len))
}

/// The file at the path in WASM memory, written back like a fetched body: (ptr, len), or (ptr, -len) of the failure reason
fn read_into_memory(caller: &mut Caller<'_, HostState>, path_ptr: i32, path_len: i32) -> (i32, i32) {
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
		trace!("host.read: no memory export");
		return (0, 0);
	};
	let (bytes, failed) = match read_string_from_memory(&memory, &*caller, path_ptr as u32, path_len as u32) {
		Ok(path) => match std::fs::read(&path) {
			Ok(bytes) => (bytes, false),
			Err(reason) => (format!("read {path} failed: {reason}").into_bytes(), true),
		},
		Err(e) => (format!("read failed: unreadable path: {e}").into_bytes(), true),
	};
	answer_in_memory(&memory, caller, &bytes, failed, "read")
}

/// The answer written into the module's memory: (ptr, len), or (ptr, -len) of a failure's reason; (0, 0) if it could
/// not be written
fn answer_in_memory(memory: &Memory, caller: &mut Caller<'_, HostState>, bytes: &[u8], failed: bool, word: &str) -> (i32, i32) {
	match write_bytes_to_caller(memory, caller, bytes) {
		Ok((ptr, len)) if failed => (ptr as i32, -(len as i32)),
		Ok((ptr, len)) => (ptr as i32, len as i32),
		Err(e) => {
			trace!("host.{word}: failed to write result: {}", e);
			(0, 0)
		}
	}
}

/// Run WASM bytes and return i64 result (for simple modules returning i64)
fn run_wasm_simple(bytes: &[u8]) -> Result<i64> {
	let engine = gc_engine();
	let mut store = crate::util::fueled_store(&engine, ());
	let module = crate::run::module_cache::compiled_module(&engine, bytes)?;
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

/// Fetch the URL in WASM memory and write the result back: (ptr, len) of the body, (ptr, -len) of the failure reason
fn fetch_into_memory(caller: &mut Caller<'_, HostState>, url_ptr: i32, url_len: i32, timeout: Duration) -> (i32, i32) {
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
		trace!("host.fetch: no memory export");
		return (0, 0);
	};
	let (text, failed) = match read_string_from_memory(&memory, &*caller, url_ptr as u32, url_len as u32) {
		Ok(url) => {
			trace!("host.fetch: fetching {} (timeout {:?})", url, timeout);
			match fetch(&url, timeout) {
				Ok(body) => (body, false),
				Err(reason) => (reason, true),
			}
		}
		Err(e) => (format!("fetch failed: unreadable URL: {e}"), true),
	};
	answer_in_memory(&memory, caller, text.as_bytes(), failed, "fetch")
}

/// Link host functions into a wasmtime Linker
pub fn link_host_functions(linker: &mut Linker<HostState>, _engine: &Engine) -> Result<()> {
	// host.fetch(url_ptr: i32, url_len: i32) -> (result_ptr: i32, result_len: i32)
	// Returns the body via two i32 values (multivalue return); a negative length marks the text as the failure reason
	linker.func_wrap(
		"host",
		"fetch",
		|mut caller: Caller<'_, HostState>, url_ptr: i32, url_len: i32| -> (i32, i32) {
			fetch_into_memory(&mut caller, url_ptr, url_len, FETCH_TIMEOUT)
		},
	)?;

	// host.fetch_within(url_ptr: i32, url_len: i32, timeout_ms: i64) -> (result_ptr: i32, result_len: i32)
	linker.func_wrap(
		"host",
		"fetch_within",
		|mut caller: Caller<'_, HostState>, url_ptr: i32, url_len: i32, timeout_ms: i64| -> (i32, i32) {
			fetch_into_memory(&mut caller, url_ptr, url_len, Duration::from_millis(timeout_ms.max(1) as u64))
		},
	)?;

	// host.read(path_ptr: i32, path_len: i32) -> (result_ptr: i32, result_len: i32), the file's bytes
	linker.func_wrap(
		"host",
		"read",
		|mut caller: Caller<'_, HostState>, path_ptr: i32, path_len: i32| -> (i32, i32) {
			read_into_memory(&mut caller, path_ptr, path_len)
		},
	)?;

	linker.func_wrap(HOST_LIBRARY, GUARDED_CALL, guarded_call)?;
	warp_runtime::host_words::link_host_words(linker)?;
	linker.func_wrap(HOST_LIBRARY, RUN_BLOCK, run_block)?;
	linker.func_wrap(HOST_LIBRARY, BLOCK_VALUE, block_value)?;
	linker.func_wrap(HOST_LIBRARY, FOREIGN_CALL, foreign_call)?;
	linker.func_wrap(HOST_LIBRARY, STD_PURE, std_call)?;
	linker.func_wrap(HOST_LIBRARY, STD_IO, std_call)?;
	linker.func_wrap(HOST_LIBRARY, SERVE_ROUTES, serve_routes)?;
	linker.func_wrap(HOST_LIBRARY, FETCH_START, fetch_start)?;
	linker.func_wrap(HOST_LIBRARY, FETCH_REPLY, fetch_reply)?;
	crate::channels::forget_listeners(); // each run links anew, on its own thread
	linker.func_wrap(HOST_LIBRARY, CHANNEL_LISTEN, channel_listen)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_PENDING, crate::channels::pending)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_NEXT, channel_next)?;
	linker.func_wrap(HOST_LIBRARY, CLIPBOARD_TEXT, clipboard_text)?;
	linker.func_wrap(HOST_LIBRARY, PAGE_PATH, page_path)?;
	linker.func_wrap(HOST_LIBRARY, NOTIFY, notify)?;
	linker.func_wrap(HOST_LIBRARY, GPU_COMPUTE, gpu_compute)?;
	linker.func_wrap(HOST_LIBRARY, GPU_RENDER, gpu_render)?;
	linker.func_wrap(HOST_LIBRARY, TEXT_COVERAGE, text_coverage)?;
	linker.func_wrap(HOST_LIBRARY, GPU_COMPUTE_LINEAR, gpu_compute_linear)?;
	linker.func_wrap(HOST_LIBRARY, GPU_MAP_LINEAR, gpu_map_linear)?;
	linker.func_wrap(HOST_LIBRARY, GPU_REDUCE_LINEAR, gpu_reduce_linear)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_SEND, channel_send)?;
	linker.func_wrap(HOST_LIBRARY, PAINT, paint)?;
	linker.func_wrap(HOST_LIBRARY, SOUND, sound)?;

	// host.warn(message_ptr: i32, message_len: i32): a runtime warning, reported and collected
	linker.func_wrap("host", "warn", |mut caller: Caller<'_, HostState>, message_ptr: i32, message_len: i32| {
		if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
			match read_string_from_memory(&memory, &caller, message_ptr as u32, message_len as u32) {
				Ok(message) => crate::diagnostic::report_runtime_warning(&message),
				Err(e) => trace!("host.warn: unreadable message: {e}"),
			}
		}
	})?;

	// host.run(wasm_ptr: i32, wasm_len: i32) -> i64
	// Runs WASM bytes and returns result as an i64 (simplified for now)
	// Full GC object return requires more complex setup
	linker.func_wrap(
		"host",
		"run",
		|mut caller: Caller<'_, HostState>, wasm_ptr: i32, wasm_len: i32| -> i64 {
			let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
				trace!("host.run: no memory export");
				return -1;
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
				Ok(result) => result,
				Err(e) => {
					trace!("host.run: execution failed: {}", e);
					-1
				}
			}
		},
	)?;

	Ok(())
}

/// paint(pixels, width, height) natively: the pixels as a PNG (src/paint.rs); paint(shader, width, height, values)
/// renders the WGSL fragment shader first, as gpu_render does (P234)
fn paint(mut caller: Caller<'_, HostState>, pixels: HostNode, width: i64, height: i64, shader_values: HostNode) -> wasmtime::Result<()> {
	warp_runtime::system_signals::end_at_first_frame()?;
	let memory = module_memory(&mut caller, "paint")?;
	let size = (width.max(0) as usize, height.max(0) as usize);
	let values = match ints_of_list(&mut caller, memory, pixels, size.0 * size.1)? {
		Some(values) => values,
		None => match crate::wasm_reader::node_in(&Val::AnyRef(pixels), &mut caller.as_context_mut(), memory).drop_meta() {
			crate::node::Node::List(items, _, _) => items.iter().map(pixel_value).collect(),
			crate::node::Node::Text(shader) => {
				let rgba = rendered(&mut caller, shader, width, height, shader_values, &gpu_failure(PAINT))?;
				return crate::paint::paint_rgba(&rgba, size.0, size.1).map(|_| ()).map_err(task_failure);
			}
			other => return Err(task_failure(format!("paint needs a list of pixels or a shader text, got {}", other.serialize()))),
		},
	};
	crate::paint::paint(&values, size.0, size.1).map(|_| ()).map_err(task_failure)
}

/// sound_samples(samples, count, rate) natively: a WAV file, played unless headless (src/sound.rs)
fn sound(mut caller: Caller<'_, HostState>, samples: HostNode, count: i64, rate: i64) -> wasmtime::Result<()> {
	let memory = module_memory(&mut caller, "sound")?;
	let values = match ints_of_list(&mut caller, memory, samples, count.max(0) as usize)? {
		Some(values) => values,
		None => match crate::wasm_reader::node_in(&Val::AnyRef(samples), &mut caller.as_context_mut(), memory).drop_meta() {
			crate::node::Node::List(items, _, _) => items.iter().map(pixel_value).collect(),
			other => return Err(task_failure(format!("sound needs a list of samples, got {}", other.serialize()))),
		},
	};
	crate::sound::sound(&values, rate.clamp(1, u32::MAX as i64) as u32).map(|_| ()).map_err(task_failure)
}

/// The first `room` items of a list of fixnum Ints read in one call of the module's list_to_ints (wasm_emitter/int_lists.rs), their magnitudes;
/// None for any other value, which the caller reads node by node
fn ints_of_list(caller: &mut Caller<'_, HostState>, memory: Memory, list: HostNode, room: usize) -> wasmtime::Result<Option<Vec<u64>>> {
	use crate::wasm_emitter::int_lists::LIST_TO_INTS;
	let Some(Extern::Func(list_to_ints)) = caller.get_export(LIST_TO_INTS) else { return Ok(None) };
	let list_to_ints = list_to_ints.typed::<(HostNode, i32, i32), i32>(&*caller)?;
	let Some(address) = scratch(caller, memory, room * I64_BYTES)? else { return Ok(None) };
	let written = list_to_ints.call(&mut *caller, (list, address as i32, room as i32))?;
	let Ok(written) = usize::try_from(written) else { return Ok(None) };
	let mut bytes = vec![0; written * I64_BYTES];
	memory.read(&*caller, address as usize, &mut bytes)?;
	Ok(Some(bytes.chunks_exact(I64_BYTES).map(|value| u64::from_le_bytes(value.try_into().expect("8 bytes"))).collect()))
}

/// Free memory for `bytes` bytes above the module's text heap, which the next text may take again (int_lists.rs), at
/// an i64's alignment: the heap starts past the memory when unset, as write_to_heap starts it; None without a text heap
fn scratch(caller: &mut Caller<'_, HostState>, memory: Memory, bytes: usize) -> wasmtime::Result<Option<u32>> {
	let Some(Extern::Global(heap)) = caller.get_export(TEXT_HEAP_EXPORT) else { return Ok(None) };
	let memory_end = (memory.size(&*caller) as u32) << PAGE_BITS;
	let address = match heap.get(&mut *caller).i32().unwrap_or(0) as u32 {
		0 => {
			heap.set(&mut *caller, Val::I32(memory_end as i32))?;
			memory_end
		}
		top => top.next_multiple_of(I64_BYTES as u32),
	};
	let pages_needed = (address as usize + bytes).div_ceil(1 << PAGE_BITS);
	let pages = memory.size(&*caller) as usize;
	if pages_needed > pages {
		memory.grow(&mut *caller, (pages_needed - pages) as u64)?;
	}
	Ok(Some(address))
}

/// A pixel as paint reads it: 0 for false, ø and 0, a number as it is (a color 0xAARRGGBB), any other value 1 (ink)
fn pixel_value(pixel: &crate::node::Node) -> u64 {
	use crate::node::Node;
	match pixel.drop_meta() {
		Node::False | Node::Empty => 0,
		Node::Number(number) => f64::from(*number).abs() as u64,
		_ => 1,
	}
}

/// The host side of run_block: the block and the values it sees come in as Nodes, the block's value goes back as one
fn run_block(mut caller: Caller<'_, HostState>, block: Option<wasmtime::Rooted<wasmtime::AnyRef>>, names: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	values: Option<wasmtime::Rooted<wasmtime::AnyRef>>, definitions: Option<wasmtime::Rooted<wasmtime::AnyRef>>) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::tasks::TaskValue;
	let memory = module_memory(&mut caller, "run_block")?;
	let mut store = caller.as_context_mut();
	let [block, names, values, definitions] = [block, names, values, definitions].map(|value| crate::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let result = crate::pipeline::eval_block(block, &names, &values, &definitions).map_err(task_failure)?;
	let value = TaskValue::of(&result).map_err(|_| task_failure(crate::pipeline::cannot_hand_back(&result)))?;
	build_in_program(&mut caller, &value, &task_failure)
}

thread_local! {
	/// The values of the blocks running on this thread, the innermost last (a block may run a block)
	static BLOCK_VALUES: std::cell::RefCell<Vec<Vec<crate::tasks::TaskValue>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `body` while block·value reads `values`
pub fn with_block_values<R>(values: Vec<crate::tasks::TaskValue>, body: impl FnOnce() -> R) -> R {
	BLOCK_VALUES.with(|stack| stack.borrow_mut().push(values));
	let result = body();
	BLOCK_VALUES.with(|stack| stack.borrow_mut().pop());
	result
}

/// The host side of block·value: the value at `index`, built in the block's module
fn block_value(mut caller: Caller<'_, HostState>, index: i64) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	let failure = |message: String| wasmtime::Error::msg(message);
	let value = BLOCK_VALUES.with(|stack| stack.borrow().last().and_then(|values| values.get(index as usize).cloned()))
		.ok_or_else(|| failure(format!("{BLOCK_VALUE}({index}): no such value of the running block")))?;
	build_in_program(&mut caller, &value, &failure)
}

/// The host side of foreign_call: the runtime, module and member names and the arguments come in as Nodes, the
/// answer goes back as one (src/foreign.rs)
fn foreign_call(mut caller: Caller<'_, HostState>, runtime: Option<wasmtime::Rooted<wasmtime::AnyRef>>, module: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	member: Option<wasmtime::Rooted<wasmtime::AnyRef>>, call: Option<wasmtime::Rooted<wasmtime::AnyRef>>, arguments: Option<wasmtime::Rooted<wasmtime::AnyRef>>) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::tasks::{Builders, TaskValue};
	let memory = module_memory(&mut caller, "foreign_call")?;
	let mut store = caller.as_context_mut();
	let functions = function_arguments(&mut store, arguments)?;
	let [runtime, module, member, call, arguments] = [runtime, module, member, call, arguments].map(|value| crate::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let arguments = with_function_markers(arguments, &functions);
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| task_failure(problem.to_string()))?;
	let mut callback = |index: usize, values: Node| -> Result<Node, String> {
		let Some(Extern::Func(apply)) = caller.get_export(crate::wasm_emitter::CLOSURE_APPLY) else { return Err("the module exports no closure_apply".into()) };
		let function = functions.iter().flatten().nth(index).copied().ok_or(format!("no warp function {index}"))?;
		let values = TaskValue::of(&values).and_then(|values| builders.build(&values, &mut caller.as_context_mut())).map_err(|problem| problem.to_string())?;
		let mut result = [Val::AnyRef(None)];
		apply.call(&mut caller, &[Val::AnyRef(Some(function)), values], &mut result).map_err(|trap| trap.root_cause().to_string())?;
		given_node(&mut caller, result[0].unwrap_anyref().copied()).map_err(|problem| problem.to_string())
	};
	let answer = crate::foreign::call(&runtime.name(), &module, &member.name(), call == 1, &arguments, &mut callback).map_err(task_failure)?;
	let value = TaskValue::of(&answer).map_err(|problem| task_failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| task_failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// A foreign call's arguments, the list's items or the one argument, each a closure or None
fn function_arguments(store: &mut wasmtime::StoreContextMut<'_, HostState>, arguments: HostNode) -> wasmtime::Result<Vec<Option<wasmtime::Rooted<wasmtime::AnyRef>>>> {
	use crate::wasm_reader::{FIELD_DATA, FIELD_KIND, FIELD_VALUE};
	use crate::type_kinds::{Kind, KIND_MASK};
	let node_of = |value: &Val, store: &wasmtime::StoreContextMut<'_, HostState>| value.unwrap_anyref().and_then(|reference| reference.unwrap_struct(store).ok());
	let kind_of = |node: &wasmtime::StructRef, store: &mut wasmtime::StoreContextMut<'_, HostState>| node.field(&mut *store, FIELD_KIND).map(|kind| (kind.unwrap_i64() & KIND_MASK) as u8);
	let mut items = vec![];
	let mut cell = node_of(&Val::AnyRef(arguments), store);
	match &cell {
		Some(node) if kind_of(node, store)? == Kind::List as u8 => {}
		_ => cell = None,
	}
	if cell.is_none() {
		items.extend(arguments.into_iter().map(|argument| Val::AnyRef(Some(argument))));
	}
	while let Some(current) = cell {
		let first = current.field(&mut *store, FIELD_DATA)?;
		if first.unwrap_anyref().is_some() {
			items.push(first);
		}
		cell = node_of(&current.field(&mut *store, FIELD_VALUE)?, store);
	}
	items.iter().map(|item| Ok(match node_of(item, store) {
		Some(node) if kind_of(&node, store)? == Kind::Function as u8 => item.unwrap_anyref().copied(),
		_ => None,
	})).collect()
}

/// The arguments with each closure `{$warp_function: index}`, its index among the closures (crate::foreign)
fn with_function_markers(arguments: Node, functions: &[Option<wasmtime::Rooted<wasmtime::AnyRef>>]) -> Node {
	if functions.iter().all(Option::is_none) {
		return arguments;
	}
	let marker = |index: usize| Node::List(vec![Node::key(crate::foreign::WARP_FUNCTION_KEY, Node::int(index as i64))], crate::node::Bracket::Curly, crate::node::Separator::Space);
	let mut index = 0;
	let mut marked = |item: Node, function: &Option<_>| match function {
		Some(_) => (marker(index), index += 1).0,
		None => item,
	};
	match arguments.drop_meta().clone() {
		Node::List(items, bracket, separator) if items.len() == functions.len() => Node::List(items.into_iter().zip(functions).map(|(item, function)| marked(item, function)).collect(), bracket, separator),
		single => marked(single, &functions[0]),
	}
}

type HostNode = Option<wasmtime::Rooted<wasmtime::AnyRef>>;

/// serve_routes(port, routes): HTTP on the port, each request answered by its route's function (src/web_server.rs)
fn serve_routes(mut caller: Caller<'_, HostState>, port: i64, routes: HostNode) -> wasmtime::Result<HostNode> {
	use crate::web_server::{routes_of, serve};
	let routes = routes_of(&given_node(&mut caller, routes)?);
	let port = u16::try_from(port).map_err(|_| task_failure(format!("serve {port}: no port number")))?;
	let site = served_site(port);
	serve(port, &routes, &site, |route, request| match route_value(&mut caller, &route.function, &request) {
		Ok(value) => Ok(route.answer_of(&value)),
		Err(failure) => Err(route_failure(&mut caller, &route.path, failure)),
	}).map_err(task_failure)?;
	built_in_program(&mut caller, &Node::Empty, SERVE_ROUTES)
}

/// What a failed route says to the client: the program's error message, read with the value the program left in
/// trap_detail (as wasm_reader::with_trap_detail does for main); the whole trace goes to the server's log
fn route_failure(caller: &mut Caller<'_, HostState>, path: &str, failure: wasmtime::Error) -> crate::web_server::RouteFailure {
	let detail = caller.get_export(crate::wasm_reader::TRAP_DETAIL).and_then(Extern::into_global);
	let left = detail.map(|global| global.get(&mut *caller));
	// read once: the next failure must not show this one's detail
	if let Some(global) = detail {
		let _ = global.set(&mut *caller, Val::AnyRef(None));
	}
	let failure = anyhow::Error::from(failure);
	let failure = match left {
		Some(Val::AnyRef(Some(value))) => match given_node(caller, Some(value)) {
			Ok(node) => failure.context(crate::wasm_reader::trap_detail_line(&node)),
			Err(_) => failure,
		},
		_ => failure,
	};
	let trace = format!("{failure:?}");
	let message = crate::tasks::failure_message(failure);
	eprintln!("{path} failed: {message}\n{trace}");
	crate::web_server::RouteFailure { message, raised: trace.contains(crate::wasm_emitter::list_ops::RETURNED_ERROR) }
}

/// The site of the program file being run, which a program whose last line shows a page serves at / (src/site.rs); a
/// page that fails to build is said, and the routes are served without it
fn served_site(port: u16) -> crate::site::ServedSite {
	let Some(file) = crate::modules::program_file() else { return Default::default() };
	let title = crate::site::program_stem(&file);
	let rendered = std::fs::read_to_string(&file).map_err(|failure| failure.to_string()).and_then(|code| crate::site::served_files(&code, &title));
	rendered.unwrap_or_else(|failure| {
		eprintln!("warning: serve {port} serves no page: {failure}");
		None
	}).unwrap_or_default()
}

/// The value of a route's function called with the request (if it takes one), as a Node
fn route_value(caller: &mut Caller<'_, HostState>, function: &str, request: &Node) -> wasmtime::Result<Node> {
	let Some(Extern::Func(route)) = caller.get_export(function) else { return Err(wasmtime::Error::msg(format!("no route function {function}"))) };
	let arguments = match route.ty(&*caller).params().len() {
		0 => vec![],
		_ => vec![Val::AnyRef(built_in_program(caller, request, function)?)],
	};
	let mut results = vec![Val::I64(0); route.ty(&*caller).results().len()];
	route.call(&mut *caller, &arguments, &mut results)?;
	Ok(match results.first() {
		Some(Val::I64(integer)) => Node::int(*integer),
		Some(Val::F64(bits)) => Node::Number(crate::extensions::numbers::Number::Float(f64::from_bits(*bits))),
		Some(Val::AnyRef(value)) => given_node(caller, *value)?,
		_ => Node::Empty,
	})
}

/// std_pure / std_io(module, member, arguments): a word of the standard library's adapters (src/std_adapters.rs)
fn std_call(mut caller: Caller<'_, HostState>, module: HostNode, member: HostNode, arguments: HostNode) -> wasmtime::Result<HostNode> {
	let [module, member, arguments] = [module, member, arguments].map(|value| given_node(&mut caller, value));
	let (module, member) = (module?.name(), member?.name());
	let answer = match (module.as_str(), member.as_str()) {
		("table", "select") => queried_rows(&mut caller, &arguments?)?,
		_ => match crate::std_adapters::call(&module, &member, &arguments?) {
			Ok(answer) => answer,
			// the program raises it (emit_ffi_result): `try` catches it, uncaught it ends the run with this reason
			Err(problem) => return error_in_program(&mut caller, &problem),
		},
	};
	built_in_program(&mut caller, &answer, &format!("{module}.{member}"))
}

/// The ids a filter's query keeps (database.rs select), its warp_call calling the program's functions back; a function
/// that fails fails the query with its own error (the trap, whose trap_detail says what went wrong)
fn queried_rows(caller: &mut Caller<'_, HostState>, arguments: &Node) -> wasmtime::Result<Node> {
	let mut trapped = None;
	let rows = crate::database::select(&arguments.children(), &mut |function, request| route_value(caller, function, request).map_err(|trap| {
		let problem = format!("{trap:#}");
		trapped.get_or_insert(trap);
		problem
	}));
	match (rows, trapped) {
		(Ok(rows), _) => Ok(rows),
		(Err(_), Some(trap)) => Err(trap),
		(Err(problem), None) => Err(task_failure(format!("table.select: {problem}"))),
	}
}

/// The node a host word was given, read out of the caller's instance
fn given_node(caller: &mut Caller<'_, HostState>, value: HostNode) -> wasmtime::Result<Node> {
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return Err(wasmtime::Error::msg("the module exports no memory")) };
	Ok(crate::wasm_reader::node_in(&Val::AnyRef(value), &mut caller.as_context_mut(), memory))
}

/// `on message from "chat" {…}` starts listening (src/channels.rs)
fn channel_listen(mut caller: Caller<'_, HostState>, id: i64, channel: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	crate::channels::listen(id, &channel.name()).map_err(task_failure)
}

/// A fetch's URL, or `[url, body]`: a POST of the body as JSON (a server function the page calls, lowering/serve.rs)
fn request_of(request: &Node) -> crate::fetches::Request {
	match request.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => (items[0].name(), Some(crate::foreign::json_of(&items[1]).to_string())),
		url => (url.name(), None),
	}
}

/// `users := fetch url` starts the fetch (src/fetches.rs)
fn fetch_start(mut caller: Caller<'_, HostState>, id: i64, url: HostNode) -> wasmtime::Result<()> {
	let request = given_node(&mut caller, url)?;
	crate::fetches::start(id, request_of(&request), FETCH_TIMEOUT);
	Ok(())
}

/// [value, error] of the fetch's reply (src/fetches.rs)
fn fetch_reply(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	built_in_program(&mut caller, &crate::fetches::reply(id), "fetch")
}

/// The oldest message the listener got, as the value it was sent as (ø when none waits)
fn channel_next(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	let message = crate::web_sockets::next(id).unwrap_or_else(|| crate::channels::next(id).map(|text| crate::warp_parser::parse(&text).drop_meta().clone()).unwrap_or(Node::Empty));
	built_in_program(&mut caller, &message, "on message")
}

/// `clipboard`: the clipboard's text, read now (warp-runtime system_values.rs)
fn clipboard_text(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let text = warp_runtime::system_values::clipboard_text().map_err(task_failure)?;
	built_in_program(&mut caller, &Node::Text(text), "clipboard")
}

/// The path of the page natively: "/", or the one a render for a request is made for (with_page_path)
fn page_path(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let path = NATIVE_PAGE_PATH.with(|path| path.borrow().clone());
	built_in_program(&mut caller, &Node::Text(path), PAGE_PATH)
}

thread_local! {
	static NATIVE_PAGE_PATH: std::cell::RefCell<String> = std::cell::RefCell::new(ROOT_PATH.to_string());
}
const ROOT_PATH: &str = "/";

/// Run `body` with the page at `path` (a page rendered for a request, src/site.rs)
pub fn with_page_path<R>(path: &str, body: impl FnOnce() -> R) -> R {
	let previous = NATIVE_PAGE_PATH.with(|current| current.replace(path.to_string()));
	let result = body();
	NATIVE_PAGE_PATH.with(|current| current.replace(previous));
	result
}

/// The value built in the program by its exported constructors
fn build_in_program(caller: &mut Caller<'_, HostState>, value: &crate::tasks::TaskValue, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<HostNode> {
	let builders = crate::tasks::Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// A value of the host as a value of the program, built by its exported constructors
fn built_in_program(caller: &mut Caller<'_, HostState>, value: &Node, word: &str) -> wasmtime::Result<HostNode> {
	use crate::tasks::TaskValue;
	let failure = |problem: String| task_failure(format!("{word}: {problem}"));
	let value = TaskValue::of(value).map_err(|problem| failure(problem.to_string()))?;
	build_in_program(caller, &value, &failure)
}

/// `broadcast value on "chat"`: the value as warp text to every listener of the channel
fn channel_send(mut caller: Caller<'_, HostState>, channel: HostNode, message: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	let message = given_node(&mut caller, message)?;
	if crate::web_sockets::is_socket_address(&channel.name()) {
		return crate::web_sockets::send(&channel.name(), &message).map_err(task_failure);
	}
	crate::channels::send(&channel.name(), message.serialize().trim()).map_err(task_failure)
}

/// `notify "text"`: a desktop notification of the text (a value other than a text as warp writes it)
fn notify(mut caller: Caller<'_, HostState>, text: HostNode) -> wasmtime::Result<()> {
	let text = match given_node(&mut caller, text)? {
		Node::Text(text) => text,
		other => other.serialize(),
	};
	warp_runtime::system_values::notify(&text).map_err(task_failure)
}

/// `gpu_compute(shader, numbers, workgroups)` through wgpu (src/gpu.rs): the floats the shader left
fn gpu_compute(mut caller: Caller<'_, HostState>, shader: HostNode, numbers: HostNode, workgroups: i64) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_COMPUTE);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let numbers = given_node(&mut caller, numbers)?;
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
	if let Some(arrays) = named_arrays(&numbers) {
		let arrays = arrays.into_iter().map(|(name, numbers)| Ok((name, numbers.iter().map(|number| number_of(&number, &failure).map(f64::from)).collect::<wasmtime::Result<Vec<f64>>>()?))).collect::<wasmtime::Result<Vec<_>>>()?;
		let left = crate::gpu::compute_named(&shader, &arrays, workgroups).map_err(&failure)?;
		let entries = left.into_iter().map(|(name, element, numbers)| {
			let item = |number: f64| Node::Number(if element == crate::gpu::Element::Float { Number::Float(number) } else { Number::Int(number as i64) });
			Node::Key(Box::new(Node::Symbol(name)), crate::operators::Op::Colon, Box::new(Node::List(numbers.into_iter().map(item).collect(), crate::node::Bracket::Square, crate::node::Separator::Space)))
		}).collect();
		return built_in_program(&mut caller, &Node::List(entries, crate::node::Bracket::Curly, crate::node::Separator::Space), GPU_COMPUTE);
	}
	let numbers = numbers.iter().map(|number| number_of(&number, &failure)).collect::<wasmtime::Result<Vec<Number>>>()?;
	let left: Vec<Node> = match crate::gpu::element_of(&shader) {
		crate::gpu::Element::Float => {
			let floats: Vec<f32> = numbers.iter().map(|&number| f64::from(number) as f32).collect();
			crate::gpu::compute(&shader, &floats, workgroups).map_err(&failure)?.into_iter().map(|float| Node::Number(Number::Float(float as f64))).collect()
		}
		element => {
			let ints = numbers.iter().map(|number| match number {
				Number::Int(int) => Ok(*int),
				other => Err(failure(format!("an array<i32> or array<u32> shader computes over ints, got {other}"))),
			}).collect::<wasmtime::Result<Vec<i64>>>()?;
			crate::gpu::compute_ints(&shader, &ints, workgroups, element).map_err(&failure)?.into_iter().map(|int| Node::Number(Number::Int(int))).collect()
		}
	};
	built_in_program(&mut caller, &Node::List(left, crate::node::Bracket::Square, crate::node::Separator::Space), GPU_COMPUTE)
}

fn number_of(node: &Node, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<Number> {
	match node.drop_meta() {
		Node::Number(number) => Ok(*number),
		other => Err(failure(format!("it computes over numbers, got {}", other.serialize()))),
	}
}

/// `{xs: [1 2 3], ys: [4 5 6]}`: gpu_compute's named arrays, each bound where the shader declares one of that name
fn named_arrays(numbers: &Node) -> Option<Vec<(String, &Node)>> {
	let Node::List(entries, _, _) = numbers.drop_meta() else { return None };
	entries.iter().map(|entry| match entry.drop_meta() {
		Node::Key(name, _, array) => match name.drop_meta() {
			Node::Symbol(name) | Node::Text(name) => Some((name.clone(), array.as_ref())),
			_ => None,
		},
		_ => None,
	}).collect::<Option<Vec<_>>>().filter(|arrays| !arrays.is_empty())
}

/// `gpu_compute(shader, xs, workgroups)` of a linear float array: its block `[count: i64][count f64 cells]`
/// (wasm_emitter/linear_arrays.rs) read and written in place, as f32 on the GPU
fn gpu_compute_linear(mut caller: Caller<'_, HostState>, shader: HostNode, block: i64, workgroups: i64) -> wasmtime::Result<i64> {
	let failure = gpu_failure(GPU_COMPUTE);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let floats = linear_floats(&mut caller, block, &failure)?;
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
	let left = crate::gpu::compute(&shader, &floats, workgroups).map_err(&failure)?;
	write_linear_floats(&mut caller, block, &left, &failure)?;
	Ok(block)
}

/// `ys = xs.map(x => …) @gpu` (src/lowering/gpu_maps.rs): the kernel over the cells of the block `source`, then the
/// values of the program's numbers it reads, written into the block `target` of the same count; 1, or 0 without an
/// adapter (a warning, once), when the program maps on the CPU. `keeping` (gpu_maps.rs KEEP_RESULT, SOURCE_KEPT): the
/// result's buffer stays on the GPU for a later map of it, the source's items come from such a buffer
fn gpu_map_linear(caller: Caller<'_, HostState>, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64) -> wasmtime::Result<i64> {
	gpu_kernel_linear(caller, GPU_MAP_LINEAR, shader, source, values, target, workgroups, keeping, false)
}

/// `s = sum(xs.map(x => …) @gpu)`, min, max: as gpu_map_linear, but each workgroup leaves its partial result in a cell
/// after the values, and those, the only ones read back, go into the block `target` of `workgroups` cells
fn gpu_reduce_linear(caller: Caller<'_, HostState>, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64) -> wasmtime::Result<i64> {
	gpu_kernel_linear(caller, GPU_REDUCE_LINEAR, shader, source, values, target, workgroups, keeping, true)
}

#[allow(clippy::too_many_arguments)]
fn gpu_kernel_linear(mut caller: Caller<'_, HostState>, word: &'static str, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64, reduces: bool) -> wasmtime::Result<i64> {
	static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
	let failure = gpu_failure(word);
	let automatic = keeping & crate::gpu_maps::AUTOMATIC != 0;
	if let Err(problem) = crate::gpu::available() {
		// a map the compiler switched says nothing: the program did not ask for the GPU
		if !automatic && !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
			crate::diagnostic::report_runtime_warning(&format!("@gpu: {problem}, so the map runs on the CPU"));
		}
		return Ok(0);
	}
	let shader = given_shader(&mut caller, shader, &failure)?;
	let kept = (keeping & crate::gpu_maps::SOURCE_KEPT != 0).then(|| crate::gpu::kept_count(source)).flatten();
	let mut floats = if kept.is_some() { vec![] } else { linear_floats(&mut caller, source, &failure)? };
	let items = kept.unwrap_or(floats.len());
	for value in given_node(&mut caller, values)?.iter() {
		match value.drop_meta() {
			Node::Number(number) => floats.push(f64::from(*number) as f32),
			other => return Err(failure(format!("the lambda reads {}, not a number", other.serialize()))),
		}
	}
	let partials = kept.unwrap_or(0) + floats.len();
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
	if reduces {
		floats.resize(floats.len() + workgroups as usize, 0.0);
	}
	let keeping = crate::gpu::Keeping { source: kept.map(|_| source), result: (keeping & crate::gpu_maps::KEEP_RESULT != 0).then_some((target, items)) };
	let left = crate::gpu::compute_kept(&shader, &floats, workgroups, if reduces { partials } else { 0 }, keeping).map_err(&failure)?;
	write_linear_floats(&mut caller, target, &left, &failure)?;
	if automatic {
		crate::diagnostic::report_runtime_warning_once(crate::gpu_maps::AUTOMATIC_NOTICE, &format!("{} of {items} items", if reduces { "a reduction" } else { "a map" }));
	}
	Ok(1)
}

const LINEAR_CELL_BYTES: usize = 8;

/// The byte range of the cells of a linear array's block `[count: i64][count f64 cells]` (wasm_emitter/linear_arrays.rs)
fn linear_cells(memory: &[u8], block: i64) -> Option<std::ops::Range<usize>> {
	let start = usize::try_from(block).ok()?;
	let count = i64::from_le_bytes(memory.get(start..start + LINEAR_CELL_BYTES)?.try_into().ok()?);
	let end = start + LINEAR_CELL_BYTES + usize::try_from(count).ok()? * LINEAR_CELL_BYTES;
	(end <= memory.len()).then_some(start + LINEAR_CELL_BYTES..end)
}

fn linear_memory(caller: &mut Caller<'_, HostState>, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<wasmtime::Memory> {
	match caller.get_export("memory") {
		Some(Extern::Memory(memory)) => Ok(memory),
		_ => Err(failure("the module exports no memory".into())),
	}
}

/// A linear float array's cells as f32, as the GPU takes them
fn linear_floats(caller: &mut Caller<'_, HostState>, block: i64, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<Vec<f32>> {
	let memory = linear_memory(caller, failure)?;
	let bytes = memory.data(&*caller);
	let cells = linear_cells(bytes, block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	Ok(bytes[cells].chunks_exact(LINEAR_CELL_BYTES).map(|cell| f64::from_le_bytes(cell.try_into().expect("8 bytes")) as f32).collect())
}

/// The f32 the GPU left, written into a linear float array's cells
fn write_linear_floats(caller: &mut Caller<'_, HostState>, block: i64, floats: &[f32], failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<()> {
	let memory = linear_memory(caller, failure)?;
	let cells = linear_cells(memory.data(&*caller), block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	let written = &mut memory.data_mut(caller)[cells];
	written.chunks_exact_mut(LINEAR_CELL_BYTES).zip(floats).for_each(|(cell, float)| cell.copy_from_slice(&f64::from(*float).to_le_bytes()));
	Ok(())
}

/// `gpu_render(shader, width, height)` through wgpu (src/gpu.rs): the pixels the fragment shader colored
fn gpu_render(mut caller: Caller<'_, HostState>, shader: HostNode, width: i64, height: i64, values: HostNode) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_RENDER);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let pixels: Vec<u32> = rendered(&mut caller, &shader, width, height, values, &failure)?.chunks_exact(4).map(crate::gpu::opaque_color).collect();
	if let Some(list) = list_of_ints(&mut caller, &pixels)? {
		return Ok(list);
	}
	let pixels = pixels.into_iter().map(|pixel| Node::Number(Number::Int(i64::from(pixel)))).collect();
	built_in_program(&mut caller, &Node::List(pixels, crate::node::Bracket::Square, crate::node::Separator::Space), GPU_RENDER)
}

/// `text_coverage(words, size)` through a system font (src/text_raster.rs): [width, height, coverage row by row]
fn text_coverage(mut caller: Caller<'_, HostState>, words: HostNode, size: i64) -> wasmtime::Result<HostNode> {
	let words = match given_node(&mut caller, words)? {
		Node::Text(text) => text,
		other => other.serialize(),
	};
	let covered = crate::text_raster::coverage(&words, size as f32).map_err(|problem| task_failure(format!("{TEXT_COVERAGE}: {problem}")))?;
	if let Some(list) = list_of_ints(&mut caller, &covered)? {
		return Ok(list);
	}
	let ints = covered.into_iter().map(|int| Node::Number(Number::Int(i64::from(int)))).collect();
	built_in_program(&mut caller, &Node::List(ints, crate::node::Bracket::Square, crate::node::Separator::Space), TEXT_COVERAGE)
}

/// The pixels the fragment shader colors through wgpu (src/gpu.rs), 0xFFRRGGBB row by row
fn rendered(caller: &mut Caller<'_, HostState>, shader: &str, width: i64, height: i64, values: HostNode, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<Vec<u8>> {
	let values = shader_values(&given_node(caller, values)?).map_err(failure)?;
	let side = |pixels: i64| u32::try_from(pixels).ok().filter(|&pixels| pixels > 0).ok_or_else(|| failure(format!("an image {width}×{height} pixels")));
	crate::gpu::render_rgba(shader, side(width)?, side(height)?, &values).map_err(failure)
}

/// The ints as a list built in one call of the module's ints_to_list (wasm_emitter/int_lists.rs); None when it has none
fn list_of_ints(caller: &mut Caller<'_, HostState>, ints: &[u32]) -> wasmtime::Result<Option<HostNode>> {
	use crate::wasm_emitter::int_lists::INTS_TO_LIST;
	let (Some(Extern::Func(ints_to_list)), Some(Extern::Memory(memory))) = (caller.get_export(INTS_TO_LIST), caller.get_export("memory")) else { return Ok(None) };
	let ints_to_list = ints_to_list.typed::<(i32, i32), HostNode>(&*caller)?;
	let Some(address) = scratch(caller, memory, std::mem::size_of_val(ints))? else { return Ok(None) };
	let bytes: Vec<u8> = ints.iter().flat_map(|int| int.to_le_bytes()).collect();
	memory.write(&mut *caller, address as usize, &bytes)?;
	Ok(Some(ints_to_list.call(&mut *caller, (address as i32, ints.len() as i32))?))
}

/// The module's memory, or the task failure "`word`: the module exports no memory"
fn module_memory(caller: &mut Caller<'_, HostState>, word: &str) -> wasmtime::Result<Memory> {
	match caller.get_export("memory") {
		Some(Extern::Memory(memory)) => Ok(memory),
		_ => Err(task_failure(format!("{word}: the module exports no memory"))),
	}
}

fn gpu_failure(word: &'static str) -> impl Fn(String) -> wasmtime::Error {
	move |problem| task_failure(format!("{word}: {problem}"))
}

/// gpu_render's values (none when left out): each entry a number, a list of numbers or a list of vectors
fn shader_values(values: &Node) -> Result<Vec<crate::gpu::ShaderValue>, String> {
	let float = |name: &str, value: &Node| match value.drop_meta() {
		Node::Number(number) => Ok(f64::from(*number) as f32),
		other => Err(format!("values.{name} is a number or a list of numbers, got {}", other.serialize())),
	};
	// a map of one entry arrives as that key
	let entries = match values.drop_meta() {
		Node::Empty => vec![],
		single @ Node::Key(..) => vec![single.clone()],
		map => map.iter().collect(),
	};
	entries.iter().map(|entry| {
		let Node::Key(name, _, value) = entry.drop_meta() else { return Err(format!("values is a map of names, got {}", entry.serialize())) };
		let name = name.drop_meta().name();
		let floats_of = |value: &Node| match value.drop_meta() {
			Node::List(items, _, _) => items.iter().map(|item| float(&name, item)).collect::<Result<Vec<f32>, String>>(),
			single => Ok(vec![float(&name, single)?]),
		};
		let items: Vec<Node> = match value.drop_meta() {
			Node::List(items, _, _) => items.clone(),
			_ => vec![],
		};
		// a list of lists: one vec4f per item, its missing coordinates 0
		if items.iter().any(|item| matches!(item.drop_meta(), Node::List(..))) {
			let mut floats = vec![];
			for item in &items {
				let mut vector = floats_of(item)?;
				if vector.len() > crate::gpu::VECTOR_FLOATS {
					return Err(format!("values.{name} is a list of vectors of up to four numbers, got {}", item.serialize()));
				}
				vector.resize(crate::gpu::VECTOR_FLOATS, 0.0);
				floats.extend(vector);
			}
			return Ok(crate::gpu::ShaderValue { name, floats, array: true });
		}
		let floats = floats_of(value)?;
		let array = floats.len() > crate::gpu::VECTOR_FLOATS;
		Ok(crate::gpu::ShaderValue { name, floats, array })
	}).collect()
}

/// The WGSL text of a gpu word's shader
fn given_shader(caller: &mut Caller<'_, HostState>, shader: HostNode, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<String> {
	match given_node(caller, shader)? {
		Node::Text(shader) => Ok(shader),
		Node::Char(letter) => Ok(letter.to_string()), // a one-letter text reads as a code point
		other => Err(failure(format!("the shader is a text of WGSL, got {}", other.serialize()))),
	}
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

