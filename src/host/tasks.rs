//! Real threads for `go` (user decision P33, notes/threads.md): `go f(x)` runs f in a new instance of the same module on
//! a fixed worker pool; `await job` waits for it. Values are copied: the arguments go in, the result comes out (a TaskValue
//! between the threads, rebuilt in each instance through its exported constructors); the task never changes the
//! caller's variables. The program calls the host words (declarations::resolve_tasks):
//! `task_spawn_values(name, [args])` / `task_await_value(id)` for numbers, texts, characters and lists, and `task_control(id, op)` for stop, pause and resume.
//! The task's instance starts with the spawning instance's capture globals: what its closures captured and the
//! program's globals (`capture·global·<name>`, card task-sound), as they are at the spawn.

use crate::host::{
	HostState, HOST_LIBRARY, TASK_AWAIT_VALUE, TASK_CONTROL, TASK_FAILURE, TASK_JOIN, TASK_PAUSE, TASK_SPAWN_VALUES,
	TASK_STATUS, TASK_STOP, TEXT_HEAP_EXPORT,
};
use crate::node::{Bracket, Node};
use crate::run::wasm_reader::Imports;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;
use wasmtime::{
	AnyRef, AsContextMut, Caller, Engine, Func, Global, Linker, Memory, Module, Rooted, StoreContextMut,
	UpdateDeadline, Val, ValType,
};

/// The bytes of a linear array's header (its count) and of each of its cells (linear_arrays.rs)
const CELL_BYTES: usize = 8;
/// How often the engine's epoch ticks while tasks run: a stop or pause takes effect within about this long
const EPOCH_TICK: Duration = Duration::from_millis(5);
/// Tasks may block in sleep, channels and host calls, so one worker per core would leave cores idle.
const TASK_WORKERS_PER_CORE: usize = 2;
/// The main program's epoch deadline: it never stops at an epoch check
pub const MAIN_EPOCH_DEADLINE: u64 = 1 << 62;
use crate::host::TASK_WORDS;
const CHANNEL_NEW: &str = crate::host::CHANNEL_WORDS[0];
const CHANNEL_PUT: &str = crate::host::CHANNEL_WORDS[1];
const CHANNEL_TAKE: &str = crate::host::CHANNEL_WORDS[2];
const CHANNEL_MORE: &str = crate::host::CHANNEL_WORDS[3];
const CHANNEL_CLOSE: &str = crate::host::CHANNEL_WORDS[4];
/// How often a waiting send or receive looks whether it waits forever (the program alone, or ended)
const CHANNEL_CHECK: Duration = Duration::from_millis(20);
/// The failure a stopped task ends with (TaskControl::checkpoint), told apart by task_status
const STOPPED: &str = "task stopped";
/// The exported constructors a value is rebuilt with in an instance
const CONSTRUCTORS: [&str; 11] = [
	"new_empty",
	"new_int",
	crate::wasm_emitter::NEW_BOOL,
	"new_float",
	"new_codepoint",
	"new_text",
	"new_symbol",
	"new_key",
	"new_list",
	crate::wasm_emitter::CLOSURE_REBUILD,
	NODE_AT,
];
use crate::wasm_emitter::EXACT_BUILDERS;
use crate::wasm_emitter::NODE_AT;
const LIMB_BITS: i64 = 32;
/// The node fields (CLAUDE.md "Serialization") and the closure's captured values (wasm_emitter/closures.rs)
const KIND_FIELD: usize = 0;
const DATA_FIELD: usize = 1;
const VALUE_FIELD: usize = 2;
const CAPTURED_FIELD: usize = 1;

/// A value crossing between threads: what a task takes and gives, free of any instance (a Node may hold data that
/// cannot leave its thread)
#[derive(Clone, Debug)]
pub enum TaskValue {
	Empty,
	Int(i64),
	Bool(bool),
	Float(f64),
	Char(char),
	Text(String),
	Symbol(String),
	/// the items and the list's info: its bracket and separator (wasm_emitter::list_info)
	List(Vec<TaskValue>, i64),
	/// left, the operator's code, right
	Key(Box<TaskValue>, i64, Box<TaskValue>),
	/// a function value: its target's name and its captured values, rebuilt by the module's closure_rebuild
	Closure(String, Box<TaskValue>),
	/// an exact number beyond the fixnums: numerator and denominator (1 for an integer)
	Exact(num_bigint::BigInt, num_bigint::BigInt),
	/// a value with where it was written: line, column and the comment before it (Node::written_at, card parse-word)
	Positioned(Box<TaskValue>, i64, i64, Option<String>),
}

impl TaskValue {
	pub(crate) fn of(node: &Node) -> Result<TaskValue> {
		use crate::extensions::numbers::Number;
		if let Some((line, column, comment)) = node.written_at() {
			let value = TaskValue::of(node.drop_meta())?;
			return Ok(TaskValue::Positioned(Box::new(value), line, column, comment));
		}
		Ok(match node.drop_meta() {
			Node::Empty => TaskValue::Empty,
			Node::True | Node::False => TaskValue::Bool(matches!(node.drop_meta(), Node::True)),
			Node::Number(Number::Int(n)) if crate::wasm_emitter::is_fixnum(*n) => TaskValue::Int(*n),
			Node::Number(Number::Int(n)) => TaskValue::Exact((*n).into(), 1.into()),
			Node::Number(Number::BigInt(n)) => TaskValue::Exact((*n).clone(), 1.into()),
			Node::Number(Number::Quotient(n, d)) => TaskValue::Exact((*n).into(), (*d).into()),
			Node::Number(Number::BigQuotient(q)) => TaskValue::Exact(q.numerator.clone(), q.denominator.clone()),
			Node::Number(Number::Float(x)) => TaskValue::Float(*x),
			Node::Char(c) => TaskValue::Char(*c),
			Node::Text(text) => TaskValue::Text(text.clone()),
			Node::Symbol(name) => TaskValue::Symbol(name.clone()),
			Node::List(items, bracket, separator) => TaskValue::List(
				items.iter().map(TaskValue::of).collect::<Result<_>>()?,
				crate::wasm_emitter::list_info(bracket, separator),
			),
			Node::Key(left, op, right) => TaskValue::Key(
				Box::new(TaskValue::of(left)?),
				crate::operators::op_to_code(op),
				Box::new(TaskValue::of(right)?),
			),
			other => return Err(anyhow!("{} cannot cross to another task yet", other.serialize())),
		})
	}
}

/// The constructors, memory and text heap of one instance: what a TaskValue is rebuilt with
pub(crate) struct Builders {
	functions: HashMap<&'static str, Func>,
	memory: Memory,
	heap: Option<Global>,
}

impl Builders {
	pub(crate) fn of(lookup: &mut dyn FnMut(&str) -> Option<wasmtime::Extern>) -> Result<Builders> {
		let mut functions = HashMap::new();
		for &name in CONSTRUCTORS.iter().chain(EXACT_BUILDERS.iter()) {
			if let Some(function) = lookup(name).and_then(|export| export.into_func()) {
				functions.insert(name, function);
			}
		}
		let memory = lookup("memory")
			.and_then(|export| export.into_memory())
			.ok_or_else(|| anyhow!("no memory export"))?;
		Ok(Builders {
			functions,
			memory,
			heap: lookup(TEXT_HEAP_EXPORT).and_then(|export| export.into_global()),
		})
	}

	fn call(&self, name: &str, arguments: &[Val], store: &mut StoreContextMut<'_, HostState>) -> Result<Val> {
		self.call_into(name, arguments, Val::AnyRef(None), store)
	}

	fn call_into(
		&self,
		name: &str,
		arguments: &[Val],
		result: Val,
		store: &mut StoreContextMut<'_, HostState>,
	) -> Result<Val> {
		let function = self
			.functions
			.get(name)
			.ok_or_else(|| anyhow!("the module exports no {name}"))?;
		let mut results = [result];
		function.call(&mut *store, arguments, &mut results)?;
		Ok(results[0])
	}

	fn int_operation(&self, name: &str, a: i64, b: i64, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
		Ok(self
			.call_into(name, &[Val::I64(a), Val::I64(b)], Val::I64(0), store)?
			.unwrap_i64())
	}

	/// The Int handle of an integer: a fixnum as it is, a larger one composed from 32-bit limbs
	fn integer_handle(&self, n: &num_bigint::BigInt, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
		use num_traits::ToPrimitive;
		if let Some(small) = n.to_i64().filter(|small| crate::wasm_emitter::is_fixnum(*small)) {
			return Ok(small);
		}
		let (sign, limbs) = n.to_u32_digits();
		let mut handle = 0;
		for limb in limbs.iter().rev() {
			handle = self.int_operation(EXACT_BUILDERS[0], handle, LIMB_BITS, store)?;
			handle = self.int_operation(EXACT_BUILDERS[1], handle, *limb as i64, store)?;
		}
		match sign {
			num_bigint::Sign::Minus => self.int_operation(EXACT_BUILDERS[2], 0, handle, store),
			_ => Ok(handle),
		}
	}

	/// The Int handle of an exact number in this instance: numerator / denominator
	fn exact_handle(
		&self,
		numerator: &num_bigint::BigInt,
		denominator: &num_bigint::BigInt,
		store: &mut StoreContextMut<'_, HostState>,
	) -> Result<i64> {
		let numerator = self.integer_handle(numerator, store)?;
		if denominator == &num_bigint::BigInt::from(1) {
			return Ok(numerator);
		}
		let denominator = self.integer_handle(denominator, store)?;
		self.int_operation(EXACT_BUILDERS[3], numerator, denominator, store)
	}

	/// An Int of this instance as a value of its own: a fixnum as it is, a handle (a big integer, a ratio) read
	/// through the instance's Int node
	fn integer(&self, n: i64, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		if crate::wasm_emitter::is_fixnum(n) {
			return Ok(TaskValue::Int(n));
		}
		let node = self.call("new_int", &[Val::I64(n)], store)?;
		TaskValue::of(&crate::run::wasm_reader::node_in(&node, store, self.memory))
	}

	/// A value as an Int of this instance: a fixnum, or the handle of an exact number
	fn int_of(&self, value: &TaskValue, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
		match value {
			TaskValue::Int(n) => Ok(*n),
			TaskValue::Exact(numerator, denominator) => self.exact_handle(numerator, denominator, store),
			TaskValue::Positioned(value, ..) => self.int_of(value, store),
			other => Err(anyhow!("the task gave {other:?}, not an Int")),
		}
	}

	pub(crate) fn build(&self, value: &TaskValue, store: &mut StoreContextMut<'_, HostState>) -> Result<Val> {
		match value {
			TaskValue::Empty => self.call("new_empty", &[], store),
			TaskValue::Int(n) => self.call("new_int", &[Val::I64(*n)], store),
			// a module that has no bools (a page reading form JSON) takes a truth as the Int 1/0 its code reads
			TaskValue::Bool(truth) if !self.functions.contains_key(crate::wasm_emitter::NEW_BOOL) => {
				self.call("new_int", &[Val::I64(*truth as i64)], store)
			}
			TaskValue::Bool(truth) => self.call(crate::wasm_emitter::NEW_BOOL, &[Val::I64(*truth as i64)], store),
			TaskValue::Float(x) => self.call("new_float", &[Val::F64(x.to_bits())], store),
			TaskValue::Char(c) => self.call("new_codepoint", &[Val::I32(*c as i32)], store),
			TaskValue::Text(text) | TaskValue::Symbol(text) => {
				let (pointer, length) = crate::host::write_bytes(&self.memory, self.heap, store, text.as_bytes())?;
				let constructor = if matches!(value, TaskValue::Text(_)) {
					"new_text"
				} else {
					"new_symbol"
				};
				self.call(constructor, &[Val::I32(pointer as i32), Val::I32(length as i32)], store)
			}
			TaskValue::Key(left, op, right) => {
				let (left, right) = (self.build(left, store)?, self.build(right, store)?);
				self.call("new_key", &[left, right, Val::I64(*op)], store)
			}
			// ø, as the emitter makes `[]`: a null would not pass for a Node (a wrapper's empty argument list)
			TaskValue::List(items, _) if items.is_empty() => self.call("new_empty", &[], store),
			TaskValue::List(items, bracket) => items.iter().rev().try_fold(Val::AnyRef(None), |rest, item| {
				let first = self.build(item, store)?;
				self.call("new_list", &[first, rest, Val::I64(*bracket)], store)
			}),
			TaskValue::Exact(numerator, denominator) => {
				let handle = self.exact_handle(numerator, denominator, store)?;
				self.call("new_int", &[Val::I64(handle)], store)
			}
			TaskValue::Closure(name, captured) => {
				let name = self.build(&TaskValue::Symbol(name.clone()), store)?;
				let captured = match captured.as_ref() {
					TaskValue::Empty => Val::AnyRef(None),
					values => self.build(values, store)?,
				};
				self.call(crate::wasm_emitter::CLOSURE_REBUILD, &[name, captured], store)
			}
			// a module without node_at has no positions to keep (it quotes no code)
			TaskValue::Positioned(value, ..) if !self.functions.contains_key(NODE_AT) => self.build(value, store),
			TaskValue::Positioned(value, line, column, comment) => {
				let node = self.build(value, store)?;
				let comment = match comment {
					Some(comment) => self.build(&TaskValue::Text(comment.clone()), store)?,
					None => Val::AnyRef(None),
				};
				self.call(NODE_AT, &[node, Val::I64(*line), Val::I64(*column), comment], store)
			}
		}
	}

	/// A value of the instance, a function value with its captured values, a list item by item (cells walked)
	fn read_value(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		let Some(node) = value
			.unwrap_anyref()
			.and_then(|reference| reference.unwrap_struct(&*store).ok())
		else {
			return self.read(value, store);
		};
		let kind = node.field(&mut *store, KIND_FIELD)?.unwrap_i64();
		match (kind & crate::type_kinds::KIND_MASK) as u8 {
			tag if tag == crate::type_kinds::Kind::Function as u8 => {
				let name =
					crate::run::wasm_reader::node_in(&node.field(&mut *store, VALUE_FIELD)?, store, self.memory).name();
				let closure = node.field(&mut *store, DATA_FIELD)?;
				let captured = match closure
					.unwrap_anyref()
					.and_then(|reference| reference.unwrap_struct(&*store).ok())
				{
					Some(closure) => self.read_value(&closure.field(&mut *store, CAPTURED_FIELD)?, store)?,
					None => TaskValue::Empty,
				};
				Ok(TaskValue::Closure(name, Box::new(captured)))
			}
			tag if tag == crate::type_kinds::Kind::List as u8 => {
				let (mut items, mut cell) = (vec![], Some(node));
				while let Some(current) = cell {
					let first = current.field(&mut *store, DATA_FIELD)?;
					if first.unwrap_anyref().is_some() {
						items.push(self.read_value(&first, store)?);
					}
					let rest = current.field(&mut *store, VALUE_FIELD)?;
					cell = rest
						.unwrap_anyref()
						.and_then(|reference| reference.unwrap_struct(&*store).ok());
				}
				Ok(TaskValue::List(
					items,
					((kind >> 8) & 0xFF) | ((kind & crate::type_kinds::SEPARATOR_MASK) >> 8),
				))
			}
			_ => self.read(value, store),
		}
	}

	fn read(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		match value {
			Val::I64(n) => self.integer(*n, store),
			Val::F64(bits) => Ok(TaskValue::Float(warp_runtime::floats::canonical_nan(f64::from_bits(
				*bits,
			)))),
			Val::AnyRef(_) => TaskValue::of(&crate::run::wasm_reader::node_in(value, store, self.memory)),
			other => Err(anyhow!("a task gave {other:?}")),
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
enum Control {
	Run,
	Pause,
	Stop,
}

/// What the program asked of one task; the task reads it at every epoch check
struct TaskControl {
	state: Mutex<Control>,
	changed: Condvar,
}

impl TaskControl {
	/// At an epoch check: a paused task waits for resume (or stop), a stopped one ends with `task stopped`
	fn checkpoint(&self) -> wasmtime::Result<UpdateDeadline> {
		let mut state = self.state.lock().expect("task control");
		while *state == Control::Pause {
			state = self.changed.wait(state).expect("task control");
		}
		match *state {
			Control::Stop => Err(task_failure(STOPPED.to_string())),
			_ => Ok(UpdateDeadline::Continue(1)),
		}
	}

	fn set(&self, control: Control) {
		*self.state.lock().expect("task control") = control;
		self.changed.notify_all();
	}
}

/// A task that ended in an error: `await` gives it to the program as that error ("task f: index out of range")
#[derive(Debug, Clone)]
pub struct TaskFailure(pub String);

impl std::fmt::Display for TaskFailure {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for TaskFailure {}

/// A failure the program sees as the error of its call
pub(crate) fn task_failure(message: String) -> wasmtime::Error {
	wasmtime::Error::new(TaskFailure(message))
}

/// One local channel (P155): the values offered and not yet taken, in order; a send waits until the receives counted
/// past its own number
#[derive(Default)]
struct Channel {
	offered: std::collections::VecDeque<TaskValue>,
	sent: u64,
	taken: u64,
	closed: bool,
}

/// The local channels of a run (`ch = channel()`, notes/channels.md): Go's unbuffered channels between its tasks
#[derive(Default)]
struct Channels {
	channels: Mutex<HashMap<i64, Channel>>,
	changed: Condvar,
	next_id: AtomicI64,
	/// the program ended: a task still waiting on a channel stops (as Go drops its goroutines)
	ended: std::sync::atomic::AtomicBool,
}

impl Channels {
	fn open(&self) -> i64 {
		let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
		self.channels.lock().expect("channels").insert(id, Channel::default());
		id
	}

	/// Wait on channel `id` until `ready` gives an answer; `alone` says whether nothing else could ever change it
	fn wait<T>(
		&self,
		id: i64,
		alone: &dyn Fn() -> bool,
		waiting: &str,
		mut ready: impl FnMut(&mut Channel) -> Option<Result<T, String>>,
	) -> Result<T, String> {
		let mut channels = self.channels.lock().expect("channels");
		loop {
			let channel = channels.get_mut(&id).ok_or_else(|| format!("no channel {id}"))?;
			if let Some(answer) = ready(channel) {
				self.changed.notify_all();
				return answer;
			}
			if self.ended.load(Ordering::Relaxed) {
				return Err(STOPPED.to_string());
			}
			if alone() {
				return Err(format!("{waiting} waits forever: no task is left to answer it"));
			}
			channels = self.changed.wait_timeout(channels, CHANNEL_CHECK).expect("channels").0;
		}
	}

	/// `ch.send(v)`: offers v, then waits until a receiver took it
	fn put(&self, id: i64, value: TaskValue, alone: &dyn Fn() -> bool) -> Result<(), String> {
		let mut number = None;
		self.wait(id, alone, "ch.send", |channel| {
			if channel.closed && number.is_none() {
				return Some(Err("send on a closed channel".to_string()));
			}
			let mine = *number.get_or_insert_with(|| {
				channel.offered.push_back(value.clone());
				channel.sent += 1;
				channel.sent - 1
			});
			(channel.taken > mine).then_some(Ok(()))
		})
	}

	/// `ch.receive()`: the oldest value offered, waiting for one; None once the channel is closed and empty
	fn take(&self, id: i64, alone: &dyn Fn() -> bool) -> Result<Option<TaskValue>, String> {
		self.wait(id, alone, "ch.receive()", |channel| match channel.offered.pop_front() {
			Some(value) => {
				channel.taken += 1;
				Some(Ok(Some(value)))
			}
			None => channel.closed.then_some(Ok(None)),
		})
	}

	/// `for v in ch`: whether another value comes, waiting until one is offered or the channel is closed
	fn more(&self, id: i64, alone: &dyn Fn() -> bool) -> Result<bool, String> {
		self.wait(id, alone, "for … in ch", |channel| {
			match (channel.offered.is_empty(), channel.closed) {
				(false, _) => Some(Ok(true)),
				(true, closed) => closed.then_some(Ok(false)),
			}
		})
	}

	fn close(&self, id: i64) -> Result<(), String> {
		let mut channels = self.channels.lock().expect("channels");
		let channel = channels.get_mut(&id).ok_or_else(|| format!("no channel {id}"))?;
		if std::mem::replace(&mut channel.closed, true) {
			return Err("close of a closed channel".to_string());
		}
		self.changed.notify_all();
		Ok(())
	}

	fn end(&self) {
		let _channels = self.channels.lock().expect("channels");
		self.ended.store(true, Ordering::Relaxed);
		self.changed.notify_all();
	}
}

/// The value of a capture global (`capture·f·x`), carried from the spawning instance to the task's
#[derive(Clone, Debug)]
enum Captured {
	/// an Int as a value of its own: an exact number's handle means nothing in another instance (card error-undefined)
	Int(TaskValue),
	Float(u64),
	Value(TaskValue),
}

enum Slot {
	Queued,
	Running,
	Done(Result<TaskValue, String>),
}

struct TaskJob {
	id: i64,
	function: String,
	arguments: Vec<TaskValue>,
	captured: Vec<(String, Captured)>,
	control: Arc<TaskControl>,
	voice: crate::sound::Voice,
}

struct TaskPoolState {
	jobs: Mutex<VecDeque<TaskJob>>,
	ready: Condvar,
	stopping: AtomicBool,
}

struct TaskPool {
	state: Arc<TaskPoolState>,
	workers: Mutex<Vec<JoinHandle<()>>>,
}

thread_local! {
	static IN_TASK_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The fixed number of native workers used by one program run.
pub fn worker_count() -> usize {
	std::thread::available_parallelism().map_or(1, |cores| cores.get() * TASK_WORKERS_PER_CORE)
}

impl TaskPool {
	fn new() -> Self {
		Self {
			state: Arc::new(TaskPoolState {
				jobs: Mutex::new(VecDeque::new()),
				ready: Condvar::new(),
				stopping: AtomicBool::new(false),
			}),
			workers: Mutex::new(Vec::new()),
		}
	}

	fn start(&self, table: Weak<TaskTable>) -> Result<()> {
		let mut workers = self.workers.lock().expect("task workers");
		for number in 0..worker_count() {
			let state = self.state.clone();
			let table = table.clone();
			let worker = std::thread::Builder::new()
				.name(format!("warp-task-{number}"))
				.spawn(move || {
					IN_TASK_WORKER.with(|inside| inside.set(true));
					while let Some(job) = TaskPool::next(&state, true) {
						let Some(table) = table.upgrade() else { break };
						table.run_job(job);
					}
				})?;
			workers.push(worker);
		}
		Ok(())
	}

	fn submit(&self, job: TaskJob) {
		self.state.jobs.lock().expect("task queue").push_back(job);
		self.state.ready.notify_one();
	}

	fn help(&self) -> Option<TaskJob> {
		TaskPool::next(&self.state, false)
	}

	fn next(state: &TaskPoolState, wait: bool) -> Option<TaskJob> {
		let mut jobs = state.jobs.lock().expect("task queue");
		loop {
			if let Some(job) = jobs.pop_front() {
				return Some(job);
			}
			if !wait || state.stopping.load(Ordering::Relaxed) {
				return None;
			}
			jobs = state.ready.wait(jobs).expect("task queue");
		}
	}
}

impl Drop for TaskPool {
	fn drop(&mut self) {
		self.state.stopping.store(true, Ordering::Relaxed);
		self.state.ready.notify_all();
		let current = std::thread::current().id();
		for worker in self.workers.get_mut().expect("task workers").drain(..) {
			if worker.thread().id() != current {
				let _ = worker.join();
			}
		}
	}
}

/// The tasks of one program run, shared by every instance it starts
pub struct TaskTable {
	own: Weak<TaskTable>,
	engine: Engine,
	module: Module,
	imports: Imports,
	slots: Mutex<HashMap<i64, Slot>>,
	finished: Condvar,
	pool: TaskPool,
	/// the tasks whose result or failure the program read (await, join): a failure nobody read ends the run (join_all)
	taken: Mutex<std::collections::HashSet<i64>>,
	/// raises inside tasks for the starting thread: the handler's node wrapper and the argument list (deliver)
	signals: Mutex<Vec<(String, TaskValue)>>,
	controls: Mutex<HashMap<i64, Arc<TaskControl>>>,
	next_id: AtomicI64,
	/// the tasks of the run still running, nested ones too (task_forks)
	running: AtomicI64,
	/// the shared arrays of the run, linked into every task (shared.rs)
	shared: Option<Arc<crate::host::shared::SharedArrays>>,
	channels: Channels,
}

/// Link task_spawn and task_await for `module`, sharing one table with every task it starts
pub fn link(
	linker: &mut Linker<HostState>,
	engine: &Engine,
	module: &Module,
	imports: Imports,
	shared: Option<Arc<crate::host::shared::SharedArrays>>,
) -> Result<Arc<TaskTable>> {
	let table = Arc::new_cyclic(|own| TaskTable {
		own: own.clone(),
		engine: engine.clone(),
		module: module.clone(),
		imports,
		slots: Mutex::default(),
		finished: Condvar::new(),
		pool: TaskPool::new(),
		taken: Mutex::default(),
		signals: Mutex::default(),
		controls: Mutex::default(),
		next_id: AtomicI64::new(1),
		running: AtomicI64::new(0),
		shared,
		channels: Channels::default(),
	});
	table.link_into(linker)?;
	table.pool.start(Arc::downgrade(&table))?;
	tick_while_running(Arc::downgrade(&table));
	Ok(table)
}

/// The epoch advances while the table lives, so tasks reach their checks (stop, pause)
/// The machine's cores: a parallel let forks while fewer tasks run (task_forks)
fn cores() -> i64 {
	worker_count() as i64
}

fn tick_while_running(table: Weak<TaskTable>) {
	std::thread::spawn(move || loop {
		std::thread::sleep(EPOCH_TICK);
		match table.upgrade() {
			Some(table) => table.engine.increment_epoch(),
			None => break,
		}
	});
}

/// Does the module call the task words
pub fn imports_tasks(module: &Module) -> bool {
	module
		.imports()
		.any(|import| import.module() == HOST_LIBRARY && TASK_WORDS.contains(&import.name()))
}

/// Does the module (as bytes, before it is compiled) call the task words: its engine then checks epochs
/// (util::task_engine)
pub fn imports_tasks_in(bytes: &[u8]) -> bool {
	wasmparser::Parser::new(0)
		.parse_all(bytes)
		.filter_map(Result::ok)
		.any(|payload| match payload {
			wasmparser::Payload::ImportSection(imports) => imports
				.into_imports()
				.filter_map(Result::ok)
				.any(|import| import.module == HOST_LIBRARY && TASK_WORDS.contains(&import.name)),
			_ => false,
		})
}

impl TaskTable {
	fn link_into(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		let value_spawner = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			TASK_SPAWN_VALUES,
			move |mut caller: Caller<'_, HostState>,
			      name: i32,
			      arguments: Option<Rooted<AnyRef>>|
			      -> wasmtime::Result<i64> {
				let name = c_string(&mut caller, name).map_err(host_error)?;
				let unreadable = |failure: anyhow::Error| task_failure(format!("task {}: {failure}", task_name(&name)));
				let arguments = match read_in(&mut caller, arguments).map_err(unreadable)? {
					TaskValue::List(items, _) => items,
					TaskValue::Empty => vec![],
					single => vec![single],
				};
				let captured = value_spawner.captured(&mut caller).map_err(host_error)?;
				Ok(value_spawner.spawn(name, arguments, captured))
			},
		)?;
		let value_awaiter = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			TASK_AWAIT_VALUE,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<Option<Rooted<AnyRef>>> {
				let value = value_awaiter.take(id).map_err(task_failure)?;
				build_in(&mut caller, &value)
			},
		)?;
		// every await joins first: the raises the task forwarded run here, before its value is read
		let joiner = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			TASK_JOIN,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<i64> {
				let failed = joiner.take(id).is_err() as i64;
				joiner.deliver_to(&mut caller).map_err(host_error)?;
				Ok(failed)
			},
		)?;
		let reporter = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			TASK_FAILURE,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<Option<Rooted<AnyRef>>> {
				let message = reporter.take(id).err().unwrap_or_default();
				build_in(&mut caller, &TaskValue::Text(message))
			},
		)?;
		// natively the epoch checks pause a task: a loop start only runs the forwarded raises
		let poller = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			crate::host::TASK_POLL,
			move |mut caller: Caller<'_, HostState>| -> wasmtime::Result<()> {
				poller.deliver_to(&mut caller).map_err(host_error)
			},
		)?;
		let observer = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			TASK_STATUS,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<i64> {
				let status = observer.status(id);
				observer.deliver_to(&mut caller).map_err(host_error)?;
				Ok(status)
			},
		)?;
		linker.func_wrap(
			HOST_LIBRARY,
			crate::host::TASK_INSIDE,
			|caller: Caller<'_, HostState>| -> i64 { caller.data().in_task as i64 },
		)?;
		let forker = self.clone();
		linker.func_wrap(HOST_LIBRARY, crate::host::TASK_FORKS, move || -> i64 {
			(forker.running.load(Ordering::Relaxed) < cores()) as i64
		})?;
		let splitter = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			crate::host::TASK_SPLIT_MAPF,
			move |mut caller: Caller<'_, HostState>, kernel: i32, source: i64| -> wasmtime::Result<i64> {
				let kernel = c_string(&mut caller, kernel).map_err(host_error)?;
				splitter.split_map(&mut caller, &kernel, source).map_err(host_error)
			},
		)?;
		let sender = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			crate::host::SIGNAL_SEND,
			move |mut caller: Caller<'_, HostState>,
			      handler: i32,
			      arguments: Option<Rooted<AnyRef>>|
			      -> wasmtime::Result<i64> {
				let handler = c_string(&mut caller, handler).map_err(host_error)?;
				let arguments = read_in(&mut caller, arguments).map_err(host_error)?;
				sender.signals.lock().expect("task signals").push((handler, arguments));
				Ok(0)
			},
		)?;
		let controller = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_CONTROL, move |id: i64, operation: i64| -> i64 {
			controller.control(id, operation)
		})?;
		self.link_channels(linker)?;
		Ok(())
	}

	/// The local channel words (Channels); the program waiting on a channel while no task runs any more waits forever:
	/// an error, as Go's "all goroutines are asleep"
	fn link_channels(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		let opener = self.clone();
		linker.func_wrap(HOST_LIBRARY, CHANNEL_NEW, move || -> i64 { opener.channels.open() })?;
		let sender = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_PUT,
			move |mut caller: Caller<'_, HostState>, id: i64, value: Option<Rooted<AnyRef>>| -> wasmtime::Result<()> {
				let value = read_in(&mut caller, value).map_err(host_error)?;
				let alone = sender.alone_in(&caller);
				sender.channels.put(id, value, &alone).map_err(task_failure)
			},
		)?;
		let receiver = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_TAKE,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<Option<Rooted<AnyRef>>> {
				let alone = receiver.alone_in(&caller);
				let value = receiver
					.channels
					.take(id, &alone)
					.map_err(task_failure)?
					.unwrap_or(TaskValue::Empty);
				build_in(&mut caller, &value)
			},
		)?;
		let looper = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_MORE,
			move |caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<i64> {
				let alone = looper.alone_in(&caller);
				looper.channels.more(id, &alone).map(i64::from).map_err(task_failure)
			},
		)?;
		let closer = self.clone();
		linker.func_wrap(HOST_LIBRARY, CHANNEL_CLOSE, move |id: i64| -> wasmtime::Result<()> {
			closer.channels.close(id).map_err(task_failure)
		})?;
		Ok(())
	}

	/// Whether the caller waits alone: the program, with every task finished (a task's waits end with the program)
	fn alone_in(self: &Arc<Self>, caller: &Caller<'_, HostState>) -> impl Fn() -> bool + use<> {
		let table = self.clone();
		let in_task = caller.data().in_task;
		move || {
			!in_task
				&& table
					.slots
					.lock()
					.expect("task table")
					.values()
					.all(|slot| matches!(slot, Slot::Done(_)))
		}
	}

	fn deliver_to(&self, caller: &mut Caller<'_, HostState>) -> Result<()> {
		self.deliver(caller, &mut |caller, name| caller.get_export(name))
	}

	/// The raises tasks forwarded (signal_send), run in the program's instance by their handlers' node wrappers, in the
	/// order they came; a task's own instance leaves them to the program
	pub fn deliver<T: AsContextMut<Data = HostState>>(
		&self,
		context: &mut T,
		lookup: &mut dyn FnMut(&mut T, &str) -> Option<wasmtime::Extern>,
	) -> Result<()> {
		if context.as_context_mut().data().in_task {
			return Ok(());
		}
		loop {
			let pending = std::mem::take(&mut *self.signals.lock().expect("task signals"));
			if pending.is_empty() {
				return Ok(());
			}
			for (handler, arguments) in pending {
				let builders = Builders::of(&mut |name| lookup(context, name))?;
				let function = lookup(context, &handler)
					.and_then(wasmtime::Extern::into_func)
					.ok_or_else(|| anyhow!("no exported handler {handler}"))?;
				let mut store = context.as_context_mut();
				let argument = builders.build(&arguments, &mut store)?;
				let mut results = zero_results(&function.ty(&store));
				function.call(&mut store, &[argument], &mut results)?;
			}
		}
	}

	/// The capture globals of the spawning instance, as their values now
	fn captured(&self, caller: &mut Caller<'_, HostState>) -> Result<Vec<(String, Captured)>> {
		let names: Vec<String> = self
			.module
			.exports()
			.map(|export| export.name().to_string())
			.filter(|name| name.starts_with(crate::wasm_emitter::CAPTURE_EXPORT_PREFIX))
			.collect();
		if names.is_empty() {
			return Ok(vec![]);
		}
		let builders = Builders::of(&mut |export| caller.get_export(export))?;
		let mut captured = vec![];
		for name in names {
			let Some(global) = caller.get_export(&name).and_then(|export| export.into_global()) else {
				continue;
			};
			let value = match global.get(&mut *caller) {
				Val::I64(n) => Captured::Int(builders.integer(n, &mut caller.as_context_mut())?),
				Val::F64(bits) => Captured::Float(warp_runtime::floats::canonical_nan(f64::from_bits(bits)).to_bits()),
				reference => Captured::Value(builders.read_value(&reference, &mut caller.as_context_mut())?),
			};
			captured.push((name, value));
		}
		Ok(captured)
	}

	fn spawn(self: &Arc<Self>, function: String, arguments: Vec<TaskValue>, captured: Vec<(String, Captured)>) -> i64 {
		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		let control = Arc::new(TaskControl {
			state: Mutex::new(Control::Run),
			changed: Condvar::new(),
		});
		self.controls.lock().expect("task controls").insert(id, control.clone());
		let voice = crate::sound::voice(); // the task is a voice of its own, starting where this one stands
		self.running.fetch_add(1, Ordering::Relaxed);
		self.slots.lock().expect("task table").insert(id, Slot::Queued);
		self.pool.submit(TaskJob {
			id,
			function,
			arguments,
			captured,
			control,
			voice,
		});
		id
	}

	fn run_job(self: &Arc<Self>, job: TaskJob) {
		if let Some(slot) = self.slots.lock().expect("task table").get_mut(&job.id) {
			*slot = Slot::Running;
		}
		crate::sound::join_voice(job.voice);
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			self.run(&job.function, &job.arguments, &job.captured, job.control)
				.map_err(|failure| format!("task {}: {}", task_name(&job.function), failure_message(failure)))
		}))
		.unwrap_or_else(|_| Err("the task panicked".to_string()));
		self.running.fetch_sub(1, Ordering::Relaxed);
		self.slots
			.lock()
			.expect("task table")
			.insert(job.id, Slot::Done(result));
		self.finished.notify_all();
	}

	/// A fresh instance of the module for a task, with the same imports as the program
	fn instance(
		self: &Arc<Self>,
		control: Arc<TaskControl>,
	) -> Result<(wasmtime::Store<HostState>, wasmtime::Instance)> {
		let mut state = HostState::new();
		state.in_task = true;
		let mut store = crate::util::fueled_store(&self.engine, state);
		store.epoch_deadline_callback(move |_| control.checkpoint());
		store.set_epoch_deadline(1);
		let mut linker = Linker::new(&self.engine);
		crate::run::wasm_reader::link_imports(&mut linker, &self.engine, &self.module, self.imports)?;
		if let Some(shared) = &self.shared {
			shared.link_into(&mut linker)?;
		}
		self.link_into(&mut linker)?;
		let instance = linker.instantiate(&mut store, &self.module)?;
		Ok((store, instance))
	}

	/// `go xs.map(x => …)` of a linear float array (card go-split-kernel): a new block of the caller's, its cells the
	/// kernel's results for xs's, mapped in slices by instances of their own on threads of their own; 0 inside a task,
	/// where the caller maps alone
	fn split_map(self: &Arc<Self>, caller: &mut Caller<'_, HostState>, kernel: &str, source: i64) -> Result<i64> {
		use crate::wasm_emitter::lists::linear_arrays::{SPLIT_BLOCK_NEW, SPLIT_KERNEL_PREFIX};
		if caller.data().in_task {
			return Ok(0);
		}
		let memory = caller_memory(caller)?;
		let count = cell_count(memory.data(&*caller), source)?;
		let target = typed_export(caller, SPLIT_BLOCK_NEW)?.call(&mut *caller, count as i64)?;
		let bytes = count * CELL_BYTES;
		let (source, target) = (source as usize + CELL_BYTES, target as usize + CELL_BYTES);
		if target < source + bytes {
			return Err(anyhow!("the mapped block lies before its source"));
		}
		let (before, after) = memory.data_mut(&mut *caller).split_at_mut(target);
		let cells = &before[source..source + bytes];
		let results = &mut after[..bytes];
		let slice_bytes = count.div_ceil(split_threads()) * CELL_BYTES;
		let kernel = format!("{SPLIT_KERNEL_PREFIX}{kernel}");
		std::thread::scope(|scope| {
			let slices: Vec<_> = cells
				.chunks(slice_bytes)
				.zip(results.chunks_mut(slice_bytes))
				.map(|(cells, results)| scope.spawn(|| self.map_slice(&kernel, cells, results)))
				.collect();
			slices.into_iter().try_for_each(|slice| {
				slice
					.join()
					.unwrap_or_else(|_| Err(anyhow!("a slice of the map panicked")))
			})
		})?;
		Ok((target - CELL_BYTES) as i64)
	}

	/// The kernel's results for `cells` (f64s as bytes) into `results`, in an instance of their own
	fn map_slice(self: &Arc<Self>, kernel: &str, cells: &[u8], results: &mut [u8]) -> Result<()> {
		use crate::wasm_emitter::lists::linear_arrays::SPLIT_BLOCK_NEW;
		let control = Arc::new(TaskControl {
			state: Mutex::new(Control::Run),
			changed: Condvar::new(),
		});
		let (mut store, instance) = self.instance(control)?;
		let memory = instance
			.get_memory(&mut store, "memory")
			.ok_or_else(|| anyhow!("no exported memory"))?;
		let new = instance.get_typed_func::<i64, i64>(&mut store, SPLIT_BLOCK_NEW)?;
		let block = new.call(&mut store, (cells.len() / CELL_BYTES) as i64)? as usize + CELL_BYTES;
		memory.data_mut(&mut store)[block..block + cells.len()].copy_from_slice(cells);
		let mapped = instance.get_typed_func::<i64, i64>(&mut store, kernel)?;
		let mapped = mapped.call(&mut store, (block - CELL_BYTES) as i64)? as usize + CELL_BYTES;
		results.copy_from_slice(&memory.data(&store)[mapped..mapped + results.len()]);
		Ok(())
	}

	/// f(arguments) in a fresh instance of the module, with the same imports as the program
	fn run(
		self: &Arc<Self>,
		function: &str,
		arguments: &[TaskValue],
		captured: &[(String, Captured)],
		control: Arc<TaskControl>,
	) -> Result<TaskValue> {
		let (mut store, instance) = self.instance(control)?;
		let callee = instance
			.get_func(&mut store, function)
			.ok_or_else(|| anyhow!("no exported function {function}"))?;
		let builders = Builders::of(&mut |export| instance.get_export(&mut store, export))
			.map_err(|failure| anyhow!("{failure}"))?;
		// what the spawning instance's closures captured, as it had it
		for (name, value) in captured {
			let Some(global) = instance.get_global(&mut store, name) else {
				continue;
			};
			let value = match value {
				Captured::Int(value) => Val::I64(builders.int_of(value, &mut store.as_context_mut())?),
				Captured::Float(bits) => Val::F64(*bits),
				Captured::Value(value) => builders.build(value, &mut store.as_context_mut())?,
			};
			global.set(&mut store, value)?;
		}
		let signature = callee.ty(&store);
		let values: Vec<Val> = match signature.params().next() {
			// a wrapper of values (declarations::with_node_wrappers): the one argument list
			Some(ValType::Ref(_))
				if signature.params().len() == 1 && function.ends_with(crate::declarations::NODE_WRAPPER_SUFFIX) =>
			{
				vec![builders.build(
					&TaskValue::List(arguments.to_vec(), crate::wasm_emitter::bracket_info(&Bracket::Square)),
					&mut store.as_context_mut(),
				)?]
			}
			_ => signature
				.params()
				.zip(arguments)
				.map(|(parameter, argument)| match (parameter, argument) {
					(ValType::I64, TaskValue::Int(_) | TaskValue::Exact(..)) => {
						Ok(Val::I64(builders.int_of(argument, &mut store.as_context_mut())?))
					}
					(ValType::Ref(_), argument) => builders.build(argument, &mut store.as_context_mut()),
					(parameter, argument) => Err(anyhow!("{function} takes {parameter}, got {argument:?}")),
				})
				.collect::<Result<_>>()?,
		};
		let mut results = zero_results(&signature);
		// a raised error leaves its message in trap_detail, as in the program
		crate::run::wasm_reader::with_trap_detail(
			callee.call(&mut store, &values, &mut results),
			&mut store,
			&instance,
		)?;
		match results.first() {
			Some(result) => builders.read(result, &mut store.as_context_mut()),
			None => Ok(TaskValue::Empty),
		}
	}

	/// `stop job`, `pause job`, `resume job`: 1 when the task exists, else 0
	fn control(&self, id: i64, operation: i64) -> i64 {
		let Some(control) = self.controls.lock().expect("task controls").get(&id).cloned() else {
			return 0;
		};
		control.set(match operation {
			TASK_STOP => Control::Stop,
			TASK_PAUSE => Control::Pause,
			_ => Control::Run,
		});
		1
	}

	/// Where the task is, without waiting for it: running, finished, failed, stopped or paused (host TASK_* codes)
	fn status(&self, id: i64) -> i64 {
		let paused = self
			.controls
			.lock()
			.expect("task controls")
			.get(&id)
			.is_some_and(|control| *control.state.lock().expect("task control") == Control::Pause);
		match self.slots.lock().expect("task table").get(&id) {
			Some(Slot::Done(Ok(_))) => crate::host::TASK_FINISHED,
			Some(Slot::Done(Err(message))) if message.ends_with(STOPPED) => crate::host::TASK_STOPPED,
			Some(Slot::Done(Err(_))) => crate::host::TASK_FAILED,
			_ if paused => crate::host::TASK_PAUSED,
			_ => 0,
		}
	}

	/// The result of the task: joins it the first time, then remembers it
	fn await_task(&self, id: i64) -> Result<TaskValue, String> {
		loop {
			let mut slots = self.slots.lock().expect("task table");
			match slots.get(&id) {
				Some(Slot::Done(result)) => return result.clone(),
				Some(Slot::Queued | Slot::Running) => {}
				None => return Err(format!("no task {id}")),
			}
			drop(slots);
			if IN_TASK_WORKER.with(std::cell::Cell::get) {
				if let Some(job) = self.pool.help() {
					self.own
						.upgrade()
						.ok_or_else(|| "task pool ended".to_string())?
						.run_job(job);
					continue;
				}
			}
			slots = self.slots.lock().expect("task table");
			if !matches!(slots.get(&id), Some(Slot::Done(_))) {
				drop(self.finished.wait(slots).expect("task table"));
			}
		}
	}

	/// The result of the task, read by the program
	fn take(&self, id: i64) -> Result<TaskValue, String> {
		self.taken.lock().expect("taken tasks").insert(id);
		self.await_task(id)
	}

	/// The program ended: tasks nobody awaited still finish (their output is part of the run); the first failure nobody
	/// read is the run's error, as loud as one in the program (a stopped task was stopped on purpose)
	pub fn join_all(&self) -> Result<(), TaskFailure> {
		self.channels.end();
		let mut ids: Vec<i64> = self.slots.lock().expect("task table").keys().copied().collect();
		ids.sort();
		let mut unread = None;
		for id in ids {
			if let Err(message) = self.await_task(id) {
				if unread.is_none()
					&& !message.ends_with(STOPPED)
					&& !self.taken.lock().expect("taken tasks").contains(&id)
				{
					unread = Some(TaskFailure(message));
				}
			}
		}
		unread.map_or(Ok(()), Err)
	}
}

/// A task value as the calling module's GC value
fn build_in(caller: &mut Caller<'_, HostState>, value: &TaskValue) -> wasmtime::Result<Option<Rooted<AnyRef>>> {
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
	let built = builders
		.build(value, &mut caller.as_context_mut())
		.map_err(host_error)?;
	Ok(built.unwrap_anyref().copied())
}

/// The calling module's GC value as a task value
fn read_in(caller: &mut Caller<'_, HostState>, value: Option<Rooted<AnyRef>>) -> Result<TaskValue> {
	Builders::of(&mut |export| caller.get_export(export))?.read_value(&Val::AnyRef(value), &mut caller.as_context_mut())
}

fn host_error(failure: anyhow::Error) -> wasmtime::Error {
	wasmtime::Error::msg(failure.to_string())
}

/// What went wrong in a task, in the words a failure of the main program gets (wasm_emitter::failed_run)
/// The task's function as the program wrote it: `f`, not its wrapper `f·node`; a go block as `go block 1`
fn task_name(function: &str) -> String {
	crate::go_blocks::written_name(
		function
			.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX)
			.unwrap_or(function),
	)
}

pub(crate) fn failure_message(failure: anyhow::Error) -> String {
	match crate::wasm_emitter::failed_run(failure) {
		crate::node::Node::Error(message) => message.drop_meta().to_string(),
		other => other.serialize(),
	}
}

/// The zero-terminated text at `pointer` in the caller's memory: the function name
pub(crate) fn c_string(caller: &mut Caller<'_, HostState>, pointer: i32) -> Result<String> {
	let memory = caller
		.get_export("memory")
		.and_then(|export| export.into_memory())
		.ok_or_else(|| anyhow!("no memory export"))?;
	let bytes = memory.data(&*caller);
	let start = pointer as usize;
	let end = bytes[start..]
		.iter()
		.position(|byte| *byte == 0)
		.map_or(bytes.len(), |length| start + length);
	Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}

/// The result slots of a call, zero or null by type
fn zero_results(signature: &wasmtime::FuncType) -> Vec<Val> {
	signature
		.results()
		.map(|result| match result {
			ValType::I64 => Val::I64(0),
			ValType::F64 => Val::F64(0),
			_ => Val::AnyRef(None),
		})
		.collect()
}

/// The threads a split map takes: one a core
fn split_threads() -> usize {
	std::thread::available_parallelism().map_or(1, usize::from)
}

fn caller_memory(caller: &mut Caller<'_, HostState>) -> Result<Memory> {
	match caller.get_export("memory") {
		Some(wasmtime::Extern::Memory(memory)) => Ok(memory),
		_ => Err(anyhow!("no exported memory")),
	}
}

/// The count in the header of the linear array at `block`
fn cell_count(memory: &[u8], block: i64) -> Result<usize> {
	let header = memory
		.get(block as usize..block as usize + CELL_BYTES)
		.ok_or_else(|| anyhow!("no linear array at {block}"))?;
	Ok(i64::from_le_bytes(header.try_into()?) as usize)
}

fn typed_export(caller: &mut Caller<'_, HostState>, name: &str) -> Result<wasmtime::TypedFunc<i64, i64>> {
	let function = caller
		.get_export(name)
		.and_then(wasmtime::Extern::into_func)
		.ok_or_else(|| anyhow!("no exported function {name}"))?;
	Ok(function.typed(&*caller)?)
}
