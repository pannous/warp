//! Host functions for WASM modules
//! Provides `fetch(url) -> text or error` and `run(wasm_bytes) -> Node`
//!
//! A remote call can fail, so `fetch` returns either the body or an Error value carrying the reason
//! (DNS, connection, timeout, HTTP status >= 400), never a silent empty string (DESIGN.md "Effects": Result<T, E>).

use std::time::Duration;

/// The module's bump pointer for runtime texts; texts the host returns are allocated from it too, so they never overlap
pub const TEXT_HEAP_EXPORT: &str = "text_heap";
#[cfg(feature = "native")]
const PAGE_BITS: u32 = 16;
const I64_BYTES: usize = 8;

/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>` is `fetch <url>`; `random(n)` is `random_below(n)`)
const HOST_ALIASES: [(&str, Option<usize>, &str); 2] = [("download", None, "fetch"), ("random", Some(1), RANDOM_BELOW)];

/// The words of the program's environment, imported from the "host" module and called like C functions (ffi.rs);
/// sleep, random, random_below and clock need no compiler and live in warp-runtime (runtime/src/host_words.rs)
pub use warp_runtime::host_words::{CLOCK, EXIT, FILE_HANDLER_PREFIX, HOST_LIBRARY, SIGNAL_WATCH, INTERRUPT_HANDLER, RANDOM, RANDOM_BELOW, RANDOM_SEED, SHARED_HANDLER, SIGNAL_AT, SIGNAL_DAILY, SIGNAL_EVERY, SIGNAL_POLL, SLEEP, SYSTEM_VALUE, CLIPBOARD_TEXT, NOTIFY, GPU_COMPUTE, GPU_RENDER, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, PAGE_PATH, TIMER_HANDLER_PREFIX, FETCH_HANDLER_PREFIX};
/// `go f(x)` on a thread (tasks.rs): task_spawn(function name, up to four Int arguments) → task id, task_await(id) → result
pub const TASK_SPAWN: &str = "task_spawn";
pub const TASK_AWAIT: &str = "task_await";
pub const TASK_CONTROL: &str = "task_control";
/// `go f(x)` of a function of numbers, texts or characters: task_spawn_values(name, [arguments]) → task id,
/// task_await_value(id) → the result, both carried as Nodes
pub const TASK_SPAWN_VALUES: &str = "task_spawn_values";
pub const TASK_AWAIT_VALUE: &str = "task_await_value";
/// `await job` checks first (declarations::resolve_tasks): task_join(id) joins the task and is 1 when it failed,
/// task_failure(id) is the failure's text, raised from wasm so that `try` catches it
pub const TASK_JOIN: &str = "task_join";
pub const TASK_FAILURE: &str = "task_failure";
/// The marker `task·check(task_join(job), task_failure(job))` the emitter raises a failed task's error from
pub const TASK_CHECK: &str = "task·check";
/// task_status(id): 0 running, 1 finished, 2 failed, 3 stopped, 4 paused, without waiting (`once job finishes: …`)
pub const TASK_STATUS: &str = "task_status";
/// task_poll(): where a loop starts in a program that controls tasks, a task the browser paused waits there (natively
/// epoch interruption does it: a no-op)
pub const TASK_POLL: &str = "task_poll";
/// task_inside(): 1 in a task's instance, 0 in the program's; signal_send(handler, [event]): a raise inside a task
/// queued for the starting thread, which runs the handler's node wrapper at its next task word or at the end of the run
/// (P110, notes/signals.md phase 6)
pub const TASK_INSIDE: &str = "task_inside";
pub const SIGNAL_SEND: &str = "signal_send";
/// `try f(args) else Y` of a user function: guarded_call("f·node", [args]) calls f's node wrapper in the same instance
/// through the host, which turns the engine's stack overflow into the Error "call stack exhausted" that `try` catches
pub const GUARDED_CALL: &str = "guarded_call";
/// The Error a caught stack overflow is (the wasmtime trap's own words)
pub const STACK_EXHAUSTED: &str = "call stack exhausted";
pub const TASK_FINISHED: i64 = 1;
pub const TASK_FAILED: i64 = 2;
pub const TASK_STOPPED: i64 = 3;
pub const TASK_PAUSED: i64 = 4;
/// The operations of `task_control(id, op)`: `stop job` / `cancel job`, `pause job`, `resume job`
pub const TASK_STOP: i64 = 1;
pub const TASK_PAUSE: i64 = 2;
pub const TASK_RESUME: i64 = 3;
/// The Int arguments task_spawn carries; a function of more runs where it is started
pub const MAX_TASK_ARGUMENTS: usize = 4;
/// Shared arrays (shared_arrays.rs, src/shared.rs natively, host.js): shared_new(n) → array, shared_get(array, i),
/// shared_set(array, i, v), shared_add(array, i, v) (atomic, the new value), shared_count(array)
pub const SHARED_WORDS: [&str; 5] = ["shared_new", "shared_get", "shared_set", "shared_add", "shared_count"];
/// shared_writes(array): how many sets and adds the array had, so a listener polling it sees each write (P156 footgun
/// "coalesced signals")
pub const SHARED_WRITES: &str = "shared_writes";
/// The same for an array of floats (`shared xs = float[n]`), its cells holding the bits: get, set, add
pub const SHARED_FLOAT_WORDS: [&str; 3] = ["shared_getf", "shared_setf", "shared_addf"];
/// `interpret e` of a block known only at run time (wiki/charged.md §5, notes/runtime_eval.md): run_block(block, names,
/// values, definitions) compiles the block with the names bound to the values the program had where it ran it and the
/// program's function definitions, runs it, and gives its value
pub const RUN_BLOCK: &str = "run_block";
/// block·value(index): the number a block run at run time reads, of the values its run_block call passed
/// (pipeline::eval_block): the block is compiled as a function of them, the same module for other values
pub const BLOCK_VALUE: &str = "block·value";
/// foreign_call(runtime, module, member, call, arguments): a module of another runtime (src/foreign.rs), Nodes in and out
pub const FOREIGN_CALL: &str = "foreign_call";
/// paint(pixels, width, height): the pixels (a list, row after row, nonzero ink, 0 paper) drawn on the canvas of the
/// browser playground (host.js); natively a PNG file (src/paint.rs)
pub const PAINT: &str = "paint";
/// Channels between programs (src/channels.rs): channel_listen(id, channel), channel_pending(id) → count,
/// channel_next(id) → the oldest message, channel_send(channel, value)
pub const CHANNEL_LISTEN: &str = "channel_listen";
pub const CHANNEL_PENDING: &str = "channel_pending";
pub const CHANNEL_NEXT: &str = "channel_next";
pub const CHANNEL_SEND: &str = "channel_send";
/// std_pure(module, member, arguments) and std_io(…): the standard library's adapters (src/std_adapters.rs, host.js),
/// called by the words of lib/<module>.warp; std_pure's words have no effect (json), std_io's touch the outside
pub const STD_PURE: &str = "std_pure";
pub const STD_IO: &str = "std_io";
/// serve_routes(port, [[method, path, function] …]): `serve 8080 { get "/" { … } }` (src/web_server.rs), blocks while
/// it serves; ø once it stops
pub const SERVE_ROUTES: &str = "serve_routes";
/// `users := fetch url` (src/fetches.rs): fetch_start(id, url) fetches without waiting, the handler on·fetch·id runs once
/// the reply arrived, fetch_reply(id) → [value, error]: the parsed JSON (else the text) and ø, or ø and the failure
/// `users := fetch url` (lowering/fetch_signals.rs)
pub const FETCH_WORD: &str = "fetch";
pub const FETCH_START: &str = "fetch_start";
pub const FETCH_REPLY: &str = "fetch_reply";
/// The host words whose result is any Node, its kind decided at run time (held like a map value)
pub const ANY_VALUE_WORDS: [&str; 4] = [FETCH_REPLY, FOREIGN_CALL, STD_PURE, STD_IO];
/// Channels inside one run (P155, notes/channels.md, tasks.rs Channels), Go's unbuffered channel: channel_new() → id,
/// channel_put(id, value) waits until a receiver took it, channel_take(id) waits for a value (ø once closed and empty),
/// channel_more(id) waits until a value is offered (1) or the channel is closed (0), channel_close(id)
pub const CHANNEL_WORDS: [&str; 5] = ["channel_new", "channel_put", "channel_take", "channel_more", "channel_close"];
/// The host words of tasks, their signals and channels: a module calling one runs tasks (tasks.rs), a function that
/// calls one has the effect Async (effects.rs)
pub const TASK_WORDS: [&str; 16] = [TASK_SPAWN, TASK_AWAIT, TASK_CONTROL, TASK_SPAWN_VALUES, TASK_AWAIT_VALUE, TASK_JOIN, TASK_FAILURE, TASK_STATUS, TASK_POLL,
	TASK_INSIDE, SIGNAL_SEND, CHANNEL_WORDS[0], CHANNEL_WORDS[1], CHANNEL_WORDS[2], CHANNEL_WORDS[3], CHANNEL_WORDS[4]];
/// The host words that build a value in the program (tasks.rs Builders): it exports its constructors
pub const VALUE_GIVING_WORDS: [&str; 13] = [GPU_COMPUTE, GPU_RENDER, FETCH_REPLY, RUN_BLOCK, FOREIGN_CALL, BLOCK_VALUE, CHANNEL_NEXT, CLIPBOARD_TEXT, PAGE_PATH, CHANNEL_WORDS[2], STD_PURE, STD_IO, SERVE_ROUTES];
pub const HOST_WORDS: [&str; 59] = [GPU_COMPUTE, GPU_RENDER, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, FETCH_START, FETCH_REPLY, SERVE_ROUTES, STD_PURE, STD_IO, CHANNEL_LISTEN, CHANNEL_PENDING, CHANNEL_NEXT, CHANNEL_SEND, CLIPBOARD_TEXT, NOTIFY, PAGE_PATH, GUARDED_CALL, PAINT, RUN_BLOCK, BLOCK_VALUE, FOREIGN_CALL, SLEEP, RANDOM, RANDOM_BELOW, RANDOM_SEED, CLOCK, SIGNAL_POLL, SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT, SIGNAL_WATCH, SYSTEM_VALUE, EXIT, TASK_SPAWN, TASK_AWAIT, TASK_CONTROL, TASK_SPAWN_VALUES, TASK_AWAIT_VALUE, TASK_JOIN, TASK_FAILURE, TASK_STATUS, TASK_POLL, TASK_INSIDE, SIGNAL_SEND,
	SHARED_WORDS[0], SHARED_WORDS[1], SHARED_WORDS[2], SHARED_WORDS[3], SHARED_WORDS[4], SHARED_WRITES, SHARED_FLOAT_WORDS[0], SHARED_FLOAT_WORDS[1], SHARED_FLOAT_WORDS[2],
	CHANNEL_WORDS[0], CHANNEL_WORDS[1], CHANNEL_WORDS[2], CHANNEL_WORDS[3], CHANNEL_WORDS[4]];

/// name, parameters, results of the host words
pub fn host_word_signatures() -> [(&'static str, Vec<wasm_encoder::ValType>, Vec<wasm_encoder::ValType>); 59] {
	use wasm_encoder::ValType::{F64, I32, I64};
	let node = wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF);
	[(GPU_COMPUTE, vec![node, node, I64], vec![node]), (GPU_RENDER, vec![node, I64, I64, node], vec![node]), (GPU_COMPUTE_LINEAR, vec![node, I64, I64], vec![I64]), (GPU_MAP_LINEAR, vec![node, I64, node, I64, I64, I64], vec![I64]), (GPU_REDUCE_LINEAR, vec![node, I64, node, I64, I64, I64], vec![I64]), (FETCH_START, vec![I64, node], vec![]), (FETCH_REPLY, vec![I64], vec![node]), (SERVE_ROUTES, vec![I64, node], vec![node]), (STD_PURE, vec![node, node, node], vec![node]), (STD_IO, vec![node, node, node], vec![node]), (CHANNEL_LISTEN, vec![I64, node], vec![]), (CHANNEL_PENDING, vec![I64], vec![I64]), (CHANNEL_NEXT, vec![I64], vec![node]), (CLIPBOARD_TEXT, vec![], vec![node]), (PAGE_PATH, vec![], vec![node]), (NOTIFY, vec![node], vec![]), (CHANNEL_SEND, vec![node, node], vec![]),
		(GUARDED_CALL, vec![I32, node], vec![node]), (PAINT, vec![node, I64, I64], vec![]), (RUN_BLOCK, vec![node, node, node, node], vec![node]), (BLOCK_VALUE, vec![I64], vec![node]), (FOREIGN_CALL, vec![node, node, node, node, node], vec![node]), (SLEEP, vec![I64], vec![]), (RANDOM, vec![], vec![F64]), (RANDOM_BELOW, vec![I64], vec![I64]), (RANDOM_SEED, vec![I64], vec![]), (CLOCK, vec![], vec![I64]), (SIGNAL_POLL, vec![], vec![]), (SIGNAL_EVERY, vec![I64, I64], vec![]), (SIGNAL_DAILY, vec![I64, I64, I64], vec![]), (SIGNAL_AT, vec![I64, I64], vec![]), (SIGNAL_WATCH, vec![I64, I32], vec![]), (SYSTEM_VALUE, vec![I32], vec![I64]), (EXIT, vec![I64], vec![]),
		(TASK_SPAWN, vec![I32, I64, I64, I64, I64], vec![I64]), (TASK_AWAIT, vec![I64], vec![I64]), (TASK_CONTROL, vec![I64, I64], vec![I64]),
		(TASK_SPAWN_VALUES, vec![I32, node], vec![I64]), (TASK_AWAIT_VALUE, vec![I64], vec![node]),
		(TASK_JOIN, vec![I64], vec![I64]), (TASK_FAILURE, vec![I64], vec![node]), (TASK_STATUS, vec![I64], vec![I64]), (TASK_POLL, vec![], vec![]),
		(TASK_INSIDE, vec![], vec![I64]), (SIGNAL_SEND, vec![I32, node], vec![I64]),
		(SHARED_WORDS[0], vec![I64], vec![I64]), (SHARED_WORDS[1], vec![I64, I64], vec![I64]), (SHARED_WORDS[2], vec![I64, I64, I64], vec![I64]),
		(SHARED_WORDS[3], vec![I64, I64, I64], vec![I64]), (SHARED_WORDS[4], vec![I64], vec![I64]), (SHARED_WRITES, vec![I64], vec![I64]),
		(SHARED_FLOAT_WORDS[0], vec![I64, I64], vec![F64]), (SHARED_FLOAT_WORDS[1], vec![I64, I64, F64], vec![F64]), (SHARED_FLOAT_WORDS[2], vec![I64, I64, F64], vec![F64]),
		(CHANNEL_WORDS[0], vec![], vec![I64]), (CHANNEL_WORDS[1], vec![I64, node], vec![]), (CHANNEL_WORDS[2], vec![I64], vec![node]),
		(CHANNEL_WORDS[3], vec![I64], vec![I64]), (CHANNEL_WORDS[4], vec![I64], vec![])]
}

/// guarded_call(name, arguments): the node wrapper `name` called with the argument list in the caller's own instance;
/// a stack overflow inside is the Error "call stack exhausted" (what the aborted call wrote to globals and memory stays),
/// any other failure, running out of fuel included, ends the run as before
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn error_in_program(caller: &mut Caller<'_, HostState>, reason: &str) -> wasmtime::Result<HostNode> {
	let text = built_in_program(caller, &Node::Text(reason.to_string()), crate::wasm_emitter::text_builtins::ERROR_OF)?;
	let error_of = caller.get_export(crate::wasm_emitter::text_builtins::ERROR_OF).and_then(Extern::into_func).ok_or_else(|| wasmtime::Error::msg("no exported error_of"))?;
	let mut error = [Val::AnyRef(None)];
	error_of.call(&mut *caller, &[Val::AnyRef(text)], &mut error)?;
	Ok(error[0].unwrap_anyref().copied())
}

/// How long `fetch URL` waits for the whole response; `fetch URL timeout SECONDS` overrides it
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

use crate::extensions::utils::download_within;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
#[cfg(feature = "native")]
use crate::util::gc_engine;
#[cfg(feature = "native")]
use anyhow::anyhow;
use anyhow::Result;
#[cfg(feature = "native")]
use log::trace;
#[cfg(feature = "native")]
use wasmtime::{AsContextMut, Caller, Engine, Extern, Linker, Memory, Val};

#[cfg(feature = "native")]
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

#[cfg(feature = "native")]
impl Default for HostState {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(feature = "native")]
impl HostState {
	pub fn new() -> Self {
		HostState {
			next_alloc: 65536, // Start allocation after initial memory region
			wasi: wasmtime_wasi::WasiCtxBuilder::new().inherit_stdout().inherit_stderr().build_p1(),
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

#[cfg(feature = "native")]
/// Read a string from WASM linear memory
pub fn read_string_from_memory(memory: &Memory, store: &impl wasmtime::AsContext, ptr: u32, len: u32) -> Result<String> {
	let mut buf = vec![0u8; len as usize];
	memory.read(store, ptr as usize, &mut buf)?;
	String::from_utf8(buf).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

/// Write a string to WASM linear memory using Caller, returns (ptr, len)
#[cfg(feature = "native")]
fn write_string_to_caller(memory: &Memory, caller: &mut Caller<'_, HostState>, s: &str) -> Result<(u32, u32)> {
	write_bytes_to_caller(memory, caller, s.as_bytes())
}

/// Copy bytes into the module's memory: from its text heap when it exports one (the same rule as its own
/// emit_text_allocation: fresh pages past the current memory when the heap is unset or full), else from HostState
#[cfg(feature = "native")]
fn write_bytes_to_caller(memory: &Memory, caller: &mut Caller<'_, HostState>, bytes: &[u8]) -> Result<(u32, u32)> {
	let heap = match caller.get_export(TEXT_HEAP_EXPORT) {
		Some(Extern::Global(global)) => Some(global),
		_ => None,
	};
	write_bytes(memory, heap, &mut caller.as_context_mut(), bytes)
}

/// Copy bytes into a module's memory at its text heap `heap` (or from HostState without one): from a host function
/// (its caller) or into an instance the host made (a task, tasks.rs)
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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

#[cfg(feature = "native")]
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
	match write_bytes_to_caller(&memory, caller, &bytes) {
		Ok((ptr, len)) if failed => (ptr as i32, -(len as i32)),
		Ok((ptr, len)) => (ptr as i32, len as i32),
		Err(e) => {
			trace!("host.read: failed to write result: {}", e);
			(0, 0)
		}
	}
}

#[cfg(feature = "native")]
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

/// Body of `url`, or the reason it could not be fetched
pub fn fetch(url: &str, timeout: Duration) -> Result<String, String> {
	let mut content = download_within(url, timeout).map_err(|reason| format!("fetch {url} failed: {reason}"))?;
	if !content.ends_with('\n') {
		content.push('\n'); // warp convention
	}
	Ok(content)
}

#[cfg(feature = "native")]
/// Fetch the URL in WASM memory and write the result back: (ptr, len) of the body, (ptr, -len) of the failure reason
fn fetch_into_memory(caller: &mut Caller<'_, HostState>, url_ptr: i32, url_len: i32, timeout: Duration) -> (i32, i32) {
	let memory = match caller.get_export("memory") {
		Some(Extern::Memory(m)) => m,
		_ => {
			trace!("host.fetch: no memory export");
			return (0, 0);
		}
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
	match write_string_to_caller(&memory, caller, &text) {
		Ok((ptr, len)) if failed => (ptr as i32, -(len as i32)),
		Ok((ptr, len)) => (ptr as i32, len as i32),
		Err(e) => {
			trace!("host.fetch: failed to write result: {}", e);
			(0, 0)
		}
	}
}

/// `fetch URL` or `fetch URL timeout SECONDS`, whether parsed as a flat list or as nested implicit applications:
/// the URL node and the explicit timeout, if any
pub fn fetch_call(node: &Node) -> Option<(Node, Option<Duration>)> {
	// `x = fetch u timeout 2` applies implicitly from the left: ((fetch u) timeout) 2, or (fetch u) (timeout 2)
	fn parts(node: &Node) -> Vec<Node> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space | Separator::None) if !items.is_empty() => {
				let mut parts = parts(&items[0]);
				for item in &items[1..] {
					match item.drop_meta() {
						Node::List(inner, Bracket::None, Separator::Space | Separator::None) if matches!(inner.first().map(Node::drop_meta), Some(Node::Symbol(k)) if k == "timeout") => {
							parts.extend(inner.iter().map(|part| part.drop_meta().clone()))
						}
						other => parts.push(other.clone()),
					}
				}
				parts
			}
			other => vec![other.clone()],
		}
	}
	if !matches!(node.drop_meta(), Node::List(_, Bracket::None, _)) {
		return None;
	}
	let is_fetch = |head: &Node| matches!(head, Node::Symbol(name) if name == FETCH_WORD);
	match parts(node).as_slice() {
		[head, url] if is_fetch(head) => Some((url.clone(), None)),
		[head, url, keyword, Node::Number(seconds)] if is_fetch(head) && matches!(keyword, Node::Symbol(k) if k == "timeout") => {
			let seconds = match seconds {
				Number::Complex(..) => return None,
				real => f64::from(*real),
			};
			(seconds > 0.0 && seconds.is_finite()).then(|| (url.clone(), Some(Duration::from_secs_f64(seconds))))
		}
		_ => None,
	}
}

#[cfg(feature = "native")]
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
	linker.func_wrap(HOST_LIBRARY, GPU_COMPUTE_LINEAR, gpu_compute_linear)?;
	linker.func_wrap(HOST_LIBRARY, GPU_MAP_LINEAR, gpu_map_linear)?;
	linker.func_wrap(HOST_LIBRARY, GPU_REDUCE_LINEAR, gpu_reduce_linear)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_SEND, channel_send)?;
	linker.func_wrap(HOST_LIBRARY, PAINT, paint)?;

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
				Ok(result) => result,
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

/// paint(pixels, width, height) natively: the pixels as a PNG (src/paint.rs)
#[cfg(feature = "native")]
fn paint(mut caller: Caller<'_, HostState>, pixels: Option<wasmtime::Rooted<wasmtime::AnyRef>>, width: i64, height: i64) -> wasmtime::Result<()> {
	let failure = |message: String| wasmtime::Error::new(crate::tasks::TaskFailure(message));
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return Err(failure("paint: the module exports no memory".into())) };
	let (width, height) = (width.max(0) as usize, height.max(0) as usize);
	let values = match ints_of_list(&mut caller, memory, pixels, width * height)? {
		Some(values) => values,
		None => match crate::wasm_reader::node_in(&Val::AnyRef(pixels), &mut caller.as_context_mut(), memory).drop_meta() {
			crate::node::Node::List(items, _, _) => items.iter().map(pixel_value).collect(),
			other => return Err(failure(format!("paint needs a list of pixels, got {}", other.serialize()))),
		},
	};
	crate::paint::paint(&values, width, height).map(|_| ()).map_err(failure)
}

/// The first `room` items of a list of fixnum Ints read in one call of the module's list_to_ints (wasm_emitter/int_lists.rs), their magnitudes;
/// None for any other value, which the caller reads node by node
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn pixel_value(pixel: &crate::node::Node) -> u64 {
	use crate::node::Node;
	match pixel.drop_meta() {
		Node::False | Node::Empty => 0,
		Node::Number(number) => f64::from(*number).abs() as u64,
		_ => 1,
	}
}

/// The host side of run_block: the block and the values it sees come in as Nodes, the block's value goes back as one
#[cfg(feature = "native")]
fn run_block(mut caller: Caller<'_, HostState>, block: Option<wasmtime::Rooted<wasmtime::AnyRef>>, names: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	values: Option<wasmtime::Rooted<wasmtime::AnyRef>>, definitions: Option<wasmtime::Rooted<wasmtime::AnyRef>>) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::tasks::{Builders, TaskFailure, TaskValue};
	let failure = |message: String| wasmtime::Error::new(TaskFailure(message));
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return Err(failure("run_block: the module exports no memory".into())) };
	let mut store = caller.as_context_mut();
	let [block, names, values, definitions] = [block, names, values, definitions].map(|value| crate::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let result = crate::pipeline::eval_block(block, &names, &values, &definitions).map_err(failure)?;
	let value = TaskValue::of(&result).map_err(|_| failure(crate::pipeline::cannot_hand_back(&result)))?;
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

#[cfg(feature = "native")]
thread_local! {
	/// The values of the blocks running on this thread, the innermost last (a block may run a block)
	static BLOCK_VALUES: std::cell::RefCell<Vec<Vec<crate::tasks::TaskValue>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `body` while block·value reads `values`
#[cfg(feature = "native")]
pub fn with_block_values<R>(values: Vec<crate::tasks::TaskValue>, body: impl FnOnce() -> R) -> R {
	BLOCK_VALUES.with(|stack| stack.borrow_mut().push(values));
	let result = body();
	BLOCK_VALUES.with(|stack| stack.borrow_mut().pop());
	result
}

/// The host side of block·value: the value at `index`, built in the block's module
#[cfg(feature = "native")]
fn block_value(mut caller: Caller<'_, HostState>, index: i64) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	let failure = |message: String| wasmtime::Error::msg(message);
	let value = BLOCK_VALUES.with(|stack| stack.borrow().last().and_then(|values| values.get(index as usize).cloned()))
		.ok_or_else(|| failure(format!("{BLOCK_VALUE}({index}): no such value of the running block")))?;
	let builders = crate::tasks::Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// The host side of foreign_call: the runtime, module and member names and the arguments come in as Nodes, the
/// answer goes back as one (src/foreign.rs)
#[cfg(feature = "native")]
fn foreign_call(mut caller: Caller<'_, HostState>, runtime: Option<wasmtime::Rooted<wasmtime::AnyRef>>, module: Option<wasmtime::Rooted<wasmtime::AnyRef>>,
	member: Option<wasmtime::Rooted<wasmtime::AnyRef>>, call: Option<wasmtime::Rooted<wasmtime::AnyRef>>, arguments: Option<wasmtime::Rooted<wasmtime::AnyRef>>) -> wasmtime::Result<Option<wasmtime::Rooted<wasmtime::AnyRef>>> {
	use crate::tasks::{Builders, TaskFailure, TaskValue};
	let failure = |message: String| wasmtime::Error::new(TaskFailure(message));
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return Err(failure("foreign_call: the module exports no memory".into())) };
	let mut store = caller.as_context_mut();
	let functions = function_arguments(&mut store, arguments)?;
	let [runtime, module, member, call, arguments] = [runtime, module, member, call, arguments].map(|value| crate::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let arguments = with_function_markers(arguments, &functions);
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let mut callback = |index: usize, values: Node| -> Result<Node, String> {
		let Some(Extern::Func(apply)) = caller.get_export(crate::wasm_emitter::CLOSURE_APPLY) else { return Err("the module exports no closure_apply".into()) };
		let function = functions.iter().flatten().nth(index).copied().ok_or(format!("no warp function {index}"))?;
		let values = TaskValue::of(&values).and_then(|values| builders.build(&values, &mut caller.as_context_mut())).map_err(|problem| problem.to_string())?;
		let mut result = [Val::AnyRef(None)];
		apply.call(&mut caller, &[Val::AnyRef(Some(function)), values], &mut result).map_err(|trap| trap.root_cause().to_string())?;
		given_node(&mut caller, result[0].unwrap_anyref().copied()).map_err(|problem| problem.to_string())
	};
	let answer = crate::foreign::call(&runtime.name(), &module, &member.name(), call == 1, &arguments, &mut callback).map_err(failure)?;
	let value = TaskValue::of(&answer).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// A foreign call's arguments, the list's items or the one argument, each a closure or None
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn with_function_markers(arguments: Node, functions: &[Option<wasmtime::Rooted<wasmtime::AnyRef>>]) -> Node {
	if functions.iter().all(Option::is_none) {
		return arguments;
	}
	let marker = |index: usize| Node::List(vec![Node::Key(Box::new(Node::Symbol(crate::foreign::WARP_FUNCTION_KEY.into())), crate::operators::Op::Colon, Box::new(Node::int(index as i64)))], crate::node::Bracket::Curly, crate::node::Separator::Space);
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

#[cfg(feature = "native")]
type HostNode = Option<wasmtime::Rooted<wasmtime::AnyRef>>;

/// serve_routes(port, routes): HTTP on the port, each request answered by its route's function (src/web_server.rs)
#[cfg(feature = "native")]
fn serve_routes(mut caller: Caller<'_, HostState>, port: i64, routes: HostNode) -> wasmtime::Result<HostNode> {
	use crate::web_server::{routes_of, serve};
	let routes = routes_of(&given_node(&mut caller, routes)?);
	let port = u16::try_from(port).map_err(|_| wasmtime::Error::new(crate::tasks::TaskFailure(format!("serve {port}: no port number"))))?;
	let site = served_site(port);
	serve(port, &routes, &site, |route, request| match route_value(&mut caller, &route.function, &request) {
		Ok(value) => Ok(route.answer_of(&value)),
		Err(failure) => Err(route_failure(&mut caller, &route.path, failure)),
	}).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))?;
	built_in_program(&mut caller, &Node::Empty, SERVE_ROUTES)
}

/// What a failed route says to the client: the program's error message, read with the value the program left in
/// trap_detail (as wasm_reader::with_trap_detail does for main); the whole trace goes to the server's log
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn served_site(port: u16) -> crate::site::ServedSite {
	let Some(file) = crate::modules::program_file() else { return Default::default() };
	let title = file.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().to_string());
	let rendered = std::fs::read_to_string(&file).map_err(|failure| failure.to_string()).and_then(|code| crate::site::served_files(&code, &title));
	rendered.unwrap_or_else(|failure| {
		eprintln!("warning: serve {port} serves no page: {failure}");
		None
	}).unwrap_or_default()
}

/// The value of a route's function called with the request (if it takes one), as a Node
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
		(Err(problem), None) => Err(wasmtime::Error::new(crate::tasks::TaskFailure(format!("table.select: {problem}")))),
	}
}

/// The node a host word was given, read out of the caller's instance
#[cfg(feature = "native")]
fn given_node(caller: &mut Caller<'_, HostState>, value: HostNode) -> wasmtime::Result<Node> {
	let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return Err(wasmtime::Error::msg("the module exports no memory")) };
	Ok(crate::wasm_reader::node_in(&Val::AnyRef(value), &mut caller.as_context_mut(), memory))
}

/// `on message from "chat" {…}` starts listening (src/channels.rs)
#[cfg(feature = "native")]
fn channel_listen(mut caller: Caller<'_, HostState>, id: i64, channel: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	crate::channels::listen(id, &channel.name()).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))
}

/// A fetch's URL, or `[url, body]`: a POST of the body as JSON (a server function the page calls, lowering/serve.rs)
#[cfg(feature = "native")]
fn request_of(request: &Node) -> crate::fetches::Request {
	match request.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => (items[0].name(), Some(crate::foreign::json_of(&items[1]).to_string())),
		url => (url.name(), None),
	}
}

/// `users := fetch url` starts the fetch (src/fetches.rs)
#[cfg(feature = "native")]
fn fetch_start(mut caller: Caller<'_, HostState>, id: i64, url: HostNode) -> wasmtime::Result<()> {
	let request = given_node(&mut caller, url)?;
	crate::fetches::start(id, request_of(&request), FETCH_TIMEOUT);
	Ok(())
}

/// [value, error] of the fetch's reply (src/fetches.rs)
#[cfg(feature = "native")]
fn fetch_reply(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	built_in_program(&mut caller, &crate::fetches::reply(id), "fetch")
}

/// The oldest message the listener got, as the value it was sent as (ø when none waits)
#[cfg(feature = "native")]
fn channel_next(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	let message = crate::web_sockets::next(id).unwrap_or_else(|| crate::channels::next(id).map(|text| crate::warp_parser::parse(&text).drop_meta().clone()).unwrap_or(Node::Empty));
	built_in_program(&mut caller, &message, "on message")
}

/// `clipboard`: the clipboard's text, read now (warp-runtime system_values.rs)
#[cfg(feature = "native")]
fn clipboard_text(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let text = warp_runtime::system_values::clipboard_text().map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))?;
	built_in_program(&mut caller, &Node::Text(text), "clipboard")
}

/// The path of the page natively: "/", or the one a render for a request is made for (with_page_path)
#[cfg(feature = "native")]
fn page_path(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let path = NATIVE_PAGE_PATH.with(|path| path.borrow().clone());
	built_in_program(&mut caller, &Node::Text(path), PAGE_PATH)
}

#[cfg(feature = "native")]
thread_local! {
	static NATIVE_PAGE_PATH: std::cell::RefCell<String> = std::cell::RefCell::new(ROOT_PATH.to_string());
}
#[cfg(feature = "native")]
const ROOT_PATH: &str = "/";

/// Run `body` with the page at `path` (a page rendered for a request, src/site.rs)
#[cfg(feature = "native")]
pub fn with_page_path<R>(path: &str, body: impl FnOnce() -> R) -> R {
	let previous = NATIVE_PAGE_PATH.with(|current| current.replace(path.to_string()));
	let result = body();
	NATIVE_PAGE_PATH.with(|current| current.replace(previous));
	result
}

/// A value of the host as a value of the program, built by its exported constructors
#[cfg(feature = "native")]
fn built_in_program(caller: &mut Caller<'_, HostState>, value: &Node, word: &str) -> wasmtime::Result<HostNode> {
	use crate::tasks::{Builders, TaskFailure, TaskValue};
	let failure = |problem: String| wasmtime::Error::new(TaskFailure(format!("{word}: {problem}")));
	let value = TaskValue::of(value).map_err(|problem| failure(problem.to_string()))?;
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

/// `broadcast value on "chat"`: the value as warp text to every listener of the channel
#[cfg(feature = "native")]
fn channel_send(mut caller: Caller<'_, HostState>, channel: HostNode, message: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	let message = given_node(&mut caller, message)?;
	if crate::web_sockets::is_socket_address(&channel.name()) {
		return crate::web_sockets::send(&channel.name(), &message).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)));
	}
	crate::channels::send(&channel.name(), message.serialize().trim()).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))
}

/// `notify "text"`: a desktop notification of the text (a value other than a text as warp writes it)
#[cfg(feature = "native")]
fn notify(mut caller: Caller<'_, HostState>, text: HostNode) -> wasmtime::Result<()> {
	let text = match given_node(&mut caller, text)? {
		Node::Text(text) => text,
		other => other.serialize(),
	};
	warp_runtime::system_values::notify(&text).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))
}

/// `gpu_compute(shader, numbers, workgroups)` through wgpu (src/gpu.rs): the floats the shader left
#[cfg(feature = "native")]
fn gpu_compute(mut caller: Caller<'_, HostState>, shader: HostNode, numbers: HostNode, workgroups: i64) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_COMPUTE);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let numbers = given_node(&mut caller, numbers)?;
	let numbers = numbers.iter().map(|number| match number.drop_meta() {
		Node::Number(number) => Ok(*number),
		other => Err(failure(format!("it computes over numbers, got {}", other.serialize()))),
	}).collect::<wasmtime::Result<Vec<Number>>>()?;
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
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

/// `gpu_compute(shader, xs, workgroups)` of a linear float array: its block `[count: i64][count f64 cells]`
/// (wasm_emitter/linear_arrays.rs) read and written in place, as f32 on the GPU
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn gpu_map_linear(caller: Caller<'_, HostState>, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64) -> wasmtime::Result<i64> {
	gpu_kernel_linear(caller, GPU_MAP_LINEAR, shader, source, values, target, workgroups, keeping, false)
}

/// `s = sum(xs.map(x => …) @gpu)`, min, max: as gpu_map_linear, but each workgroup leaves its partial result in a cell
/// after the values, and those, the only ones read back, go into the block `target` of `workgroups` cells
#[cfg(feature = "native")]
fn gpu_reduce_linear(caller: Caller<'_, HostState>, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64) -> wasmtime::Result<i64> {
	gpu_kernel_linear(caller, GPU_REDUCE_LINEAR, shader, source, values, target, workgroups, keeping, true)
}

#[cfg(feature = "native")]
#[allow(clippy::too_many_arguments)]
fn gpu_kernel_linear(mut caller: Caller<'_, HostState>, word: &'static str, shader: HostNode, source: i64, values: HostNode, target: i64, workgroups: i64, keeping: i64, reduces: bool) -> wasmtime::Result<i64> {
	static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
	let failure = gpu_failure(word);
	if let Err(problem) = crate::gpu::available() {
		if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
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
	Ok(1)
}

#[cfg(feature = "native")]
const LINEAR_CELL_BYTES: usize = 8;

/// The byte range of the cells of a linear array's block `[count: i64][count f64 cells]` (wasm_emitter/linear_arrays.rs)
#[cfg(feature = "native")]
fn linear_cells(memory: &[u8], block: i64) -> Option<std::ops::Range<usize>> {
	let start = usize::try_from(block).ok()?;
	let count = i64::from_le_bytes(memory.get(start..start + LINEAR_CELL_BYTES)?.try_into().ok()?);
	let end = start + LINEAR_CELL_BYTES + usize::try_from(count).ok()? * LINEAR_CELL_BYTES;
	(end <= memory.len()).then_some(start + LINEAR_CELL_BYTES..end)
}

#[cfg(feature = "native")]
fn linear_memory(caller: &mut Caller<'_, HostState>, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<wasmtime::Memory> {
	match caller.get_export("memory") {
		Some(Extern::Memory(memory)) => Ok(memory),
		_ => Err(failure("the module exports no memory".into())),
	}
}

/// A linear float array's cells as f32, as the GPU takes them
#[cfg(feature = "native")]
fn linear_floats(caller: &mut Caller<'_, HostState>, block: i64, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<Vec<f32>> {
	let memory = linear_memory(caller, failure)?;
	let bytes = memory.data(&*caller);
	let cells = linear_cells(bytes, block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	Ok(bytes[cells].chunks_exact(LINEAR_CELL_BYTES).map(|cell| f64::from_le_bytes(cell.try_into().expect("8 bytes")) as f32).collect())
}

/// The f32 the GPU left, written into a linear float array's cells
#[cfg(feature = "native")]
fn write_linear_floats(caller: &mut Caller<'_, HostState>, block: i64, floats: &[f32], failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<()> {
	let memory = linear_memory(caller, failure)?;
	let cells = linear_cells(memory.data(&*caller), block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	let written = &mut memory.data_mut(caller)[cells];
	written.chunks_exact_mut(LINEAR_CELL_BYTES).zip(floats).for_each(|(cell, float)| cell.copy_from_slice(&f64::from(*float).to_le_bytes()));
	Ok(())
}

/// `gpu_render(shader, width, height)` through wgpu (src/gpu.rs): the pixels the fragment shader colored
#[cfg(feature = "native")]
fn gpu_render(mut caller: Caller<'_, HostState>, shader: HostNode, width: i64, height: i64, values: HostNode) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_RENDER);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let values = shader_values(&given_node(&mut caller, values)?).map_err(&failure)?;
	let side = |pixels: i64| u32::try_from(pixels).ok().filter(|&pixels| pixels > 0).ok_or_else(|| failure(format!("an image {width}×{height} pixels")));
	let pixels = crate::gpu::render(&shader, side(width)?, side(height)?, &values).map_err(&failure)?;
	if let Some(list) = list_of_ints(&mut caller, &pixels)? {
		return Ok(list);
	}
	let pixels = pixels.into_iter().map(|pixel| Node::Number(Number::Int(i64::from(pixel)))).collect();
	built_in_program(&mut caller, &Node::List(pixels, crate::node::Bracket::Square, crate::node::Separator::Space), GPU_RENDER)
}

/// The ints as a list built in one call of the module's ints_to_list (wasm_emitter/int_lists.rs); None when it has none
#[cfg(feature = "native")]
fn list_of_ints(caller: &mut Caller<'_, HostState>, ints: &[u32]) -> wasmtime::Result<Option<HostNode>> {
	use crate::wasm_emitter::int_lists::INTS_TO_LIST;
	let (Some(Extern::Func(ints_to_list)), Some(Extern::Memory(memory))) = (caller.get_export(INTS_TO_LIST), caller.get_export("memory")) else { return Ok(None) };
	let ints_to_list = ints_to_list.typed::<(i32, i32), HostNode>(&*caller)?;
	let Some(address) = scratch(caller, memory, std::mem::size_of_val(ints))? else { return Ok(None) };
	let bytes: Vec<u8> = ints.iter().flat_map(|int| int.to_le_bytes()).collect();
	memory.write(&mut *caller, address as usize, &bytes)?;
	Ok(Some(ints_to_list.call(&mut *caller, (address as i32, ints.len() as i32))?))
}

#[cfg(feature = "native")]
fn gpu_failure(word: &'static str) -> impl Fn(String) -> wasmtime::Error {
	move |problem| wasmtime::Error::new(crate::tasks::TaskFailure(format!("{word}: {problem}")))
}

/// gpu_render's values (none when left out): each entry a number or a list of numbers
#[cfg(feature = "native")]
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
		let floats = match value.drop_meta() {
			Node::List(items, _, _) => items.iter().map(|item| float(&name, item)).collect::<Result<Vec<f32>, String>>()?,
			single => vec![float(&name, single)?],
		};
		Ok((name, floats))
	}).collect()
}

/// The WGSL text of a gpu word's shader
#[cfg(feature = "native")]
fn given_shader(caller: &mut Caller<'_, HostState>, shader: HostNode, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<String> {
	match given_node(caller, shader)? {
		Node::Text(shader) => Ok(shader),
		Node::Char(letter) => Ok(letter.to_string()), // a one-letter text reads as a code point
		other => Err(failure(format!("the shader is a text of WGSL, got {}", other.serialize()))),
	}
}

#[cfg(feature = "native")]
/// Create a linker with host functions pre-linked
pub fn create_host_linker(engine: &Engine) -> Result<Linker<HostState>> {
	let mut linker = Linker::new(engine);
	link_host_functions(&mut linker, engine)?;
	Ok(linker)
}

#[cfg(all(test, feature = "native"))]
mod tests {
	use super::*;

	#[test]
	fn test_host_state() {
		let mut state = HostState::new();
		assert_eq!(state.alloc(10), 65536);
		assert_eq!(state.alloc(5), 65552); // 65536 + 10, aligned to 8 = 65544? No, 65536+10=65546, aligned = 65552
	}
}

/// `download url` → `fetch url`: an alias at the head of an application or call, unless the program defines the alias itself
pub fn lower_aliases(node: Node) -> Node {
	if !node.mentions_any(&HOST_ALIASES.map(|(alias, _, _)| alias)) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let aliases: Vec<_> = HOST_ALIASES.into_iter().filter(|(alias, _, _)| !defined.contains(*alias)).collect();
	if aliases.is_empty() {
		return node;
	}
	renamed_heads(node, &aliases)
}

fn renamed_heads(node: Node, aliases: &[(&str, Option<usize>, &str)]) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| renamed_heads(item, aliases)).collect();
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				let arguments = items.len() - 1;
				if let Some((_, _, word)) = aliases.iter().find(|(alias, arity, _)| alias == name && arity.is_none_or(|arity| arity == arguments)) {
					items[0] = Node::Symbol(word.to_string());
				}
			}
			Node::List(items, bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(renamed_heads(*left, aliases)), op, Box::new(renamed_heads(*right, aliases))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(renamed_heads(*node, aliases)), data },
		other => other,
	}
}
