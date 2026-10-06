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

/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>` is `fetch <url>`; `random(n)` is `random_below(n)`)
const HOST_ALIASES: [(&str, Option<usize>, &str); 2] = [("download", None, "fetch"), ("random", Some(1), RANDOM_BELOW)];

/// The words of the program's environment, imported from the "host" module and called like C functions (ffi.rs);
/// sleep, random, random_below and clock need no compiler and live in warp-runtime (runtime/src/host_words.rs)
pub use warp_runtime::host_words::{CLOCK, EXIT, FILE_HANDLER_PREFIX, HOST_LIBRARY, SIGNAL_WATCH, INTERRUPT_HANDLER, RANDOM, RANDOM_BELOW, SHARED_HANDLER, SIGNAL_AT, SIGNAL_DAILY, SIGNAL_EVERY, SIGNAL_POLL, SLEEP, SYSTEM_VALUE, CLIPBOARD_TEXT, TIMER_HANDLER_PREFIX};
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
/// called by the words of std/<module>.wasp; std_pure's words have no effect (json), std_io's touch the outside
pub const STD_PURE: &str = "std_pure";
pub const STD_IO: &str = "std_io";
/// The host words whose result is any Node, its kind decided at run time (held like a map value)
pub const ANY_VALUE_WORDS: [&str; 3] = [FOREIGN_CALL, STD_PURE, STD_IO];
/// The host words that build a value in the program (tasks.rs Builders): it exports its constructors
pub const VALUE_GIVING_WORDS: [&str; 7] = [RUN_BLOCK, FOREIGN_CALL, BLOCK_VALUE, CHANNEL_NEXT, CLIPBOARD_TEXT, STD_PURE, STD_IO];
pub const HOST_WORDS: [&str; 42] = [STD_PURE, STD_IO, CHANNEL_LISTEN, CHANNEL_PENDING, CHANNEL_NEXT, CHANNEL_SEND, CLIPBOARD_TEXT, GUARDED_CALL, PAINT, RUN_BLOCK, BLOCK_VALUE, FOREIGN_CALL, SLEEP, RANDOM, RANDOM_BELOW, CLOCK, SIGNAL_POLL, SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT, SIGNAL_WATCH, SYSTEM_VALUE, EXIT, TASK_SPAWN, TASK_AWAIT, TASK_CONTROL, TASK_SPAWN_VALUES, TASK_AWAIT_VALUE, TASK_JOIN, TASK_FAILURE, TASK_STATUS, TASK_POLL, TASK_INSIDE, SIGNAL_SEND,
	SHARED_WORDS[0], SHARED_WORDS[1], SHARED_WORDS[2], SHARED_WORDS[3], SHARED_WORDS[4], SHARED_FLOAT_WORDS[0], SHARED_FLOAT_WORDS[1], SHARED_FLOAT_WORDS[2]];

/// name, parameters, results of the host words
pub fn host_word_signatures() -> [(&'static str, Vec<wasm_encoder::ValType>, Vec<wasm_encoder::ValType>); 42] {
	use wasm_encoder::ValType::{F64, I32, I64};
	let node = wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF);
	[(STD_PURE, vec![node, node, node], vec![node]), (STD_IO, vec![node, node, node], vec![node]), (CHANNEL_LISTEN, vec![I64, node], vec![]), (CHANNEL_PENDING, vec![I64], vec![I64]), (CHANNEL_NEXT, vec![I64], vec![node]), (CLIPBOARD_TEXT, vec![], vec![node]), (CHANNEL_SEND, vec![node, node], vec![]),
		(GUARDED_CALL, vec![I32, node], vec![node]), (PAINT, vec![node, I64, I64], vec![]), (RUN_BLOCK, vec![node, node, node, node], vec![node]), (BLOCK_VALUE, vec![I64], vec![node]), (FOREIGN_CALL, vec![node, node, node, node, node], vec![node]), (SLEEP, vec![I64], vec![]), (RANDOM, vec![], vec![F64]), (RANDOM_BELOW, vec![I64], vec![I64]), (CLOCK, vec![], vec![I64]), (SIGNAL_POLL, vec![], vec![]), (SIGNAL_EVERY, vec![I64, I64], vec![]), (SIGNAL_DAILY, vec![I64, I64, I64], vec![]), (SIGNAL_AT, vec![I64, I64], vec![]), (SIGNAL_WATCH, vec![I64, I32], vec![]), (SYSTEM_VALUE, vec![I32], vec![I64]), (EXIT, vec![I64], vec![]),
		(TASK_SPAWN, vec![I32, I64, I64, I64, I64], vec![I64]), (TASK_AWAIT, vec![I64], vec![I64]), (TASK_CONTROL, vec![I64, I64], vec![I64]),
		(TASK_SPAWN_VALUES, vec![I32, node], vec![I64]), (TASK_AWAIT_VALUE, vec![I64], vec![node]),
		(TASK_JOIN, vec![I64], vec![I64]), (TASK_FAILURE, vec![I64], vec![node]), (TASK_STATUS, vec![I64], vec![I64]), (TASK_POLL, vec![], vec![]),
		(TASK_INSIDE, vec![], vec![I64]), (SIGNAL_SEND, vec![I32, node], vec![I64]),
		(SHARED_WORDS[0], vec![I64], vec![I64]), (SHARED_WORDS[1], vec![I64, I64], vec![I64]), (SHARED_WORDS[2], vec![I64, I64, I64], vec![I64]),
		(SHARED_WORDS[3], vec![I64, I64, I64], vec![I64]), (SHARED_WORDS[4], vec![I64], vec![I64]),
		(SHARED_FLOAT_WORDS[0], vec![I64, I64], vec![F64]), (SHARED_FLOAT_WORDS[1], vec![I64, I64, F64], vec![F64]), (SHARED_FLOAT_WORDS[2], vec![I64, I64, F64], vec![F64])]
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
		Err(failure) if failure.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::StackOverflow) => {
			crate::tasks::TaskValue::Text(STACK_EXHAUSTED.to_string())
		}
		Err(failure) => return Err(failure),
	};
	let builders = crate::tasks::Builders::of(&mut |export| caller.get_export(export)).map_err(message)?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(message)?;
	if !matches!(value, crate::tasks::TaskValue::Text(_)) {
		return Ok(built.unwrap_anyref().copied());
	}
	let error_of = caller.get_export(crate::wasm_emitter::text_builtins::ERROR_OF).and_then(Extern::into_func).ok_or_else(|| wasmtime::Error::msg("no exported error_of"))?;
	let mut error = [Val::AnyRef(None)];
	error_of.call(&mut caller, &[built], &mut error)?;
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
	/// The C pointers this run got (sqlite3 *, FILE *), handed to wasp as ids (notes/ffi_handles.md)
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
		content.push('\n'); // wasp convention
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
	let is_fetch = |head: &Node| matches!(head, Node::Symbol(name) if name == "fetch");
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
	crate::channels::forget_listeners(); // each run links anew, on its own thread
	linker.func_wrap(HOST_LIBRARY, CHANNEL_LISTEN, channel_listen)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_PENDING, crate::channels::pending)?;
	linker.func_wrap(HOST_LIBRARY, CHANNEL_NEXT, channel_next)?;
	linker.func_wrap(HOST_LIBRARY, CLIPBOARD_TEXT, clipboard_text)?;
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
	let pixels = crate::wasm_reader::node_in(&Val::AnyRef(pixels), &mut caller.as_context_mut(), memory);
	let ink: Vec<bool> = match pixels.drop_meta() {
		crate::node::Node::List(items, _, _) => items.iter().map(|pixel| !matches!(pixel.drop_meta(), crate::node::Node::False | crate::node::Node::Empty) && pixel.drop_meta() != &crate::node::Node::int(0)).collect(),
		other => return Err(failure(format!("paint needs a list of pixels, got {}", other.serialize()))),
	};
	crate::paint::paint(&ink, width.max(0) as usize, height.max(0) as usize).map(|_| ()).map_err(failure)
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
	let [runtime, module, member, call, arguments] = [runtime, module, member, call, arguments].map(|value| crate::wasm_reader::node_in(&Val::AnyRef(value), &mut store, memory));
	let answer = crate::foreign::call(&runtime.name(), &module, &member.name(), call == 1, &arguments).map_err(failure)?;
	let value = TaskValue::of(&answer).map_err(|problem| failure(problem.to_string()))?;
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(|problem| failure(problem.to_string()))?;
	let built = builders.build(&value, &mut caller.as_context_mut()).map_err(|problem| failure(problem.to_string()))?;
	Ok(built.unwrap_anyref().copied())
}

#[cfg(feature = "native")]
type HostNode = Option<wasmtime::Rooted<wasmtime::AnyRef>>;

/// std_pure / std_io(module, member, arguments): a word of the standard library's adapters (src/std_adapters.rs)
#[cfg(feature = "native")]
fn std_call(mut caller: Caller<'_, HostState>, module: HostNode, member: HostNode, arguments: HostNode) -> wasmtime::Result<HostNode> {
	let [module, member, arguments] = [module, member, arguments].map(|value| given_node(&mut caller, value));
	let (module, member) = (module?.name(), member?.name());
	let answer = crate::std_adapters::call(&module, &member, &arguments?).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))?;
	built_in_program(&mut caller, &answer, &format!("{module}.{member}"))
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

/// The oldest message the listener got, as the value it was sent as (ø when none waits)
#[cfg(feature = "native")]
fn channel_next(mut caller: Caller<'_, HostState>, id: i64) -> wasmtime::Result<HostNode> {
	let message = crate::channels::next(id).map(|text| crate::wasp_parser::parse(&text).drop_meta().clone()).unwrap_or(Node::Empty);
	built_in_program(&mut caller, &message, "on message")
}

/// `clipboard`: the clipboard's text, read now (warp-runtime system_values.rs)
#[cfg(feature = "native")]
fn clipboard_text(mut caller: Caller<'_, HostState>) -> wasmtime::Result<HostNode> {
	let text = warp_runtime::system_values::clipboard_text().map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))?;
	built_in_program(&mut caller, &Node::Text(text), "clipboard")
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

/// `broadcast value on "chat"`: the value as wasp text to every listener of the channel
#[cfg(feature = "native")]
fn channel_send(mut caller: Caller<'_, HostState>, channel: HostNode, message: HostNode) -> wasmtime::Result<()> {
	let channel = given_node(&mut caller, channel)?;
	let message = given_node(&mut caller, message)?;
	crate::channels::send(&channel.name(), message.serialize().trim()).map_err(|problem| wasmtime::Error::new(crate::tasks::TaskFailure(problem)))
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
