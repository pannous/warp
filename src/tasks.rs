//! Real threads for `go` (user decision P33, notes/threads.md): `go f(x)` runs f in a new instance of the same module on
//! its own thread; `await job` joins it. Values are copied: the arguments go in, the result comes out (a TaskValue
//! between the threads, rebuilt in each instance through its exported constructors); the task's instance sees the
//! module's initial variables, never the caller's. The program calls the host words (declarations::resolve_tasks):
//! `task_spawn(name, a0, a1, a2, a3)` / `task_await(id)` for functions of Ints, `task_spawn_values(name, [args])` /
//! `task_await_value(id)` for numbers, texts and characters, and `task_control(id, op)` for stop, pause and resume.

use crate::host::{HostState, HOST_LIBRARY, MAX_TASK_ARGUMENTS, TASK_AWAIT, TASK_AWAIT_VALUE, TASK_CONTROL, TASK_FAILURE, TASK_JOIN, TASK_PAUSE, TASK_SPAWN, TASK_SPAWN_VALUES, TASK_STATUS, TASK_STOP, TEXT_HEAP_EXPORT};
use crate::node::{Bracket, Node};
use crate::wasm_reader::Imports;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;
use wasmtime::{AnyRef, AsContextMut, Caller, Engine, Func, Global, Linker, Memory, Module, Rooted, StoreContextMut, UpdateDeadline, Val, ValType};

/// How often the engine's epoch ticks while tasks run: a stop or pause takes effect within about this long
const EPOCH_TICK: Duration = Duration::from_millis(5);
/// The main program's epoch deadline: it never stops at an epoch check
pub const MAIN_EPOCH_DEADLINE: u64 = 1 << 62;
const TASK_WORDS: [&str; 9] = [TASK_SPAWN, TASK_AWAIT, TASK_CONTROL, TASK_SPAWN_VALUES, TASK_AWAIT_VALUE, TASK_JOIN, TASK_FAILURE, TASK_STATUS, crate::host::TASK_POLL];
/// The failure a stopped task ends with (TaskControl::checkpoint), told apart by task_status
const STOPPED: &str = "task stopped";
/// The exported constructors a value is rebuilt with in an instance
const CONSTRUCTORS: [&str; 9] = ["new_empty", "new_int", "new_float", "new_codepoint", "new_text", "new_symbol", "new_key", "new_list",
	crate::wasm_emitter::CLOSURE_REBUILD];
use crate::wasm_emitter::EXACT_BUILDERS;
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
	Float(f64),
	Char(char),
	Text(String),
	Symbol(String),
	/// the items and the bracket's code (wasm_emitter::bracket_info)
	List(Vec<TaskValue>, i64),
	/// left, the operator's code, right
	Key(Box<TaskValue>, i64, Box<TaskValue>),
	/// a function value: its target's name and its captured values, rebuilt by the module's closure_rebuild
	Closure(String, Box<TaskValue>),
	/// an exact number beyond the fixnums: numerator and denominator (1 for an integer)
	Exact(num_bigint::BigInt, num_bigint::BigInt),
}

impl TaskValue {
	pub(crate) fn of(node: &Node) -> Result<TaskValue> {
		use crate::extensions::numbers::Number;
		Ok(match node.drop_meta() {
			Node::Empty => TaskValue::Empty,
			Node::Number(Number::Int(n)) if crate::wasm_emitter::is_fixnum(*n) => TaskValue::Int(*n),
			Node::Number(Number::Int(n)) => TaskValue::Exact((*n).into(), 1.into()),
			Node::Number(Number::BigInt(n)) => TaskValue::Exact((*n).clone(), 1.into()),
			Node::Number(Number::Quotient(n, d)) => TaskValue::Exact((*n).into(), (*d).into()),
			Node::Number(Number::BigQuotient(q)) => TaskValue::Exact(q.numerator.clone(), q.denominator.clone()),
			Node::Number(Number::Float(x)) => TaskValue::Float(*x),
			Node::Char(c) => TaskValue::Char(*c),
			Node::Text(text) => TaskValue::Text(text.clone()),
			Node::Symbol(name) => TaskValue::Symbol(name.clone()),
			Node::List(items, bracket, _) => TaskValue::List(items.iter().map(TaskValue::of).collect::<Result<_>>()?, crate::wasm_emitter::bracket_info(bracket)),
			Node::Key(left, op, right) => TaskValue::Key(Box::new(TaskValue::of(left)?), crate::operators::op_to_code(op), Box::new(TaskValue::of(right)?)),
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
		let memory = lookup("memory").and_then(|export| export.into_memory()).ok_or_else(|| anyhow!("no memory export"))?;
		Ok(Builders { functions, memory, heap: lookup(TEXT_HEAP_EXPORT).and_then(|export| export.into_global()) })
	}

	fn call(&self, name: &str, arguments: &[Val], store: &mut StoreContextMut<'_, HostState>) -> Result<Val> {
		self.call_into(name, arguments, Val::AnyRef(None), store)
	}

	fn call_into(&self, name: &str, arguments: &[Val], result: Val, store: &mut StoreContextMut<'_, HostState>) -> Result<Val> {
		let function = self.functions.get(name).ok_or_else(|| anyhow!("the module exports no {name}"))?;
		let mut results = [result];
		function.call(&mut *store, arguments, &mut results)?;
		Ok(results[0])
	}

	fn int_operation(&self, name: &str, a: i64, b: i64, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
		Ok(self.call_into(name, &[Val::I64(a), Val::I64(b)], Val::I64(0), store)?.unwrap_i64())
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
	fn exact_handle(&self, numerator: &num_bigint::BigInt, denominator: &num_bigint::BigInt, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
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
		TaskValue::of(&crate::wasm_reader::node_in(&node, store, self.memory))
	}

	/// A value as an Int of this instance: a fixnum, or the handle of an exact number
	fn int_of(&self, value: &TaskValue, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
		match value {
			TaskValue::Int(n) => Ok(*n),
			TaskValue::Exact(numerator, denominator) => self.exact_handle(numerator, denominator, store),
			other => Err(anyhow!("the task gave {other:?}, not an Int")),
		}
	}

	pub(crate) fn build(&self, value: &TaskValue, store: &mut StoreContextMut<'_, HostState>) -> Result<Val> {
		match value {
			TaskValue::Empty => self.call("new_empty", &[], store),
			TaskValue::Int(n) => self.call("new_int", &[Val::I64(*n)], store),
			TaskValue::Float(x) => self.call("new_float", &[Val::F64(x.to_bits())], store),
			TaskValue::Char(c) => self.call("new_codepoint", &[Val::I32(*c as i32)], store),
			TaskValue::Text(text) | TaskValue::Symbol(text) => {
				let (pointer, length) = crate::host::write_bytes(&self.memory, self.heap, store, text.as_bytes())?;
				let constructor = if matches!(value, TaskValue::Text(_)) { "new_text" } else { "new_symbol" };
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
		}
	}

	/// A value of the instance, a function value with its captured values, a list item by item (cells walked)
	fn read_value(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		let Some(node) = value.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok()) else { return self.read(value, store) };
		let kind = node.field(&mut *store, KIND_FIELD)?.unwrap_i64();
		match (kind & crate::type_kinds::KIND_MASK) as u8 {
			tag if tag == crate::type_kinds::Kind::Function as u8 => {
				let name = crate::wasm_reader::node_in(&node.field(&mut *store, VALUE_FIELD)?, store, self.memory).name();
				let closure = node.field(&mut *store, DATA_FIELD)?;
				let captured = match closure.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok()) {
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
					cell = rest.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok());
				}
				Ok(TaskValue::List(items, (kind >> 8) & 0xFF))
			}
			_ => self.read(value, store),
		}
	}

	fn read(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		match value {
			Val::I64(n) => self.integer(*n, store),
			Val::F64(bits) => Ok(TaskValue::Float(f64::from_bits(*bits))),
			Val::AnyRef(_) => TaskValue::of(&crate::wasm_reader::node_in(value, store, self.memory)),
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
			Control::Stop => Err(wasmtime::Error::new(TaskFailure(STOPPED.to_string()))),
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

/// The value of a capture global (`capture·f·x`), carried from the spawning instance to the task's
#[derive(Clone, Debug)]
enum Captured {
	Int(i64),
	Float(u64),
	Value(TaskValue),
}

enum Slot {
	Running(JoinHandle<Result<TaskValue, String>>),
	Done(Result<TaskValue, String>),
}

/// The tasks of one program run, shared by every instance it starts
pub struct TaskTable {
	engine: Engine,
	module: Module,
	imports: Imports,
	slots: Mutex<HashMap<i64, Slot>>,
	/// the tasks whose result or failure the program read (await, join): a failure nobody read ends the run (join_all)
	taken: Mutex<std::collections::HashSet<i64>>,
	controls: Mutex<HashMap<i64, Arc<TaskControl>>>,
	next_id: AtomicI64,
	/// the shared arrays of the run, linked into every task (shared.rs)
	shared: Option<Arc<crate::shared::SharedArrays>>,
}

/// Link task_spawn and task_await for `module`, sharing one table with every task it starts
pub fn link(linker: &mut Linker<HostState>, engine: &Engine, module: &Module, imports: Imports, shared: Option<Arc<crate::shared::SharedArrays>>) -> Result<Arc<TaskTable>> {
	let table = Arc::new(TaskTable {
		engine: engine.clone(), module: module.clone(), imports, slots: Mutex::default(), taken: Mutex::default(), controls: Mutex::default(), next_id: AtomicI64::new(1), shared,
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
			let name = c_string(&mut caller, name).map_err(host_error)?;
			let arguments: [i64; MAX_TASK_ARGUMENTS] = [a0, a1, a2, a3];
			let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
			let arguments = arguments.iter().map(|argument| builders.integer(*argument, &mut caller.as_context_mut())).collect::<Result<Vec<_>>>().map_err(host_error)?;
			let captured = spawner.captured(&mut caller).map_err(host_error)?;
			Ok(spawner.spawn(name, arguments, captured))
		})?;
		let value_spawner = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_SPAWN_VALUES, move |mut caller: Caller<'_, HostState>, name: i32, arguments: Option<Rooted<AnyRef>>| -> wasmtime::Result<i64> {
			let name = c_string(&mut caller, name).map_err(host_error)?;
			let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
			let task_failure = |failure: anyhow::Error| wasmtime::Error::new(TaskFailure(format!("task {}: {failure}", task_name(&name))));
			let arguments = match builders.read_value(&Val::AnyRef(arguments), &mut caller.as_context_mut()).map_err(task_failure)? {
				TaskValue::List(items, _) => items,
				TaskValue::Empty => vec![],
				single => vec![single],
			};
			let captured = value_spawner.captured(&mut caller).map_err(host_error)?;
			Ok(value_spawner.spawn(name, arguments, captured))
		})?;
		let value_awaiter = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_AWAIT_VALUE, move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<Option<Rooted<AnyRef>>> {
			let value = value_awaiter.take(id).map_err(|message| wasmtime::Error::new(TaskFailure(message)))?;
			let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
			let built = builders.build(&value, &mut caller.as_context_mut()).map_err(host_error)?;
			Ok(built.unwrap_anyref().copied())
		})?;
		let joiner = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_JOIN, move |id: i64| -> i64 { joiner.take(id).is_err() as i64 })?;
		let reporter = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_FAILURE, move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<Option<Rooted<AnyRef>>> {
			let message = reporter.take(id).err().unwrap_or_default();
			let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
			let built = builders.build(&TaskValue::Text(message), &mut caller.as_context_mut()).map_err(host_error)?;
			Ok(built.unwrap_anyref().copied())
		})?;
		linker.func_wrap(HOST_LIBRARY, crate::host::TASK_POLL, || {})?; // natively the epoch checks pause a task
		let observer = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_STATUS, move |id: i64| -> i64 { observer.status(id) })?;
		let controller = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_CONTROL, move |id: i64, operation: i64| -> i64 { controller.control(id, operation) })?;
		let awaiter = self.clone();
		linker.func_wrap(HOST_LIBRARY, TASK_AWAIT, move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<i64> {
			let value = awaiter.take(id).map_err(|message| wasmtime::Error::new(TaskFailure(message)))?;
			let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
			builders.int_of(&value, &mut caller.as_context_mut()).map_err(host_error)
		})?;
		Ok(())
	}

	/// The capture globals of the spawning instance, as their values now
	fn captured(&self, caller: &mut Caller<'_, HostState>) -> Result<Vec<(String, Captured)>> {
		let names: Vec<String> = self.module.exports().map(|export| export.name().to_string()).filter(|name| name.starts_with(crate::wasm_emitter::CAPTURE_EXPORT_PREFIX)).collect();
		if names.is_empty() {
			return Ok(vec![]);
		}
		let builders = Builders::of(&mut |export| caller.get_export(export))?;
		let mut captured = vec![];
		for name in names {
			let Some(global) = caller.get_export(&name).and_then(|export| export.into_global()) else { continue };
			let value = match global.get(&mut *caller) {
				Val::I64(n) => Captured::Int(n),
				Val::F64(bits) => Captured::Float(bits),
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
		let table = self.clone();
		let handle = std::thread::spawn(move || table.run(&function, &arguments, &captured, control).map_err(|failure| format!("task {}: {}", task_name(&function), failure_message(failure))));
		self.slots.lock().expect("task table").insert(id, Slot::Running(handle));
		id
	}

	/// f(arguments) in a fresh instance of the module, with the same imports as the program
	fn run(self: &Arc<Self>, function: &str, arguments: &[TaskValue], captured: &[(String, Captured)], control: Arc<TaskControl>) -> Result<TaskValue> {
		let mut store = crate::util::fueled_store(&self.engine, HostState::new());
		store.epoch_deadline_callback(move |_| control.checkpoint());
		store.set_epoch_deadline(1);
		let mut linker = Linker::new(&self.engine);
		crate::wasm_reader::link_imports(&mut linker, &self.engine, &self.module, self.imports)?;
		if let Some(shared) = &self.shared {
			shared.link_into(&mut linker)?;
		}
		self.link_into(&mut linker)?;
		let instance = linker.instantiate(&mut store, &self.module)?;
		let callee = instance.get_func(&mut store, function).ok_or_else(|| anyhow!("no exported function {function}"))?;
		let builders = Builders::of(&mut |export| instance.get_export(&mut store, export)).map_err(|failure| anyhow!("{failure}"))?;
		// what the spawning instance's closures captured, as it had it
		for (name, value) in captured {
			let Some(global) = instance.get_global(&mut store, name) else { continue };
			let value = match value {
				Captured::Int(n) => Val::I64(*n),
				Captured::Float(bits) => Val::F64(*bits),
				Captured::Value(value) => builders.build(value, &mut store.as_context_mut())?,
			};
			global.set(&mut store, value)?;
		}
		let signature = callee.ty(&store);
		let values: Vec<Val> = match signature.params().next() {
			// a wrapper of values (declarations::with_node_wrappers): the one argument list
			Some(ValType::Ref(_)) if signature.params().len() == 1 && function.ends_with(crate::declarations::NODE_WRAPPER_SUFFIX) => {
				vec![builders.build(&TaskValue::List(arguments.to_vec(), crate::wasm_emitter::bracket_info(&Bracket::Square)), &mut store.as_context_mut())?]
			}
			_ => signature.params().zip(arguments).map(|(parameter, argument)| match (parameter, argument) {
				(ValType::I64, TaskValue::Int(_) | TaskValue::Exact(..)) => Ok(Val::I64(builders.int_of(argument, &mut store.as_context_mut())?)),
				(ValType::Ref(_), argument) => builders.build(argument, &mut store.as_context_mut()),
				(parameter, argument) => Err(anyhow!("{function} takes {parameter}, got {argument:?}")),
			}).collect::<Result<_>>()?,
		};
		let mut results: Vec<Val> = signature.results().map(|result| match result {
			ValType::I64 => Val::I64(0),
			ValType::F64 => Val::F64(0),
			_ => Val::AnyRef(None),
		}).collect();
		// a raised error leaves its message in trap_detail, as in the program
		crate::wasm_reader::with_trap_detail(callee.call(&mut store, &values, &mut results), &mut store, &instance)?;
		match results.first() {
			Some(result) => builders.read(result, &mut store.as_context_mut()),
			None => Ok(TaskValue::Empty),
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

	/// Where the task is, without waiting for it: running, finished, failed, stopped or paused (host TASK_* codes)
	fn status(&self, id: i64) -> i64 {
		let finished = matches!(self.slots.lock().expect("task table").get(&id), Some(Slot::Running(handle)) if handle.is_finished());
		if finished {
			let _ = self.await_task(id); // joins it: the result is there
		}
		let paused = self.controls.lock().expect("task controls").get(&id).is_some_and(|control| *control.state.lock().expect("task control") == Control::Pause);
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
		let slot = self.slots.lock().expect("task table").remove(&id).ok_or_else(|| format!("no task {id}"))?;
		let result = match slot {
			Slot::Running(handle) => handle.join().unwrap_or_else(|_| Err("the task panicked".to_string())),
			Slot::Done(result) => result,
		};
		self.slots.lock().expect("task table").insert(id, Slot::Done(result.clone()));
		result
	}

	/// The result of the task, read by the program
	fn take(&self, id: i64) -> Result<TaskValue, String> {
		self.taken.lock().expect("taken tasks").insert(id);
		self.await_task(id)
	}

	/// The program ended: tasks nobody awaited still finish (their output is part of the run); the first failure nobody
	/// read is the run's error, as loud as one in the program (a stopped task was stopped on purpose)
	pub fn join_all(&self) -> Result<(), TaskFailure> {
		let mut ids: Vec<i64> = self.slots.lock().expect("task table").keys().copied().collect();
		ids.sort();
		let mut unread = None;
		for id in ids {
			if let Err(message) = self.await_task(id) {
				if unread.is_none() && !message.ends_with(STOPPED) && !self.taken.lock().expect("taken tasks").contains(&id) {
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

fn failure_message(failure: anyhow::Error) -> String {
	match crate::wasm_emitter::failed_run(failure) {
		crate::node::Node::Error(message) => message.serialize().trim_matches('"').to_string(),
		other => other.serialize(),
	}
}

/// The zero-terminated text at `pointer` in the caller's memory: the function name
pub(crate) fn c_string(caller: &mut Caller<'_, HostState>, pointer: i32) -> Result<String> {
	let memory = caller.get_export("memory").and_then(|export| export.into_memory()).ok_or_else(|| anyhow!("no memory export"))?;
	let bytes = memory.data(&*caller);
	let start = pointer as usize;
	let end = bytes[start..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |length| start + length);
	Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}
