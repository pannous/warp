//! The host's native side: wasmtime host functions, linear-memory helpers and the linker (feature native)

use super::*;

mod gpu;
use gpu::*;
pub(crate) use gpu::{linear_doubles, module_memory, number_of}; // the sound module's host words call them too
use crate::sound::host_words::*;

/// A program value as a host word gets and gives it: a reference to a Node of the instance
pub(crate) type HostNode = Option<wasmtime::Rooted<wasmtime::AnyRef>>;

const PAGE_BITS: u32 = 16;
const I64_BYTES: usize = 8;

/// guarded_call(name, arguments): the node wrapper `name` called with the argument list in the caller's own instance;
/// a stack overflow inside is the Error "call stack exhausted" (what the aborted call wrote to globals and memory stays),
/// any other failure, running out of fuel included, ends the run as before
fn guarded_call(mut caller: Caller<'_, HostState>, name: i32, arguments: HostNode) -> wasmtime::Result<HostNode> {
	let message = |failure: anyhow::Error| wasmtime::Error::msg(failure.to_string());
	let name = crate::host::tasks::c_string(&mut caller, name).map_err(message)?;
	let function = caller
		.get_export(&name)
		.and_then(Extern::into_func)
		.ok_or_else(|| wasmtime::Error::msg(format!("no exported function {name}")))?;
	// the wrapper gives back its function's result: a Node, an Int or a Float
	let mut result = [match function.ty(&caller).results().next() {
		Some(wasmtime::ValType::I64) => Val::I64(0),
		Some(wasmtime::ValType::F64) => Val::F64(0),
		_ => Val::AnyRef(None),
	}];
	let value = match function.call(&mut caller, &[Val::AnyRef(arguments)], &mut result) {
		Ok(()) => match result[0] {
			Val::I64(n) => crate::host::tasks::TaskValue::Int(n),
			Val::F64(bits) => {
				crate::host::tasks::TaskValue::Float(warp_runtime::floats::canonical_nan(f64::from_bits(bits)))
			}
			node => return Ok(node.unwrap_anyref().copied()),
		},
		Err(failure) if failure.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::StackOverflow) => {
			return error_in_program(&mut caller, STACK_EXHAUSTED)
		}
		Err(failure) => return Err(failure),
	};
	build_in_program(&mut caller, &value, &wasmtime::Error::msg)
}

/// The Error of `reason`, built with the module's own error_of: a failure the program's `try` catches
fn error_in_program(caller: &mut Caller<'_, HostState>, reason: &str) -> wasmtime::Result<HostNode> {
	let text =
		built_in_program(caller, &Node::Text(reason.to_string()), crate::wasm_emitter::text::text_builtins::ERROR_OF)?;
	let error_of = caller
		.get_export(crate::wasm_emitter::text::text_builtins::ERROR_OF)
		.and_then(Extern::into_func)
		.ok_or_else(|| wasmtime::Error::msg("no exported error_of"))?;
	let mut error = [Val::AnyRef(None)];
	error_of.call(&mut *caller, &[Val::AnyRef(text)], &mut error)?;
	Ok(error[0].unwrap_anyref().copied())
}

use crate::host::tasks::task_failure;
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
	/// The events the program dispatched to its page (dispatch_event): name and detail as warp text, for the headless page
	pub dispatched: Vec<(String, String)>,
	/// The headers the route being answered set (response_header), for its HTTP answer
	pub answer_headers: Vec<(String, String)>,
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
			dispatched: vec![],
			answer_headers: vec![],
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
pub fn read_string_from_memory(
	memory: &Memory,
	store: &impl wasmtime::AsContext,
	ptr: u32,
	len: u32,
) -> Result<String> {
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
pub fn write_bytes(
	memory: &Memory,
	heap: Option<wasmtime::Global>,
	store: &mut wasmtime::StoreContextMut<'_, HostState>,
	bytes: &[u8],
) -> Result<(u32, u32)> {
	match heap {
		Some(heap) => Ok(write_to_heap(memory, heap, store, bytes)?),
		None => {
			let ptr = store.data_mut().alloc(bytes.len() as u32);
			Ok(write_at(memory, store, ptr, bytes)?)
		}
	}
}

pub use warp_runtime::gc_values::{write_at, write_to_heap};

/// The calling module's linear memory, if it exports one (host words, tasks and foreign calls read texts out of it)
pub(crate) fn exported_memory<T>(caller: &mut Caller<'_, T>) -> Option<Memory> {
	caller.get_export(crate::wasm_emitter::MEMORY_ENTITY)?.into_memory()
}

/// The file at the path in WASM memory, written back like a fetched body: (ptr, len), or (ptr, -len) of the failure reason
fn read_into_memory(caller: &mut Caller<'_, HostState>, path_ptr: i32, path_len: i32) -> (i32, i32) {
	answer_for_text(caller, (path_ptr, path_len), "read", "path", |path| {
		std::fs::read(crate::util::home_expanded(path)).map_err(|reason| format!("read {path} failed: {reason}"))
	})
}

/// The answer to the text at (ptr, len) in WASM memory (a path, a URL: `argument`), written back into memory as
/// answer_in_memory writes it; an unreadable text is the failure "`word` failed: unreadable `argument`"
fn answer_for_text(
	caller: &mut Caller<'_, HostState>,
	(ptr, len): (i32, i32),
	word: &str,
	argument: &str,
	answer: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> (i32, i32) {
	let Some(memory) = exported_memory(caller) else {
		trace!("host.{word}: no memory export");
		return (0, 0);
	};
	let answered = read_string_from_memory(&memory, &*caller, ptr as u32, len as u32)
		.map_err(|e| format!("{word} failed: unreadable {argument}: {e}"))
		.and_then(|text| answer(&text));
	match answered {
		Ok(bytes) => answer_in_memory(&memory, caller, &bytes, false, word),
		Err(reason) => answer_in_memory(&memory, caller, reason.as_bytes(), true, word),
	}
}

/// The answer written into the module's memory: (ptr, len), or (ptr, -len) of a failure's reason; (0, 0) if it could
/// not be written
fn answer_in_memory(
	memory: &Memory,
	caller: &mut Caller<'_, HostState>,
	bytes: &[u8],
	failed: bool,
	word: &str,
) -> (i32, i32) {
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
	let mut store = crate::util::limited_store(&engine, ());
	let module = crate::run::module_cache::compiled_module(&engine, bytes)?;
	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module)?;

	let main = instance.get_func(&mut store, "main").ok_or_else(|| anyhow!("No main function"))?;

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
	answer_for_text(caller, (url_ptr, url_len), "fetch", "URL", |url| {
		trace!("host.fetch: fetching {} (timeout {:?})", url, timeout);
		fetch(url, timeout).map(String::into_bytes)
	})
}

/// Link host functions into a wasmtime Linker
pub fn link_host_functions(linker: &mut Linker<HostState>, _engine: &Engine) -> Result<()> {
	warp_runtime::system_values::tell_gpu_painter(crate::graphics::paint::paints_shaders);
	link_memory_words(linker)?;
	link_library_words(linker)
}

/// The words of the module `host` that pass texts and bytes through the instance's linear memory
fn link_memory_words(linker: &mut Linker<HostState>) -> Result<()> {
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
	// host.warn(message_ptr: i32, message_len: i32): a runtime warning, reported and collected
	linker.func_wrap("host", "warn", warn)?;
	// host.run(wasm_ptr: i32, wasm_len: i32) -> i64
	linker.func_wrap("host", "run", run_wasm_bytes)?;
	Ok(())
}

/// The words of HOST_LIBRARY: blocks, foreign and std calls, serving and fetching, channels, the GPU, painting, sound
fn link_library_words(linker: &mut Linker<HostState>) -> Result<()> {
	linker.func_wrap(HOST_LIBRARY, GUARDED_CALL, guarded_call)?;
	warp_runtime::host_words::link_host_words(linker)?;
	linker.func_wrap(HOST_LIBRARY, RUN_BLOCK, run_block)?;
	linker.func_wrap(HOST_LIBRARY, PARSE_CODE, parse_code)?;
	linker.func_wrap(HOST_LIBRARY, BLOCK_VALUE, block_value)?;
	linker.func_wrap(HOST_LIBRARY, FOREIGN_CALL, foreign_call)?;
	linker.func_wrap(HOST_LIBRARY, STD_PURE, std_call)?;
	linker.func_wrap(HOST_LIBRARY, STD_IO, std_call)?;
	linker.func_wrap(HOST_LIBRARY, SERVE_ROUTES, serve_routes)?;
	linker.func_wrap(HOST_LIBRARY, RESPONSE_HEADER, response_header)?;
	linker.func_wrap(HOST_LIBRARY, FETCH_START, fetch_start)?;
	linker.func_wrap(HOST_LIBRARY, FETCH_REPLY, fetch_reply)?;
	crate::host::channels::forget_listeners(); // each run links anew, on its own thread
	linker.func_wrap(HOST_LIBRARY, CHANNEL_LISTEN, channel_listen)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_PENDING, crate::host::channels::pending)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_NEXT, channel_next)?;
	linker.func_wrap(HOST_LIBRARY, CLIPBOARD_TEXT, clipboard_text)?;
	linker.func_wrap(HOST_LIBRARY, PAGE_PATH, page_path)?;
	linker.func_wrap(HOST_LIBRARY, NOTIFY, notify)?;
	linker.func_wrap(HOST_LIBRARY, GPU_COMPUTE, gpu_compute)?;
	linker.func_wrap(HOST_LIBRARY, GPU_RENDER, gpu_render)?;
	linker.func_wrap(HOST_LIBRARY, TEXT_COVERAGE, text_coverage)?;
	linker.func_wrap(HOST_LIBRARY, GPU_COMPUTE_LINEAR, gpu_compute_linear)?;
	linker.func_wrap(HOST_LIBRARY, GPU_MAP_LINEAR, gpu_kernel_linear::<false>)?;
	linker.func_wrap(HOST_LIBRARY, GPU_REDUCE_LINEAR, gpu_kernel_linear::<true>)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_SEND, channel_send)?;
	linker.func_wrap(HOST_LIBRARY, DISPATCH_EVENT, dispatch_event)?;
	linker.func_wrap(HOST_LIBRARY, PAINT, paint)?;
	linker.func_wrap(HOST_LIBRARY, DRAW_SHAPE, |kind: i64, a: f64, b: f64, c: f64, d: f64, color: i64| {
		crate::graphics::gpu_canvas::add_shape(kind as u32, [a, b, c, d], color as u32)
	})?;
	linker.func_wrap(HOST_LIBRARY, DRAW_SHOW, draw_show)?;
	linker.func_wrap(HOST_LIBRARY, DRAW_SHADER, |mut caller: Caller<'_, HostState>, code: HostNode| {
		let code = given_node(&mut caller, code)?;
		let shader = code
			.text_value()
			.ok_or_else(|| task_failure(format!("{DRAW_SHADER} takes a text, got {}", code.serialize())))?;
		crate::graphics::gpu_canvas::use_shader(shader);
		Ok(())
	})?;
	linker.func_wrap(HOST_LIBRARY, SAVE_PICTURE, save_picture)?;
	linker.func_wrap(HOST_LIBRARY, SOUND, sound)?;
	Ok(())
}

fn warn(mut caller: Caller<'_, HostState>, message_ptr: i32, message_len: i32) {
	if let Some(memory) = exported_memory(&mut caller) {
		match read_string_from_memory(&memory, &caller, message_ptr as u32, message_len as u32) {
			Ok(message) => crate::errors::diagnostic::report_runtime_warning(&message),
			Err(e) => trace!("host.warn: unreadable message: {e}"),
		}
	}
}

/// host.run: the wasm bytes in memory run as a module of their own, its main's i64 (-1 when it fails)
fn run_wasm_bytes(mut caller: Caller<'_, HostState>, wasm_ptr: i32, wasm_len: i32) -> i64 {
	let Some(memory) = exported_memory(&mut caller) else {
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
}

/// paint(pixels, width, height) natively: the pixels as a PNG (src/graphics/paint.rs); paint(shader, width, height, values)
/// renders the WGSL fragment shader first, as gpu_render does (P234)
fn paint(
	mut caller: Caller<'_, HostState>,
	pixels: HostNode,
	width: i64,
	height: i64,
	shader_values: HostNode,
) -> wasmtime::Result<()> {
	let result = paint_or_failure(&mut caller, pixels, width, height, shader_values);
	raised(&mut caller, result)
}

fn paint_or_failure(
	caller: &mut Caller<'_, HostState>,
	pixels: HostNode,
	width: i64,
	height: i64,
	shader_values: HostNode,
) -> wasmtime::Result<()> {
	warp_runtime::system_signals::end_at_first_frame()?;
	let memory = module_memory(caller, "paint")?;
	let size = (width.max(0) as usize, height.max(0) as usize);
	let values = match list_values(caller, memory, pixels, size.0 * size.1)? {
		Ok(values) => values,
		Err(crate::node::Node::Text(shader)) => {
			let failure = gpu_failure(PAINT);
			let (width, height, values) = shader_request(caller, width, height, shader_values, &failure)?;
			return crate::graphics::paint::paint_shader(&shader, width, height, &values).map(|_| ()).map_err(failure);
		}
		Err(other) => {
			return Err(task_failure(format!(
				"paint needs a list of pixels or a shader text, got {}",
				other.serialize()
			)))
		}
	};
	crate::graphics::paint::paint(&values, size.0, size.1).map(|_| ()).map_err(task_failure)
}

/// save_picture(pixels, width, height, file) (card save-painting): `use draw`'s canvas as a PNG at that path, its pixels
/// as paint takes them, or, drawn on the GPU (no pixels then), the GPU canvas as drawn so far
fn save_picture(
	mut caller: Caller<'_, HostState>,
	pixels: HostNode,
	width: i64,
	height: i64,
	file: HostNode,
) -> wasmtime::Result<()> {
	let result = save_picture_or_failure(&mut caller, pixels, width, height, file);
	raised(&mut caller, result)
}

fn save_picture_or_failure(
	caller: &mut Caller<'_, HostState>,
	pixels: HostNode,
	width: i64,
	height: i64,
	file: HostNode,
) -> wasmtime::Result<()> {
	let path = match given_node(caller, file)? {
		Node::Text(path) => path,
		other => return Err(task_failure(format!("save: the file is a text, got {}", other.serialize()))),
	};
	let memory = module_memory(caller, SAVE_PICTURE)?;
	let size = (width.max(0) as usize, height.max(0) as usize);
	let on_gpu = warp_runtime::system_values::read(warp_runtime::host_words::GPU_DRAWING) == Ok(1);
	let saved = match list_values(caller, memory, pixels, size.0 * size.1)? {
		_ if on_gpu => crate::graphics::gpu_canvas::snapshot(size.0, size.1)
			.and_then(|rgba| crate::graphics::paint::save_rgba_png(&rgba, size.0, size.1, &path)),
		Ok(values) => crate::graphics::paint::save_png(&values, size.0, size.1, &path),
		Err(other) => Err(format!("save: a picture's pixels are a list, got {}", other.serialize())),
	};
	saved.map_err(task_failure)
}

/// show() of `use draw` on the GPU (src/graphics/gpu_canvas.rs)
fn draw_show(width: i64, height: i64) -> wasmtime::Result<()> {
	warp_runtime::system_signals::end_at_first_frame()?;
	crate::graphics::gpu_canvas::show(width.max(0) as usize, height.max(0) as usize).map(|_| ()).map_err(task_failure)
}

/// The first `room` values of a list, read in one call by ints_of_list or else node by node; the value itself when it is
/// no list
pub(crate) fn list_values(
	caller: &mut Caller<'_, HostState>,
	memory: Memory,
	list: HostNode,
	room: usize,
) -> wasmtime::Result<Result<Vec<u64>, crate::node::Node>> {
	if let Some(values) = ints_of_list(caller, memory, list, room)? {
		return Ok(Ok(values));
	}
	Ok(match crate::run::wasm_reader::node_in(&Val::AnyRef(list), &mut caller.as_context_mut(), memory).drop_meta() {
		crate::node::Node::List(items, _, _) => Ok(items.iter().map(pixel_value).collect()),
		other => Err(other.clone()),
	})
}

/// The first `room` items of a list of fixnum Ints read in one call of the module's list_to_ints (wasm_emitter/int_lists.rs), their magnitudes;
/// None for any other value, which the caller reads node by node
fn ints_of_list(
	caller: &mut Caller<'_, HostState>,
	memory: Memory,
	list: HostNode,
	room: usize,
) -> wasmtime::Result<Option<Vec<u64>>> {
	use crate::wasm_emitter::lists::int_lists::LIST_TO_INTS;
	let Some(Extern::Func(list_to_ints)) = caller.get_export(LIST_TO_INTS) else {
		return Ok(None);
	};
	let list_to_ints = list_to_ints.typed::<(HostNode, i32, i32), i32>(&*caller)?;
	let Some(address) = scratch(caller, memory, room * I64_BYTES)? else {
		return Ok(None);
	};
	let written = list_to_ints.call(&mut *caller, (list, address as i32, room as i32))?;
	let Ok(written) = usize::try_from(written) else {
		return Ok(None);
	};
	let mut bytes = vec![0; written * I64_BYTES];
	memory.read(&*caller, address as usize, &mut bytes)?;
	Ok(Some(bytes.as_chunks::<I64_BYTES>().0.iter().map(|value| u64::from_le_bytes(*value)).collect()))
}

/// Free memory for `bytes` bytes above the module's text heap, which the next text may take again (int_lists.rs), at
/// an i64's alignment: the heap starts past the memory when unset, as write_to_heap starts it; None without a text heap
fn scratch(caller: &mut Caller<'_, HostState>, memory: Memory, bytes: usize) -> wasmtime::Result<Option<u32>> {
	let Some(Extern::Global(heap)) = caller.get_export(TEXT_HEAP_EXPORT) else {
		return Ok(None);
	};
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
fn run_block(
	mut caller: Caller<'_, HostState>,
	block: HostNode,
	names: HostNode,
	values: HostNode,
	definitions: HostNode,
) -> wasmtime::Result<HostNode> {
	let result = run_block_or_failure(&mut caller, block, names, values, definitions);
	raised(&mut caller, result)
}

fn run_block_or_failure(
	caller: &mut Caller<'_, HostState>,
	block: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	names: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	values: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	definitions: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::host::tasks::TaskValue;
	let memory = module_memory(caller, "run_block")?;
	let mut store = caller.as_context_mut();
	let [block, names, values, definitions] = [block, names, values, definitions]
		.map(|value| crate::run::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let result =
		crate::compiler::run_time_blocks::eval_block(block, &names, &values, &definitions).map_err(task_failure)?;
	let value = TaskValue::of(&result)
		.map_err(|_| task_failure(crate::compiler::run_time_blocks::cannot_hand_back(&result)))?;
	build_in_program(caller, &value, &task_failure)
}

/// The host side of parse_code: the code of a text, or the parse failure the program's `try` catches
fn parse_code(mut caller: Caller<'_, HostState>, text: HostNode) -> wasmtime::Result<HostNode> {
	let text = given_node(&mut caller, text)?;
	let Some(source) = text.text_value() else {
		return error_in_program(&mut caller, &format!("{PARSE_CODE}: needs a text, got {}", text.serialize().trim()));
	};
	match crate::lowering::parse_word::code_of(&source) {
		Ok(code) => built_in_program(&mut caller, &code, PARSE_CODE),
		Err(failure) => error_in_program(&mut caller, &failure),
	}
}

thread_local! {
	/// The values of the blocks running on this thread, the innermost last (a block may run a block)
	static BLOCK_VALUES: std::cell::RefCell<Vec<Vec<crate::host::tasks::TaskValue>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `body` while block·value reads `values`
pub fn with_block_values<R>(values: Vec<crate::host::tasks::TaskValue>, body: impl FnOnce() -> R) -> R {
	BLOCK_VALUES.with(|stack| stack.borrow_mut().push(values));
	let result = body();
	BLOCK_VALUES.with(|stack| stack.borrow_mut().pop());
	result
}

/// The host side of block·value: the value at `index`, built in the block's module
fn block_value(mut caller: Caller<'_, HostState>, index: i64) -> wasmtime::Result<HostNode> {
	let failure = |message: String| wasmtime::Error::msg(message);
	let value = BLOCK_VALUES
		.with(|stack| stack.borrow().last().and_then(|values| values.get(index as usize).cloned()))
		.ok_or_else(|| failure(format!("{BLOCK_VALUE}({index}): no such value of the running block")))?;
	build_in_program(&mut caller, &value, &failure)
}

/// The host side of foreign_call: the runtime, module and member names and the arguments come in as Nodes, the
/// answer goes back as one (src/modules/foreign.rs)
fn foreign_call(
	mut caller: Caller<'_, HostState>,
	runtime: HostNode,
	module: HostNode,
	member: HostNode,
	call: HostNode,
	arguments: HostNode,
) -> wasmtime::Result<HostNode> {
	let result = foreign_call_or_failure(&mut caller, runtime, module, member, call, arguments);
	raised(&mut caller, result)
}

fn foreign_call_or_failure(
	caller: &mut Caller<'_, HostState>,
	runtime: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	module: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	member: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	call: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	arguments: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::host::tasks::{Builders, TaskValue};
	let memory = module_memory(caller, "foreign_call")?;
	let mut store = caller.as_context_mut();
	let functions = function_arguments(&mut store, arguments)?;
	let [runtime, module, member, call, arguments] = [runtime, module, member, call, arguments]
		.map(|value| crate::run::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let arguments = with_function_markers(arguments, &functions);
	let builders =
		Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| task_failure(problem.to_string()))?;
	let mut callback = |index: usize, values: Node| -> Result<Node, String> {
		let Some(Extern::Func(apply)) = caller.get_export(crate::wasm_emitter::CLOSURE_APPLY) else {
			return Err("the module exports no closure_apply".into());
		};
		let function = functions.iter().flatten().nth(index).copied().ok_or(format!("no warp function {index}"))?;
		let values = TaskValue::of(&values)
			.and_then(|values| builders.build(&values, &mut caller.as_context_mut()))
			.map_err(|problem| problem.to_string())?;
		let mut result = [Val::AnyRef(None)];
		apply
			.call(&mut *caller, &[Val::AnyRef(Some(function)), values], &mut result)
			.map_err(|trap| raised_message(caller).unwrap_or_else(|| trap.root_cause().to_string()))?;
		given_node(caller, result[0].unwrap_anyref().copied()).map_err(|problem| problem.to_string())
	};
	let answer =
		crate::modules::foreign::call(&runtime.name(), &module, &member.name(), call == 1, &arguments, &mut callback);
	let answer = match answer {
		Ok(answer) => answer,
		// raised in the program (emit_ffi_result), as a failed std word's: `try` catches a crate's Err too
		Err(problem) => return error_in_program(caller, &problem),
	};
	let value = TaskValue::of(&answer).map_err(|problem| task_failure(problem.to_string()))?;
	build_in_program(caller, &value, &task_failure)
}

/// A foreign call's arguments, the list's items or the one argument, each a closure or None
fn function_arguments(
	store: &mut wasmtime::StoreContextMut<'_, HostState>,
	arguments: HostNode,
) -> wasmtime::Result<Vec<HostNode>> {
	use crate::run::wasm_reader::{FIELD_DATA, FIELD_KIND, FIELD_VALUE};
	use crate::type_kinds::Kind;
	let node_of = |value: &Val, store: &wasmtime::StoreContextMut<'_, HostState>| {
		value.unwrap_anyref().and_then(|reference| reference.unwrap_struct(store).ok())
	};
	let kind_of = |node: &wasmtime::StructRef, store: &mut wasmtime::StoreContextMut<'_, HostState>| {
		node.field(&mut *store, FIELD_KIND).map(|kind| Kind::of_field(kind.unwrap_i64()))
	};
	let mut items = vec![];
	let mut cell = node_of(&Val::AnyRef(arguments), store);
	match &cell {
		Some(node) if kind_of(node, store)? == Some(Kind::List) => {}
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
	items
		.iter()
		.map(|item| {
			Ok(match node_of(item, store) {
				Some(node) if kind_of(&node, store)? == Some(Kind::Function) => item.unwrap_anyref().copied(),
				_ => None,
			})
		})
		.collect()
}

/// The arguments with each closure `{$warp_function: index}`, its index among the closures (crate::modules::foreign)
fn with_function_markers(arguments: Node, functions: &[HostNode]) -> Node {
	if functions.iter().all(Option::is_none) {
		return arguments;
	}
	let marker = |index: usize| {
		Node::List(
			vec![Node::key(crate::modules::foreign::WARP_FUNCTION_KEY, Node::int(index as i64))],
			crate::node::Bracket::Curly,
			crate::node::Separator::Space,
		)
	};
	let mut index = 0;
	let mut marked = |item: Node, function: &Option<_>| match function {
		Some(_) => (marker(index), index += 1).0,
		None => item,
	};
	match arguments.drop_meta().clone() {
		Node::List(items, bracket, separator) if items.len() == functions.len() => Node::List(
			items.into_iter().zip(functions).map(|(item, function)| marked(item, function)).collect(),
			bracket,
			separator,
		),
		single => marked(single, &functions[0]),
	}
}

/// serve_routes(port, routes): HTTP on the port, each request answered by its route's function (src/web/web_server.rs)
fn serve_routes(mut caller: Caller<'_, HostState>, port: i64, routes: HostNode) -> wasmtime::Result<HostNode> {
	use crate::web::web_server::{routes_of, serve};
	let routes = routes_of(&given_node(&mut caller, routes)?);
	let port = u16::try_from(port).map_err(|_| task_failure(format!("serve {port}: no port number")))?;
	let site = served_site(port);
	serve(port, &routes, &site, |route, request| {
		caller.data_mut().answer_headers.clear();
		match route_value(&mut caller, &route.function, &request) {
			Ok(value) => {
				Ok(route.answer_of(&value).with_headers(std::mem::take(&mut caller.data_mut().answer_headers)))
			}
			Err(failure) => Err(route_failure(&mut caller, &route.path, failure)),
		}
	})
	.map_err(task_failure)?;
	built_in_program(&mut caller, &Node::Empty, SERVE_ROUTES)
}

/// `header "name" "value"` in a route's body: kept for the route's HTTP answer (serve_routes)
fn response_header(mut caller: Caller<'_, HostState>, name: HostNode, value: HostNode) -> wasmtime::Result<()> {
	let name = given_node(&mut caller, name)?.name();
	let value = text_or_written(given_node(&mut caller, value)?);
	caller.data_mut().answer_headers.push((name, value));
	Ok(())
}

/// A host word's failure as the program's own error (card wgsl-error): in a program with a `try`, its message goes to
/// trap_detail and the module's returned_error raises it, which a running `try` catches (`catch e` binds the message)
/// and which otherwise traps as `raise` does; any other failure, or a module without a `try`, stays as it was
pub(super) fn raised<T>(caller: &mut Caller<'_, HostState>, result: wasmtime::Result<T>) -> wasmtime::Result<T> {
	use crate::wasm_emitter::lists::list_ops::RETURNED_ERROR;
	let failure = match result {
		Err(failure) if failure.is::<crate::host::tasks::TaskFailure>() => failure,
		done => return done,
	};
	let exports = (caller.get_export(RETURNED_ERROR), caller.get_export(crate::run::wasm_reader::TRAP_DETAIL));
	let (Some(Extern::Func(returned_error)), Some(Extern::Global(detail))) = exports else {
		return Err(failure);
	};
	let message = Node::Text(failure.to_string());
	let raise = |caller: &mut Caller<'_, HostState>| -> wasmtime::Result<()> {
		let message = built_in_program(caller, &message, RETURNED_ERROR)?;
		detail.set(&mut *caller, Val::AnyRef(message))?;
		returned_error.typed::<(), ()>(&*caller)?.call(&mut *caller, ())
	};
	Err(raise(caller).err().unwrap_or(failure))
}

/// The message of the Error a trapped call raised (left in trap_detail), read once: a warp function called back from
/// another runtime fails with its own words, not the engine's `unreachable`
fn raised_message(caller: &mut Caller<'_, HostState>) -> Option<String> {
	let detail = taken_trap_detail(caller)?;
	match given_node(caller, Some(detail)).ok()? {
		Node::Error(reason) => Some(reason.drop_meta().name()),
		_ => None,
	}
}

/// The Error value a trapped call left in trap_detail, taken: the next failure must not show this one's detail
fn taken_trap_detail(caller: &mut Caller<'_, HostState>) -> Option<wasmtime::Rooted<wasmtime::AnyRef>> {
	let detail = caller.get_export(crate::run::wasm_reader::TRAP_DETAIL).and_then(Extern::into_global)?;
	let left = detail.get(&mut *caller);
	let _ = detail.set(&mut *caller, Val::AnyRef(None));
	left.unwrap_anyref().copied()
}

/// What a failed route says to the client: the program's error message, read with the values the program left in
/// trap_detail and, for a runtime error, trap_position (as wasm_reader::with_trap_detail does for main); the whole trace goes to the server's log
fn route_failure(
	caller: &mut Caller<'_, HostState>,
	path: &str,
	failure: wasmtime::Error,
) -> crate::web::web_server::RouteFailure {
	let left = taken_trap_detail(caller);
	let failure = anyhow::Error::from(failure);
	let raised = format!("{failure:?}").contains(crate::wasm_emitter::lists::list_ops::RETURNED_ERROR);
	// a runtime error names its statement; what the program raised is its own words to the client
	let failure = match raised {
		true => failure,
		false => {
			let position = caller
				.get_export(crate::run::wasm_reader::TRAP_POSITION)
				.and_then(Extern::into_global)
				.map(|global| global.get(&mut *caller));
			crate::run::wasm_reader::with_trap_position(failure, position)
		}
	};
	let failure = match left {
		Some(value) => match given_node(caller, Some(value)) {
			Ok(node) => failure.context(crate::run::wasm_reader::trap_detail_line(&node)),
			Err(_) => failure,
		},
		_ => failure,
	};
	let trace = format!("{failure:?}");
	let message = crate::host::tasks::failure_message(failure);
	eprintln!("{path} failed: {message}\n{trace}");
	crate::web::web_server::RouteFailure { message, raised }
}

/// The site of the program file being run, which a program whose last line shows a page serves at / (src/web/site.rs); a
/// page that fails to build is said, and the routes are served without it
fn served_site(port: u16) -> crate::web::site::ServedSite {
	let Some(file) = crate::modules::program_file() else {
		return Default::default();
	};
	let title = crate::web::site::program_stem(&file);
	let rendered = std::fs::read_to_string(&file)
		.map_err(|failure| failure.to_string())
		.and_then(|code| crate::web::site::served_files(&code, &title));
	rendered
		.unwrap_or_else(|failure| {
			eprintln!("warning: serve {port} serves no page: {failure}");
			None
		})
		.unwrap_or_default()
}

/// The value of a route's function called with the request (if it takes one), as a Node
fn route_value(caller: &mut Caller<'_, HostState>, function: &str, request: &Node) -> wasmtime::Result<Node> {
	let Some(Extern::Func(route)) = caller.get_export(function) else {
		return Err(wasmtime::Error::msg(format!("no route function {function}")));
	};
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

/// std_pure / std_io(module, member, arguments): a word of the standard library's adapters (src/modules/std_adapters.rs)
fn std_call(
	mut caller: Caller<'_, HostState>,
	module: HostNode,
	member: HostNode,
	arguments: HostNode,
) -> wasmtime::Result<HostNode> {
	let result = std_call_or_failure(&mut caller, module, member, arguments);
	raised(&mut caller, result)
}

fn std_call_or_failure(
	caller: &mut Caller<'_, HostState>,
	module: HostNode,
	member: HostNode,
	arguments: HostNode,
) -> wasmtime::Result<HostNode> {
	let [module, member, arguments] = [module, member, arguments].map(|value| given_node(caller, value));
	let (module, member) = (module?.name(), member?.name());
	let answer = match (module.as_str(), member.as_str()) {
		("table", "select") => queried_rows(caller, &arguments?)?,
		("sound", "signal") => played_signal(caller, &arguments?)?,
		("sound", "blocks") => played_blocks(caller, &arguments?)?,
		_ => match crate::modules::std_adapters::call(&module, &member, &arguments?) {
			Ok(answer) => answer,
			// the program raises it (emit_ffi_result): `try` catches it, uncaught it ends the run with this reason
			Err(problem) => return error_in_program(caller, &problem),
		},
	};
	built_in_program(caller, &answer, &format!("{module}.{member}"))
}

/// The ids a filter's query keeps (database.rs select), its warp_call calling the program's functions back; a function
/// that fails fails the query with its own error (the trap, whose trap_detail says what went wrong)
fn queried_rows(caller: &mut Caller<'_, HostState>, arguments: &Node) -> wasmtime::Result<Node> {
	calling_back("table.select", |kept| {
		crate::host::database::select(&arguments.children(), &mut |function, request| {
			route_value(caller, function, request).map_err(&mut *kept)
		})
	})
}

/// What a host word computes calling the program back: a call that fails fails the word with its own error (the trap,
/// whose trap_detail says what went wrong), any other failure is the word's
pub(crate) fn calling_back<T>(
	word: &str,
	compute: impl FnOnce(&mut dyn FnMut(wasmtime::Error) -> String) -> Result<T, String>,
) -> wasmtime::Result<T> {
	let mut trapped = None;
	let computed = compute(&mut |trap| {
		let problem = format!("{trap:#}");
		trapped.get_or_insert(trap);
		problem
	});
	match (computed, trapped) {
		(Ok(computed), _) => Ok(computed),
		(Err(_), Some(trap)) => Err(trap),
		(Err(problem), None) => Err(task_failure(format!("{word}: {problem}"))),
	}
}

/// The node a host word was given, read out of the caller's instance
fn given_node(caller: &mut Caller<'_, HostState>, value: HostNode) -> wasmtime::Result<Node> {
	let memory = exported_memory(caller).ok_or_else(|| wasmtime::Error::msg("the module exports no memory"))?;
	Ok(crate::run::wasm_reader::node_in(&Val::AnyRef(value), &mut caller.as_context_mut(), memory))
}

/// `on message from "chat" {…}` starts listening (src/host/channels.rs)
fn channel_listen(mut caller: Caller<'_, HostState>, id: i64, channel: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	crate::host::channels::listen(id, &channel.name()).map_err(task_failure)
}

/// A fetch's URL, or `[url, body]`: a POST of the body as JSON (a server function the page calls, lowering/web/serve.rs)
fn request_of(request: &Node) -> crate::web::fetches::Request {
	match request.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => {
			(items[0].name(), Some(crate::modules::foreign::json_of(&items[1]).to_string()))
		}
		url => (url.name(), None),
	}
}

/// `users := fetch url` starts the fetch (src/web/fetches.rs)
fn fetch_start(mut caller: Caller<'_, HostState>, id: i64, url: HostNode) -> wasmtime::Result<()> {
	let request = given_node(&mut caller, url)?;
	crate::web::fetches::start(id, request_of(&request), FETCH_TIMEOUT);
	Ok(())
}

/// [value, error] of the fetch's reply (src/web/fetches.rs)
fn fetch_reply(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	built_in_program(&mut caller, &crate::web::fetches::reply(id), "fetch")
}

/// The oldest message the listener got, as the value it was sent as (ø when none waits)
fn channel_next(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	let message = crate::web::web_sockets::next(id).unwrap_or_else(|| {
		crate::host::channels::next(id)
			.map(|text| crate::warp_parser::parse(&text).drop_meta().clone())
			.unwrap_or(Node::Empty)
	});
	built_in_program(&mut caller, &message, "on message")
}

/// `clipboard`: the clipboard's text, read now (warp-runtime system_values.rs)
fn clipboard_text(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let result = clipboard_text_or_failure(&mut caller);
	raised(&mut caller, result)
}

fn clipboard_text_or_failure(caller: &mut Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let text = warp_runtime::system_values::clipboard_text().map_err(task_failure)?;
	built_in_program(caller, &Node::Text(text), "clipboard")
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

/// Run `body` with the page at `path` (a page rendered for a request, src/web/site.rs)
pub fn with_page_path<R>(path: &str, body: impl FnOnce() -> R) -> R {
	crate::util::scoped(&NATIVE_PAGE_PATH, path.to_string(), body)
}

/// The value built in the program by its exported constructors
fn build_in_program(
	caller: &mut Caller<'_, HostState>,
	value: &crate::host::tasks::TaskValue,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<HostNode> {
	let builders = crate::host::tasks::Builders::of(&mut |export| caller.get_export(export))
		.map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// A value of the host as a value of the program, built by its exported constructors
fn built_in_program(caller: &mut Caller<'_, HostState>, value: &Node, word: &str) -> wasmtime::Result<HostNode> {
	use crate::host::tasks::TaskValue;
	let failure = |problem: String| task_failure(format!("{word}: {problem}"));
	let value = TaskValue::of(value).map_err(|problem| failure(problem.to_string()))?;
	build_in_program(caller, &value, &failure)
}

/// `broadcast value on "chat"`: the value as warp text to every listener of the channel
fn channel_send(mut caller: Caller<'_, HostState>, channel: HostNode, message: HostNode) -> wasmtime::Result<()> {
	let result = channel_send_or_failure(&mut caller, channel, message);
	raised(&mut caller, result)
}

fn channel_send_or_failure(
	caller: &mut Caller<'_, HostState>,
	channel: HostNode,
	message: HostNode,
) -> wasmtime::Result<()> {
	let channel = given_node(caller, channel)?;
	let message = given_node(caller, message)?;
	if crate::web::web_sockets::is_socket_address(&channel.name()) {
		return crate::web::web_sockets::send(&channel.name(), &message).map_err(task_failure);
	}
	crate::host::channels::send(&channel.name(), message.serialize().trim()).map_err(task_failure)
}

/// An event for the page: kept for the headless page (headless.rs dispatched); a page without a browser shows nothing
fn dispatch_event(mut caller: Caller<'_, HostState>, name: HostNode, detail: HostNode) -> wasmtime::Result<()> {
	let name = given_node(&mut caller, name)?.name();
	let detail = given_node(&mut caller, detail)?.serialize();
	caller.data_mut().dispatched.push((name, detail));
	Ok(())
}

/// `notify "text"`: a desktop notification of the text (a value other than a text as warp writes it)
fn notify(mut caller: Caller<'_, HostState>, text: HostNode) -> wasmtime::Result<()> {
	let result = notify_or_failure(&mut caller, text);
	raised(&mut caller, result)
}

fn notify_or_failure(caller: &mut Caller<'_, HostState>, text: HostNode) -> wasmtime::Result<()> {
	let text = text_or_written(given_node(caller, text)?);
	warp_runtime::system_values::notify(&text).map_err(task_failure)
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
		assert_eq!(state.alloc(5), 65552); // 65536 + 10 = 65546, aligned to 8
	}
}
