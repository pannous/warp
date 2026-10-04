//! Host functions for WASM modules
//! Provides `fetch(url) -> text or error` and `run(wasm_bytes) -> Node`
//!
//! A remote call can fail, so `fetch` returns either the body or an Error value carrying the reason
//! (DNS, connection, timeout, HTTP status >= 400), never a silent empty string (DESIGN.md "Effects": Result<T, E>).

use std::time::Duration;

/// The module's bump pointer for runtime texts; texts the host returns are allocated from it too, so they never overlap
pub const TEXT_HEAP_EXPORT: &str = "text_heap";
const PAGE_BITS: u32 = 16;

/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>` is `fetch <url>`; `random(n)` is `random_below(n)`)
const HOST_ALIASES: [(&str, Option<usize>, &str); 2] = [("download", None, "fetch"), ("random", Some(1), RANDOM_BELOW)];

/// The words of the program's environment, imported from the "host" module and called like C functions (ffi.rs):
/// `sleep(ms)` pauses, `random()` is a float in [0, 1), `random_below(n)` an int in 0..n, `clock()` the milliseconds
/// since the Unix epoch. A program that defines a word of the same name keeps its own.
pub const HOST_LIBRARY: &str = "host";
const SLEEP: &str = "sleep";
const RANDOM: &str = "random";
const RANDOM_BELOW: &str = "random_below";
const CLOCK: &str = "clock";
/// `go f(x)` on a thread (tasks.rs): task_spawn(function name, up to four Int arguments) → task id, task_await(id) → result
pub const TASK_SPAWN: &str = "task_spawn";
pub const TASK_AWAIT: &str = "task_await";
pub const TASK_CONTROL: &str = "task_control";
/// The operations of `task_control(id, op)`: `stop job` / `cancel job`, `pause job`, `resume job`
pub const TASK_STOP: i64 = 1;
pub const TASK_PAUSE: i64 = 2;
pub const TASK_RESUME: i64 = 3;
/// The Int arguments task_spawn carries; a function of more runs where it is started
pub const MAX_TASK_ARGUMENTS: usize = 4;
pub const HOST_WORDS: [&str; 7] = [SLEEP, RANDOM, RANDOM_BELOW, CLOCK, TASK_SPAWN, TASK_AWAIT, TASK_CONTROL];

/// name, parameters, results of the host words
pub fn host_word_signatures() -> [(&'static str, Vec<wasm_encoder::ValType>, Vec<wasm_encoder::ValType>); 7] {
	use wasm_encoder::ValType::{F64, I32, I64};
	[(SLEEP, vec![I64], vec![]), (RANDOM, vec![], vec![F64]), (RANDOM_BELOW, vec![I64], vec![I64]), (CLOCK, vec![], vec![I64]),
		(TASK_SPAWN, vec![I32, I64, I64, I64, I64], vec![I64]), (TASK_AWAIT, vec![I64], vec![I64]), (TASK_CONTROL, vec![I64, I64], vec![I64])]
}

/// xorshift64*, seeded from the clock once per process: random enough for games and samples, not for secrets
#[cfg(feature = "native")]
fn next_random() -> u64 {
	use std::sync::atomic::{AtomicU64, Ordering};
	static STATE: AtomicU64 = AtomicU64::new(0);
	let mut x = STATE.load(Ordering::Relaxed);
	if x == 0 {
		x = (milliseconds_since_epoch() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
	}
	x ^= x >> 12;
	x ^= x << 25;
	x ^= x >> 27;
	STATE.store(x, Ordering::Relaxed);
	x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

fn milliseconds_since_epoch() -> i64 {
	std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// How long `fetch URL` waits for the whole response; `fetch URL timeout SECONDS` overrides it
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

use crate::extensions::utils::download_within;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
#[cfg(feature = "native")]
use crate::util::gc_engine;
use anyhow::{anyhow, Result};
use log::trace;
#[cfg(feature = "native")]
use wasmtime::{Caller, Engine, Extern, Linker, Memory, Module, Val};

#[cfg(feature = "native")]
/// The state of a running program, one for all its imports: host functions, WASI and FFI share one linker, so a program
/// can print and fetch, or print and call C
pub struct HostState {
	/// Next free offset in linear memory for string allocation
	next_alloc: u32,
	/// WASI preview 1 (fd_write …), writing to the process's stdout and stderr
	pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
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
	let len = bytes.len() as u32;
	let heap = match caller.get_export(TEXT_HEAP_EXPORT) {
		Some(Extern::Global(global)) => Some(global),
		_ => None,
	};
	let ptr = match &heap {
		Some(global) => {
			let top = global.get(&mut *caller).i32().unwrap_or(0) as u32;
			let memory_end = (memory.size(&*caller) as u32) << PAGE_BITS;
			if top == 0 || top + len > memory_end {
				memory.grow(&mut *caller, ((len >> PAGE_BITS) + 1) as u64)?;
				memory_end
			} else {
				top
			}
		}
		None => caller.data_mut().alloc(len),
	};
	let pages_needed = ((ptr + len) as usize).div_ceil(1 << PAGE_BITS);
	let current_pages = memory.size(&*caller) as usize;
	if pages_needed > current_pages {
		memory.grow(&mut *caller, (pages_needed - current_pages) as u64)?;
	}
	memory.write(&mut *caller, ptr as usize, bytes)?;
	if let Some(global) = heap {
		global.set(&mut *caller, Val::I32((ptr + len) as i32))?;
	}
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

	linker.func_wrap(HOST_LIBRARY, SLEEP, |milliseconds: i64| std::thread::sleep(Duration::from_millis(milliseconds.max(0) as u64)))?;
	// the top 53 bits make a uniform f64 in [0, 1)
	linker.func_wrap(HOST_LIBRARY, RANDOM, || (next_random() >> 11) as f64 / (1u64 << 53) as f64)?;
	linker.func_wrap(HOST_LIBRARY, RANDOM_BELOW, |bound: i64| if bound <= 0 { 0 } else { (next_random() % bound as u64) as i64 })?;
	linker.func_wrap(HOST_LIBRARY, CLOCK, milliseconds_since_epoch)?;

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
	let mut defined = std::collections::HashSet::new();
	crate::library_words::collect_assigned_names(&node, &mut defined);
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	defined.extend(context.user_functions.into_keys());
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
