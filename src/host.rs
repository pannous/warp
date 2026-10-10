//! Host functions for WASM modules
//! Provides `fetch(url) -> text or error` and `run(wasm_bytes) -> Node`
//!
//! A remote call can fail, so `fetch` returns either the body or an Error value carrying the reason
//! (DNS, connection, timeout, HTTP status >= 400), never a silent empty string (DESIGN.md "Effects": Result<T, E>).

use std::time::Duration;

/// The module's bump pointer for runtime texts; texts the host returns are allocated from it too, so they never overlap
pub const TEXT_HEAP_EXPORT: &str = "text_heap";
/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>` is `fetch <url>`; `random(n)` is `random_below(n)`)
const HOST_ALIASES: [(&str, Option<usize>, &str); 2] = [("download", None, "fetch"), ("random", Some(1), RANDOM_BELOW)];

/// The words of the program's environment, imported from the "host" module and called like C functions (ffi.rs);
/// sleep, random, random_below and clock need no compiler and live in warp-runtime (runtime/src/host_words.rs)
pub use warp_runtime::host_words::{CLOCK, LOCAL_OFFSET, EXIT, FILE_HANDLER_PREFIX, HOST_LIBRARY, SIGNAL_WATCH, INTERRUPT_HANDLER, RANDOM, RANDOM_BELOW, RANDOM_SEED, SHARED_HANDLER, SIGNAL_AT, SIGNAL_DAILY, SIGNAL_EVERY, SIGNAL_POLL, SLEEP, SYSTEM_VALUE, CLIPBOARD_TEXT, NOTIFY, GPU_COMPUTE, GPU_RENDER, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, PAGE_PATH, TEXT_COVERAGE, TIMER_HANDLER_PREFIX, FETCH_HANDLER_PREFIX};
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
/// sound_samples(samples, count, rate): 16-bit mono samples offset by 32768 (lib/sound.warp) played by the browser playground
/// (host.js, WebAudio); natively a WAV file the system's player plays (src/sound.rs)
pub const SOUND: &str = "sound_samples";
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
pub const VALUE_GIVING_WORDS: [&str; 14] = [GPU_COMPUTE, GPU_RENDER, TEXT_COVERAGE, FETCH_REPLY, RUN_BLOCK, FOREIGN_CALL, BLOCK_VALUE, CHANNEL_NEXT, CLIPBOARD_TEXT, PAGE_PATH, CHANNEL_WORDS[2], STD_PURE, STD_IO, SERVE_ROUTES];
pub const HOST_WORDS: [&str; 62] = [GPU_COMPUTE, GPU_RENDER, TEXT_COVERAGE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, FETCH_START, FETCH_REPLY, SERVE_ROUTES, STD_PURE, STD_IO, CHANNEL_LISTEN, CHANNEL_PENDING, CHANNEL_NEXT, CHANNEL_SEND, CLIPBOARD_TEXT, NOTIFY, PAGE_PATH, GUARDED_CALL, PAINT, SOUND, RUN_BLOCK, BLOCK_VALUE, FOREIGN_CALL, SLEEP, RANDOM, RANDOM_BELOW, RANDOM_SEED, CLOCK, LOCAL_OFFSET, SIGNAL_POLL, SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT, SIGNAL_WATCH, SYSTEM_VALUE, EXIT, TASK_SPAWN, TASK_AWAIT, TASK_CONTROL, TASK_SPAWN_VALUES, TASK_AWAIT_VALUE, TASK_JOIN, TASK_FAILURE, TASK_STATUS, TASK_POLL, TASK_INSIDE, SIGNAL_SEND,
	SHARED_WORDS[0], SHARED_WORDS[1], SHARED_WORDS[2], SHARED_WORDS[3], SHARED_WORDS[4], SHARED_WRITES, SHARED_FLOAT_WORDS[0], SHARED_FLOAT_WORDS[1], SHARED_FLOAT_WORDS[2],
	CHANNEL_WORDS[0], CHANNEL_WORDS[1], CHANNEL_WORDS[2], CHANNEL_WORDS[3], CHANNEL_WORDS[4]];

/// name, parameters, results of the host words
pub fn host_word_signatures() -> [(&'static str, Vec<wasm_encoder::ValType>, Vec<wasm_encoder::ValType>); 62] {
	use wasm_encoder::ValType::{F64, I32, I64};
	let node = wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF);
	[(GPU_COMPUTE, vec![node, node, I64], vec![node]), (GPU_RENDER, vec![node, I64, I64, node], vec![node]), (TEXT_COVERAGE, vec![node, I64], vec![node]), (GPU_COMPUTE_LINEAR, vec![node, I64, I64], vec![I64]), (GPU_MAP_LINEAR, vec![node, I64, node, I64, I64, I64], vec![I64]), (GPU_REDUCE_LINEAR, vec![node, I64, node, I64, I64, I64], vec![I64]), (FETCH_START, vec![I64, node], vec![]), (FETCH_REPLY, vec![I64], vec![node]), (SERVE_ROUTES, vec![I64, node], vec![node]), (STD_PURE, vec![node, node, node], vec![node]), (STD_IO, vec![node, node, node], vec![node]), (CHANNEL_LISTEN, vec![I64, node], vec![]), (CHANNEL_PENDING, vec![I64], vec![I64]), (CHANNEL_NEXT, vec![I64], vec![node]), (CLIPBOARD_TEXT, vec![], vec![node]), (PAGE_PATH, vec![], vec![node]), (NOTIFY, vec![node], vec![]), (CHANNEL_SEND, vec![node, node], vec![]),
		(GUARDED_CALL, vec![I32, node], vec![node]), (PAINT, vec![node, I64, I64, node], vec![]), (SOUND, vec![node, I64, I64], vec![]), (RUN_BLOCK, vec![node, node, node, node], vec![node]), (BLOCK_VALUE, vec![I64], vec![node]), (FOREIGN_CALL, vec![node, node, node, node, node], vec![node]), (SLEEP, vec![I64], vec![]), (RANDOM, vec![], vec![F64]), (RANDOM_BELOW, vec![I64], vec![I64]), (RANDOM_SEED, vec![I64], vec![]), (CLOCK, vec![], vec![I64]), (LOCAL_OFFSET, vec![I64], vec![I64]), (SIGNAL_POLL, vec![], vec![]), (SIGNAL_EVERY, vec![I64, I64], vec![]), (SIGNAL_DAILY, vec![I64, I64, I64], vec![]), (SIGNAL_AT, vec![I64, I64], vec![]), (SIGNAL_WATCH, vec![I64, I32], vec![]), (SYSTEM_VALUE, vec![I32], vec![I64]), (EXIT, vec![I64], vec![]),
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

/// How long `fetch URL` waits for the whole response; `fetch URL timeout SECONDS` overrides it
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

use crate::extensions::utils::download_within;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use anyhow::Result;
#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;
/// Body of `url`, or the reason it could not be fetched
pub fn fetch(url: &str, timeout: Duration) -> Result<String, String> {
	let mut content = download_within(url, timeout).map_err(|reason| format!("fetch {url} failed: {reason}"))?;
	if !content.ends_with('\n') {
		content.push('\n'); // warp convention
	}
	Ok(content)
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
