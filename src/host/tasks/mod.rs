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
	AnyRef, AsContextMut, Caller, Engine, Func, Global, Linker, Memory, Module, Rooted, StoreContextMut, Val, ValType,
};

/// The bytes of a linear array's header (its count) and of each of its cells (linear_arrays.rs)
mod channels;
use channels::*;
mod pool;
pub use pool::*;
mod values;
pub use values::*;

const CELL_BYTES: usize = 8;
/// How often the engine's epoch ticks while tasks run: a stop or pause takes effect within about this long
const EPOCH_TICK: Duration = Duration::from_millis(5);
use crate::host::TASK_WORDS;
/// The failure a stopped task ends with (TaskControl::checkpoint), told apart by task_status
const STOPPED: &str = "task stopped";
use crate::wasm_emitter::EXACT_BUILDERS;
use crate::wasm_emitter::NODE_AT;

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
	fn checkpoint(&self) -> wasmtime::Result<()> {
		let mut state = self.state.lock().expect("task control");
		while *state == Control::Pause {
			state = self.changed.wait(state).expect("task control");
		}
		match *state {
			Control::Stop => Err(task_failure(STOPPED.to_string())),
			_ => Ok(()),
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
	module.imports().any(|import| import.module() == HOST_LIBRARY && TASK_WORDS.contains(&import.name()))
}

/// Does the module (as bytes, before it is compiled) call the task words: its engine then checks epochs
/// (util::task_engine)
pub fn imports_tasks_in(bytes: &[u8]) -> bool {
	wasmparser::Parser::new(0).parse_all(bytes).filter_map(Result::ok).any(|payload| match payload {
		wasmparser::Payload::ImportSection(imports) => imports
			.into_imports()
			.filter_map(Result::ok)
			.any(|import| import.module == HOST_LIBRARY && TASK_WORDS.contains(&import.name)),
		_ => false,
	})
}

impl TaskTable {
	fn link_into(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		self.link_task_values(linker)?;
		self.link_task_state(linker)?;
		self.link_work_sharing(linker)?;
		self.link_channels(linker)
	}

	/// Spawning a task with its arguments, awaiting its value, joining it and reading its failure
	fn link_task_values(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
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
		Ok(())
	}

	/// Polling, the status of a task, whether the code runs in one and controlling one
	fn link_task_state(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
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
		linker.func_wrap(HOST_LIBRARY, crate::host::TASK_INSIDE, |caller: Caller<'_, HostState>| -> i64 {
			caller.data().in_task as i64
		})?;
		let controller = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_CONTROL, move |id: i64, operation: i64| -> i64 {
			controller.control(id, operation)
		})?;
		Ok(())
	}

	/// Whether a core is free to fork onto, a map split over the cores, a signal sent to the program
	fn link_work_sharing(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
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
		Ok(())
	}

	/// Whether the caller waits alone: the program, with every task finished (a task's waits end with the program)
	fn alone_in(self: &Arc<Self>, caller: &Caller<'_, HostState>) -> impl Fn() -> bool + use<> {
		let table = self.clone();
		let in_task = caller.data().in_task;
		move || !in_task && table.slots.lock().expect("task table").values().all(|slot| matches!(slot, Slot::Done(_)))
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
		let control = Arc::new(TaskControl { state: Mutex::new(Control::Run), changed: Condvar::new() });
		self.controls.lock().expect("task controls").insert(id, control.clone());
		let voice = crate::sound::voice(); // the task is a voice of its own, starting where this one stands
		self.running.fetch_add(1, Ordering::Relaxed);
		self.slots.lock().expect("task table").insert(id, Slot::Queued);
		self.pool.submit(TaskJob { id, function, arguments, captured, control, voice });
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
		self.slots.lock().expect("task table").insert(job.id, Slot::Done(result));
		self.finished.notify_all();
	}

	/// A fresh instance of the module for a task, with the same imports as the program
	fn instance(
		self: &Arc<Self>,
		control: Arc<TaskControl>,
	) -> Result<(wasmtime::Store<HostState>, wasmtime::Instance)> {
		let mut state = HostState::new();
		state.in_task = true;
		let (fuel, time) = (crate::util::fuel_budget(), crate::util::time_limit());
		let mut store =
			warp_runtime::engine::checked_store(&self.engine, state, fuel, time, move || control.checkpoint());
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
			slices
				.into_iter()
				.try_for_each(|slice| slice.join().unwrap_or_else(|_| Err(anyhow!("a slice of the map panicked"))))
		})?;
		Ok((target - CELL_BYTES) as i64)
	}

	/// The kernel's results for `cells` (f64s as bytes) into `results`, in an instance of their own
	fn map_slice(self: &Arc<Self>, kernel: &str, cells: &[u8], results: &mut [u8]) -> Result<()> {
		use crate::wasm_emitter::lists::linear_arrays::SPLIT_BLOCK_NEW;
		let control = Arc::new(TaskControl { state: Mutex::new(Control::Run), changed: Condvar::new() });
		let (mut store, instance) = self.instance(control)?;
		let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("no exported memory"))?;
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
		let callee =
			instance.get_func(&mut store, function).ok_or_else(|| anyhow!("no exported function {function}"))?;
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
					self.own.upgrade().ok_or_else(|| "task pool ended".to_string())?.run_job(job);
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

fn host_error(failure: anyhow::Error) -> wasmtime::Error {
	wasmtime::Error::msg(failure.to_string())
}

/// What went wrong in a task, in the words a failure of the main program gets (wasm_emitter::failed_run)
/// The task's function as the program wrote it: `f`, not its wrapper `f·node`; a go block as `go block 1`
fn task_name(function: &str) -> String {
	crate::go_blocks::written_name(function.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX).unwrap_or(function))
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
	let end = bytes[start..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |length| start + length);
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
	let header =
		memory.get(block as usize..block as usize + CELL_BYTES).ok_or_else(|| anyhow!("no linear array at {block}"))?;
	Ok(i64::from_le_bytes(header.try_into()?) as usize)
}

fn typed_export(caller: &mut Caller<'_, HostState>, name: &str) -> Result<wasmtime::TypedFunc<i64, i64>> {
	let function = caller
		.get_export(name)
		.and_then(wasmtime::Extern::into_func)
		.ok_or_else(|| anyhow!("no exported function {name}"))?;
	Ok(function.typed(&*caller)?)
}
