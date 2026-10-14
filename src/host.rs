//! Host functions for WASM modules
//! Provides `fetch(url) -> text or error` and `run(wasm_bytes) -> Node`
//!
//! A remote call can fail, so `fetch` returns either the body or an Error value carrying the reason
//! (DNS, connection, timeout, HTTP status >= 400), never a silent empty string (DESIGN.md "Effects": Result<T, E>).

use std::time::Duration;

/// The module's bump pointer for runtime texts; texts the host returns are allocated from it too, so they never overlap
pub const TEXT_HEAP_EXPORT: &str = "text_heap";

/// The words of the program's environment, imported from the "host" module and called like C functions (ffi.rs);
/// sleep, random, random_below and clock need no compiler and live in warp-runtime (runtime/src/host_words.rs)
pub use warp_runtime::host_words::{
	CLIPBOARD_TEXT, CLOCK, EXIT, FETCH_HANDLER_PREFIX, FILE_HANDLER_PREFIX, GPU_COMPUTE, GPU_COMPUTE_LINEAR,
	GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER, HOST_LIBRARY, INTERRUPT_HANDLER, LOCAL_OFFSET, NOTIFY, PAGE_PATH,
	RANDOM, RANDOM_BELOW, RANDOM_SEED, SET_SYSTEM_VALUE, SHARED_HANDLER, SIGNAL_LISTEN, SIGNAL_POLL, SIGNAL_WATCH,
	SLEEP, SYSTEM_VALUE, TEXT_COVERAGE, TIMER_HANDLER_PREFIX,
};
pub const TASK_CONTROL: &str = "task_control";
/// `go f(x)` on a thread (tasks.rs) of a function of numbers, texts, characters or lists: task_spawn_values(name, [arguments]) → task id,
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
/// task_forks(): 1 while a new task would find a free core (fewer tasks run than there are cores), else 0: a parallel
/// let (parallel.rs) then calls in sequence (notes/bend.md, BendRT's fork cutoff)
pub const TASK_FORKS: &str = "task_forks";
/// task_split_mapf(kernel, xs) → a new block of xs's cells mapped by the float kernel (linear_arrays.rs), its slices in
/// instances of their own on threads of their own (tasks.rs split_map); 0 where the program maps alone (inside a task, in
/// the browser). `go xs.map(x => …)` of a linear float array (card go-split-kernel)
pub const TASK_SPLIT_MAPF: &str = "task_split_mapf";
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
/// Shared arrays (shared_arrays.rs, src/host/shared.rs natively, host.js): shared_new(n) → array, shared_get(array, i),
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
/// parse_code(text): the code a text known only at run time holds, as data (card parse-runtime, lowering/words/parse_word.rs):
/// natively the Rust parser, in the browser run_block's compiler instance (host-compiler.js)
pub const PARSE_CODE: &str = "parse_code";
/// block·value(index): the number a block run at run time reads, of the values its run_block call passed
/// (compiler::run_time_blocks::eval_block): the block is compiled as a function of them, the same module for other values
pub const BLOCK_VALUE: &str = "block·value";
pub use warp_runtime::host_words::FOREIGN_CALL;
/// paint(pixels, width, height): the pixels (a list, row after row, nonzero ink, 0 paper) drawn on the canvas of the
/// browser playground (host.js); natively a PNG file (src/graphics/paint.rs)
pub const PAINT: &str = "paint";
/// `use draw` on the GPU (src/graphics/gpu_canvas.rs, lib/draw.warp): draw_shape(kind, a, b, c, d, color) adds a shape,
/// draw_show(width, height) shows the canvas with the shapes added since the last show
pub const DRAW_SHAPE: &str = "draw_shape";
pub const DRAW_SHOW: &str = "draw_show";
/// sound_samples(samples, count, rate): 16-bit mono samples offset by 32768 (lib/sound.warp) played by the browser playground
/// (host.js, WebAudio); natively a WAV file the system's player plays (src/sound/mod.rs)
pub const SOUND: &str = "sound_samples";
/// The host words whose first argument is a list of ints, read in one call of the module's list_to_ints
pub const INT_LIST_WORDS: [&str; 2] = [PAINT, SOUND];
/// Channels between programs (src/host/channels.rs): channel_listen(id, channel), channel_pending(id) → count,
/// channel_next(id) → the oldest message, channel_send(channel, value)
pub const CHANNEL_LISTEN: &str = "channel_listen";
pub const CHANNEL_PENDING: &str = "channel_pending";
pub const CHANNEL_NEXT: &str = "channel_next";
pub const CHANNEL_SEND: &str = "channel_send";
/// dispatch_event(name, detail): an emit no handler of a page's program receives, as a DOM CustomEvent on the page
/// (card page-custom-event, lowering/signals/event_signals.rs; host.js; natively the headless page keeps it)
pub const DISPATCH_EVENT: &str = "dispatch_event";
/// std_pure(module, member, arguments) and std_io(…): the standard library's adapters (src/modules/std_adapters.rs, host.js),
/// called by the words of lib/<module>.warp; std_pure's words have no effect (json), std_io's touch the outside
pub const STD_PURE: &str = "std_pure";
pub const STD_IO: &str = "std_io";
/// The custom section naming the std modules a program calls through std_pure / std_io, joined by the separator; the
/// browser host ships only their parts (host_parts.rs). A module named at run time is ANY_STD_MODULE: every part
pub const STD_MODULES_SECTION: &str = "warp.std_modules";
pub const STD_MODULES_SEPARATOR: &str = ",";
pub const ANY_STD_MODULE: &str = "*";
/// The custom section naming the program's own functions (`def`, lib/*.warp), joined by the separator: exported for tasks
/// and other modules; a built site without tasks moves those one route calls into its route's module (route_split.rs)
pub const USER_FUNCTIONS_SECTION: &str = "warp.user_functions";
pub const USER_FUNCTIONS_SEPARATOR: &str = "\n";
/// serve_routes(port, [[method, path, function] …]): `serve 8080 { get "/" { … } }` (src/web/web_server.rs), blocks while
/// it serves; ø once it stops
pub const SERVE_ROUTES: &str = "serve_routes";
/// response_header(name, value): `header "name" "value"` in a route's body, `status 418` the CGI header Status
/// (card web-route-header, lowering/web/serve.rs with_answer_headers): natively the route's HTTP answer carries it
pub const RESPONSE_HEADER: &str = "response_header";
pub const STATUS_HEADER: &str = "Status";
/// `users := fetch url` (src/web/fetches.rs): fetch_start(id, url) fetches without waiting, the handler on·fetch·id runs once
/// the reply arrived, fetch_reply(id) → [value, error]: the parsed JSON (else the text) and ø, or ø and the failure
/// `users := fetch url` (lowering/signals/fetch_signals.rs)
pub const FETCH_WORD: &str = "fetch";
pub const FETCH_START: &str = "fetch_start";
pub const FETCH_REPLY: &str = "fetch_reply";
/// The host words whose result is any Node, its kind decided at run time (held like a map value)
pub const ANY_VALUE_WORDS: [&str; 5] = [FETCH_REPLY, FOREIGN_CALL, STD_PURE, STD_IO, PARSE_CODE];
/// Channels inside one run (P155, notes/channels.md, tasks.rs Channels), Go's unbuffered channel: channel_new() → id,
/// channel_put(id, value) waits until a receiver took it, channel_take(id) waits for a value (ø once closed and empty),
/// channel_more(id) waits until a value is offered (1) or the channel is closed (0), channel_close(id)
pub const CHANNEL_WORDS: [&str; 5] = ["channel_new", "channel_put", "channel_take", "channel_more", "channel_close"];
/// The host words of tasks, their signals and channels: a module calling one runs tasks (tasks.rs), a function that
/// calls one has the effect Async (effects.rs)
pub const TASK_WORDS: [&str; 16] = [
	TASK_CONTROL,
	TASK_SPAWN_VALUES,
	TASK_AWAIT_VALUE,
	TASK_JOIN,
	TASK_FAILURE,
	TASK_STATUS,
	TASK_POLL,
	TASK_INSIDE,
	TASK_FORKS,
	TASK_SPLIT_MAPF,
	SIGNAL_SEND,
	CHANNEL_WORDS[0],
	CHANNEL_WORDS[1],
	CHANNEL_WORDS[2],
	CHANNEL_WORDS[3],
	CHANNEL_WORDS[4],
];
/// The host words that build a value in the program (tasks.rs Builders): it exports its constructors
pub const VALUE_GIVING_WORDS: [&str; 15] = [
	GPU_COMPUTE,
	GPU_RENDER,
	TEXT_COVERAGE,
	FETCH_REPLY,
	RUN_BLOCK,
	PARSE_CODE,
	FOREIGN_CALL,
	BLOCK_VALUE,
	CHANNEL_NEXT,
	CLIPBOARD_TEXT,
	PAGE_PATH,
	CHANNEL_WORDS[2],
	STD_PURE,
	STD_IO,
	SERVE_ROUTES,
];
use wasm_encoder::ValType::{F64, I32, I64};
const NODE: wasm_encoder::ValType = wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF);
/// name, parameters, results of each host word
pub const HOST_WORD_SIGNATURES: [(&str, &[wasm_encoder::ValType], &[wasm_encoder::ValType]); 66] = [
	(DRAW_SHAPE, &[I64, F64, F64, F64, F64, I64], &[]),
	(DRAW_SHOW, &[I64, I64], &[]),
	(GPU_COMPUTE, &[NODE, NODE, I64], &[NODE]),
	(GPU_RENDER, &[NODE, I64, I64, NODE], &[NODE]),
	(TEXT_COVERAGE, &[NODE, I64], &[NODE]),
	(GPU_COMPUTE_LINEAR, &[NODE, I64, I64], &[I64]),
	(GPU_MAP_LINEAR, &[NODE, I64, NODE, I64, I64, I64], &[I64]),
	(GPU_REDUCE_LINEAR, &[NODE, I64, NODE, I64, I64, I64], &[I64]),
	(FETCH_START, &[I64, NODE], &[]),
	(FETCH_REPLY, &[I64], &[NODE]),
	(SERVE_ROUTES, &[I64, NODE], &[NODE]),
	(RESPONSE_HEADER, &[NODE, NODE], &[]),
	(STD_PURE, &[NODE, NODE, NODE], &[NODE]),
	(STD_IO, &[NODE, NODE, NODE], &[NODE]),
	(CHANNEL_LISTEN, &[I64, NODE], &[]),
	(CHANNEL_PENDING, &[I64], &[I64]),
	(CHANNEL_NEXT, &[I64], &[NODE]),
	(CLIPBOARD_TEXT, &[], &[NODE]),
	(PAGE_PATH, &[], &[NODE]),
	(NOTIFY, &[NODE], &[]),
	(CHANNEL_SEND, &[NODE, NODE], &[]),
	(DISPATCH_EVENT, &[NODE, NODE], &[]),
	(GUARDED_CALL, &[I32, NODE], &[NODE]),
	(PAINT, &[NODE, I64, I64, NODE], &[]),
	(SOUND, &[NODE, I64, I64, I64], &[]),
	(RUN_BLOCK, &[NODE, NODE, NODE, NODE], &[NODE]),
	(PARSE_CODE, &[NODE], &[NODE]),
	(BLOCK_VALUE, &[I64], &[NODE]),
	(FOREIGN_CALL, &[NODE, NODE, NODE, NODE, NODE], &[NODE]),
	(SLEEP, &[I64], &[]),
	(RANDOM, &[], &[F64]),
	(RANDOM_BELOW, &[I64], &[I64]),
	(RANDOM_SEED, &[I64], &[]),
	(CLOCK, &[], &[I64]),
	(LOCAL_OFFSET, &[I64], &[I64]),
	(SIGNAL_POLL, &[], &[]),
	(SIGNAL_LISTEN, &[I64, I64, I64], &[]),
	(SIGNAL_WATCH, &[I64, I32], &[]),
	(SYSTEM_VALUE, &[I32], &[I64]),
	(SET_SYSTEM_VALUE, &[I32, I64], &[]),
	(EXIT, &[I64], &[]),
	(TASK_CONTROL, &[I64, I64], &[I64]),
	(TASK_SPAWN_VALUES, &[I32, NODE], &[I64]),
	(TASK_AWAIT_VALUE, &[I64], &[NODE]),
	(TASK_JOIN, &[I64], &[I64]),
	(TASK_FAILURE, &[I64], &[NODE]),
	(TASK_STATUS, &[I64], &[I64]),
	(TASK_POLL, &[], &[]),
	(TASK_INSIDE, &[], &[I64]),
	(TASK_FORKS, &[], &[I64]),
	(TASK_SPLIT_MAPF, &[I32, I64], &[I64]),
	(SIGNAL_SEND, &[I32, NODE], &[I64]),
	(SHARED_WORDS[0], &[I64], &[I64]),
	(SHARED_WORDS[1], &[I64, I64], &[I64]),
	(SHARED_WORDS[2], &[I64, I64, I64], &[I64]),
	(SHARED_WORDS[3], &[I64, I64, I64], &[I64]),
	(SHARED_WORDS[4], &[I64], &[I64]),
	(SHARED_WRITES, &[I64], &[I64]),
	(SHARED_FLOAT_WORDS[0], &[I64, I64], &[F64]),
	(SHARED_FLOAT_WORDS[1], &[I64, I64, F64], &[F64]),
	(SHARED_FLOAT_WORDS[2], &[I64, I64, F64], &[F64]),
	(CHANNEL_WORDS[0], &[], &[I64]),
	(CHANNEL_WORDS[1], &[I64, NODE], &[]),
	(CHANNEL_WORDS[2], &[I64], &[NODE]),
	(CHANNEL_WORDS[3], &[I64], &[I64]),
	(CHANNEL_WORDS[4], &[I64], &[]),
];
/// The host words: the names of HOST_WORD_SIGNATURES
pub const HOST_WORDS: [&str; HOST_WORD_SIGNATURES.len()] = {
	let mut words = [""; HOST_WORD_SIGNATURES.len()];
	let mut index = 0;
	while index < words.len() {
		words[index] = HOST_WORD_SIGNATURES[index].0;
		index += 1;
	}
	words
};

mod aliases;
use crate::extensions::numbers::Number;
use crate::extensions::utils::download_within;
use crate::lowering::nodes::children_rewritten;
use crate::node::{Bracket, Node, Separator};
use anyhow::Result;

#[cfg(feature = "native")]
pub mod channels;
#[cfg(feature = "native")]
pub mod database;
pub mod host_parts;
pub mod page_parts;
pub use aliases::*;
mod fetch;
pub use fetch::*;
#[cfg(feature = "native")]
mod native;
pub mod sandbox;
#[cfg(feature = "native")]
pub mod shared;
#[cfg(feature = "native")]
pub mod tasks;
#[cfg(feature = "native")]
pub use native::*;
