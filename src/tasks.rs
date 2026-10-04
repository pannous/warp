//! Real threads for `go` (user decision P33, notes/threads.md step 1): `go f(x)` of a function taking and giving Ints runs
//! f in a new instance of the same module on its own thread; `await job` joins it. Values are copied: the Int arguments
//! go in, the Int result comes out; the task's instance sees the module's initial variables, never the caller's.
//! The program calls the host words `task_spawn(name, a0, a1, a2, a3)` and `task_await(id)` (declarations::resolve_tasks).

use crate::host::{HostState, HOST_LIBRARY, MAX_TASK_ARGUMENTS, TASK_AWAIT, TASK_CONTROL, TASK_PAUSE, TASK_SPAWN, TASK_STOP};
use crate::wasm_reader::Imports;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;
use wasmtime::{Caller, Engine, Linker, Module, UpdateDeadline, Val};

/// How often the engine's epoch ticks while tasks run: a stop or pause takes effect within about this long
const EPOCH_TICK: Duration = Duration::from_millis(5);
/// The main program's epoch deadline: it never stops at an epoch check
pub const MAIN_EPOCH_DEADLINE: u64 = 1 << 62;
const TASK_WORDS: [&str; 3] = [TASK_SPAWN, TASK_AWAIT, TASK_CONTROL];

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
			Control::Stop => Err(wasmtime::Error::new(TaskFailure("task stopped".to_string()))),
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

enum Slot {
	Running(JoinHandle<Result<i64, String>>),
	Done(Result<i64, String>),
}

/// The tasks of one program run, shared by every instance it starts
pub struct TaskTable {
	engine: Engine,
	module: Module,
	imports: Imports,
	slots: Mutex<HashMap<i64, Slot>>,
	controls: Mutex<HashMap<i64, Arc<TaskControl>>>,
	next_id: AtomicI64,
}

/// Link task_spawn and task_await for `module`, sharing one table with every task it starts
pub fn link(linker: &mut Linker<HostState>, engine: &Engine, module: &Module, imports: Imports) -> Result<Arc<TaskTable>> {
	let table = Arc::new(TaskTable {
		engine: engine.clone(), module: module.clone(), imports, slots: Mutex::default(), controls: Mutex::default(), next_id: AtomicI64::new(1),
	});
	table.link_into(linker)?;
	tick_while_running(Arc::downgrade(&table));
	Ok(table)
}

/// The epoch advances while the table lives, so tasks reach their checks (stop, pause)
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
		wasmparser::Payload::ImportSection(imports) => {
			imports.into_imports().filter_map(Result::ok).any(|import| import.module == HOST_LIBRARY && TASK_WORDS.contains(&import.name))
		}
		_ => false,
	})
}

impl TaskTable {
	fn link_into(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		let spawner = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_SPAWN, move |mut caller: Caller<'_, HostState>, name: i32, a0: i64, a1: i64, a2: i64, a3: i64| -> wasmtime::Result<i64> {
			let name = c_string(&mut caller, name).map_err(|failure| wasmtime::Error::msg(failure.to_string()))?;
			Ok(spawner.spawn(name, [a0, a1, a2, a3]))
		})?;
		let controller = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_CONTROL, move |id: i64, operation: i64| -> i64 { controller.control(id, operation) })?;
		let awaiter = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_AWAIT, move |id: i64| -> wasmtime::Result<i64> {
			awaiter.await_task(id).map_err(|message| wasmtime::Error::new(TaskFailure(message)))
		})?;
		Ok(())
	}

	fn spawn(self: &Arc<Self>, function: String, arguments: [i64; MAX_TASK_ARGUMENTS]) -> i64 {
		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		let control = Arc::new(TaskControl { state: Mutex::new(Control::Run), changed: Condvar::new() });
		self.controls.lock().expect("task controls").insert(id, control.clone());
		let table = self.clone();
		let handle = std::thread::spawn(move || table.run(&function, &arguments, control).map_err(|failure| format!("task {function}: {}", failure_message(failure))));
		self.slots.lock().expect("task table").insert(id, Slot::Running(handle));
		id
	}

	/// f(arguments) in a fresh instance of the module, with the same imports as the program
	fn run(self: &Arc<Self>, function: &str, arguments: &[i64], control: Arc<TaskControl>) -> Result<i64> {
		let mut store = crate::util::fueled_store(&self.engine, HostState::new());
		store.epoch_deadline_callback(move |_| control.checkpoint());
		store.set_epoch_deadline(1);
		let mut linker = Linker::new(&self.engine);
		crate::wasm_reader::link_imports(&mut linker, &self.engine, &self.module, self.imports)?;
		self.link_into(&mut linker)?;
		let instance = linker.instantiate(&mut store, &self.module)?;
		let callee = instance.get_func(&mut store, function).ok_or_else(|| anyhow!("no exported function {function}"))?;
		let parameters = callee.ty(&store).params().len();
		let values: Vec<Val> = arguments.iter().take(parameters).map(|argument| Val::I64(*argument)).collect();
		let mut results = vec![Val::I64(0)];
		callee.call(&mut store, &values, &mut results)?;
		match results[0] {
			Val::I64(result) => Ok(result),
			ref other => Err(anyhow!("{function} gave {other:?}, not an Int")),
		}
	}

	/// `stop job`, `pause job`, `resume job`: 1 when the task exists, else 0
	fn control(&self, id: i64, operation: i64) -> i64 {
		let Some(control) = self.controls.lock().expect("task controls").get(&id).cloned() else { return 0 };
		control.set(match operation {
			TASK_STOP => Control::Stop,
			TASK_PAUSE => Control::Pause,
			_ => Control::Run,
		});
		1
	}

	/// The result of the task: joins it the first time, then remembers it
	fn await_task(&self, id: i64) -> Result<i64, String> {
		let slot = self.slots.lock().expect("task table").remove(&id).ok_or_else(|| format!("no task {id}"))?;
		let result = match slot {
			Slot::Running(handle) => handle.join().unwrap_or_else(|_| Err("the task panicked".to_string())),
			Slot::Done(result) => result,
		};
		self.slots.lock().expect("task table").insert(id, Slot::Done(result.clone()));
		result
	}

	/// The program ended: tasks nobody awaited still finish (their output is part of the run)
	pub fn join_all(&self) {
		let ids: Vec<i64> = self.slots.lock().expect("task table").keys().copied().collect();
		for id in ids {
			let _ = self.await_task(id);
		}
	}
}

/// What went wrong in a task, in the words a failure of the main program gets (wasm_emitter::failed_run)
fn failure_message(failure: anyhow::Error) -> String {
	match crate::wasm_emitter::failed_run(failure) {
		crate::node::Node::Error(message) => message.serialize().trim_matches('"').to_string(),
		other => other.serialize(),
	}
}

/// The zero-terminated text at `pointer` in the caller's memory: the function name
fn c_string(caller: &mut Caller<'_, HostState>, pointer: i32) -> Result<String> {
	let memory = caller.get_export("memory").and_then(|export| export.into_memory()).ok_or_else(|| anyhow!("no memory export"))?;
	let bytes = memory.data(&*caller);
	let start = pointer as usize;
	let end = bytes[start..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |length| start + length);
	Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}
