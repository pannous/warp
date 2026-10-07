use crate::node::Node;
use crate::type_kinds::{Kind, KIND_MASK};
use crate::util::gc_engine;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::rc::Rc;
use crate::extensions::numbers::Number;
use num_bigint::{BigInt, Sign};
use wasmtime::{AsContextMut, Instance, Linker, Module, Store, StoreContextMut, Val};
use wasmtime_wasi::p1;

/// GcObject wraps a WASM GC struct reference with ergonomic field access
pub struct GcObject {
	inner: Val,
	store: Rc<RefCell<Store<()>>>,
	instance: Instance,
}

impl std::fmt::Debug for GcObject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "GcObject {{ ... }}")
	}
}

/// Compact 3-field Node layout:
/// - Field 0: kind (i64) - type tag, possibly with flags in upper bits
/// - Field 1: data (ref null any) - payload (i31ref, boxed number, node ref, etc.)
/// - Field 2: value (ref null $Node) - child/value node
pub const FIELD_KIND: usize = 0;
pub const FIELD_DATA: usize = 1;
pub const FIELD_VALUE: usize = 2;
/// Exported functions a module may start with, in the order they are looked for
/// How many result values of `main` are read: a Node, or a plain number
const MAX_MAIN_RESULTS: usize = 1;
const ENTRY_POINTS: [&str; 4] = ["main", "wasp_main", "warp_main", "_start"];

/// Int payload: `$i64box(value)`, `$BigInt(negative: i32, limbs: array i32)` little-endian base 2^32,
/// or an exact `$Ratio(numerator, denominator)` of two such payloads (wasm_emitter/exact.rs)
pub fn read_int_payload(mut store: impl AsContextMut, data_val: &Val) -> Result<Number> {
	read_payload(&mut store.as_context_mut(), data_val)
}

fn read_payload<T>(store: &mut wasmtime::StoreContextMut<'_, T>, data_val: &Val) -> Result<Number> {
	let mut store = store;
	let anyref = data_val.unwrap_anyref().ok_or_else(|| anyhow!("Int node without payload"))?;
	let payload = anyref.unwrap_struct(&store)?;
	match payload.field(&mut store, 0)? {
		numerator @ Val::AnyRef(_) => {
			let numerator = read_payload(store, &numerator)?;
			let denominator = payload.field(&mut store, 1)?;
			let denominator = read_payload(store, &denominator)?;
			Ok(Number::ratio(numerator, denominator))
		}
		Val::I64(value) => Ok(Number::Int(value)),
		Val::I32(negative) => {
			let limbs_ref = payload.field(&mut store, 1)?;
			let limbs_array = limbs_ref.unwrap_anyref().ok_or_else(|| anyhow!("BigInt without limbs"))?.unwrap_array(&store)?;
			let limbs: Vec<u32> = limbs_array.elems(&mut store)?.map(|limb| limb.unwrap_i32() as u32).collect();
			let sign = if negative != 0 { Sign::Minus } else { Sign::Plus };
			Ok(Number::from_bigint(BigInt::from_slice(sign, &limbs)))
		}
		other => Err(anyhow!("unexpected Int payload {:?}", other)),
	}
}

impl GcObject {
	pub fn new(val: Val, store: Rc<RefCell<Store<()>>>, instance: Instance) -> Self {
		GcObject {
			inner: val,
			store,
			instance,
		}
	}

	/// Get field by index
	pub fn get_field(&self, idx: usize) -> Result<Val> {
		let mut store = self.store.borrow_mut();
		if let Some(anyref) = self.inner.unwrap_anyref() {
			if let Ok(structref) = anyref.unwrap_struct(&*store) {
				return Ok(structref.field(&mut *store, idx)?);
			}
		}
		Err(anyhow!("Cannot read field at index {}", idx))
	}

	/// Get the kind field (i64)
	pub fn kind(&self) -> Result<i64> {
		let val = self.get_field(FIELD_KIND)?;
		Ok(val.unwrap_i64())
	}

	/// Get the base tag (lower 8 bits of kind)
	pub fn tag(&self) -> Result<u8> {
		Ok((self.kind()? & KIND_MASK) as u8)
	}

	/// Get the data field as a Val
	pub fn data(&self) -> Result<Val> {
		self.get_field(FIELD_DATA)
	}

	/// Get the value field as a GcObject (child node)
	pub fn value(&self) -> Result<GcObject> {
		let val = self.get_field(FIELD_VALUE)?;
		Ok(GcObject::new(
			val,
			self.store.clone(),
			self.instance,
		))
	}

	/// Check if value field is null
	pub fn value_is_null(&self) -> bool {
		match self.get_field(FIELD_VALUE) {
			Ok(val) => val.unwrap_anyref().is_none(),
			Err(_) => true,
		}
	}

	/// Read the integer payload of an Int node: `$i64box` or `$BigInt` (see wasm_emitter/big_int.rs)
	pub fn read_int(&self) -> Result<Number> {
		let data_val = self.data()?;
		let mut store = self.store.borrow_mut();
		read_int_payload(&mut *store, &data_val)
	}

	/// Read f64 from boxed f64 in data field (for Float nodes)
	pub fn read_boxed_f64(&self) -> Result<f64> {
		let data_val = self.data()?;
		let mut store = self.store.borrow_mut();
		if let Some(anyref) = data_val.unwrap_anyref() {
			if let Ok(structref) = anyref.unwrap_struct(&*store) {
				let field_val = structref.field(&mut *store, 0)?;
				return Ok(warp_runtime::floats::canonical_nan(field_val.unwrap_f64()));
			}
		}
		Err(anyhow!("Cannot read boxed f64"))
	}

	/// Read i31ref value from data field (for Codepoint)
	pub fn read_i31(&self) -> Result<i32> {
		let data_val = self.data()?;
		let store = self.store.borrow();
		if let Some(anyref) = data_val.unwrap_anyref() {
			if let Ok(i31) = anyref.unwrap_i31(&*store) {
				return Ok(i31.get_i32());
			}
		}
		Err(anyhow!("Cannot read i31ref"))
	}

	/// Read string ptr+len from $String struct in data field
	pub fn read_string_ptr_len(&self) -> Result<(i32, i32)> {
		let data_val = self.data()?;
		let mut store = self.store.borrow_mut();
		if let Some(anyref) = data_val.unwrap_anyref() {
			if let Ok(structref) = anyref.unwrap_struct(&*store) {
				let ptr_val = structref.field(&mut *store, 0)?; // field 0: ptr
				let len_val = structref.field(&mut *store, 1)?; // field 1: len
				return Ok((ptr_val.unwrap_i32(), len_val.unwrap_i32()));
			}
		}
		Err(anyhow!("Cannot read $String struct"))
	}

	/// Read string from linear memory
	pub fn read_string(&self, ptr: i32, len: i32) -> Result<String> {
		if len == 0 {
			return Ok(String::new());
		}
		let mut store = self.store.borrow_mut();
		let memory = self
			.instance
			.get_memory(&mut *store, "memory")
			.ok_or_else(|| anyhow!("No memory export"))?;
		let mut buf = vec![0u8; len as usize];
		memory.read(&*store, ptr as usize, &mut buf)?;
		String::from_utf8(buf).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
	}

	/// Get text content for Text/Symbol nodes
	pub fn text(&self) -> Result<String> {
		let (ptr, len) = self.read_string_ptr_len()?;
		self.read_string(ptr, len)
	}

	/// The Node this struct encodes
	pub fn to_node(&self) -> Node {
		let mut store = self.store.borrow_mut();
		node_of(&self.inner, &mut store.as_context_mut(), &self.instance)
	}

	/// Get the data field as a child GcObject (for Key nodes where data is a node ref)
	pub fn data_as_node(&self) -> Result<GcObject> {
		let val = self.data()?;
		Ok(GcObject::new(
			val,
			self.store.clone(),
			self.instance,
		))
	}
}

/// Trait for converting Val to Rust types
pub trait FromVal: Sized {
	fn from_val(
		val: Val,
		store: &mut Store<()>,
		instance: &Instance,
		store_rc: &Rc<RefCell<Store<()>>>,
	) -> Result<Self>;
}

impl FromVal for i32 {
	fn from_val(
		val: Val,
		_store: &mut Store<()>,
		_instance: &Instance,
		_store_rc: &Rc<RefCell<Store<()>>>,
	) -> Result<Self> {
		Ok(val.unwrap_i32())
	}
}

impl FromVal for i64 {
	fn from_val(
		val: Val,
		_store: &mut Store<()>,
		_instance: &Instance,
		_store_rc: &Rc<RefCell<Store<()>>>,
	) -> Result<Self> {
		Ok(val.unwrap_i64())
	}
}

impl FromVal for f64 {
	fn from_val(
		val: Val,
		_store: &mut Store<()>,
		_instance: &Instance,
		_store_rc: &Rc<RefCell<Store<()>>>,
	) -> Result<Self> {
		Ok(warp_runtime::floats::canonical_nan(val.unwrap_f64()))
	}
}

impl FromVal for GcObject {
	fn from_val(
		val: Val,
		_store: &mut Store<()>,
		instance: &Instance,
		store_rc: &Rc<RefCell<Store<()>>>,
	) -> Result<Self> {
		Ok(GcObject::new(val, store_rc.clone(), *instance))
	}
}

/// Load a WASM module with GC support and return root GcObject
pub fn run_wasm_gc_object(path: &str) -> Result<GcObject> {
	read_bytes_gc(&std::fs::read(path)?)
}

/// Load WASM bytes and return Node (calls from_gc_object)
pub fn read_bytes(bytes: &[u8]) -> Result<Node> {
	let (result, mut store, instance) = run_main(bytes, (), |_, _, _| Ok(()))?;
	match result {
		Val::AnyRef(Some(reference)) => {
			if !reference.is_struct(&store)? {
				return Err(anyhow!("main returned a reference that is no Node struct"));
			}
			Ok(Node::from_gc_object(&GcObject::new(result, Rc::new(RefCell::new(store)), instance)))
		}
		// a plain number or the null reference: no Node struct to read
		Val::I32(_) | Val::I64(_) | Val::F64(_) | Val::AnyRef(None) => val_to_node(&result, &mut store, &instance),
		other => Err(anyhow!("main returns {:?}, which is no Node and no number", other.ty(&store)?)),
	}
}

/// Load WASM bytes and return GcObject
pub fn read_bytes_gc(bytes: &[u8]) -> Result<GcObject> {
	let (result, store, instance) = run_main(bytes, (), |_, _, _| Ok(()))?;
	Ok(GcObject::new(result, Rc::new(RefCell::new(store)), instance))
}

/// The engine that runs `bytes` (a module, or one compiled ahead of time), and whether it is the task engine: a
/// program that starts tasks needs its epoch interruption
pub fn engine_for(bytes: &[u8]) -> (wasmtime::Engine, bool) {
	let starts_tasks = crate::tasks::imports_tasks_in(bytes) || crate::run::module_cache::precompiled_for_tasks(bytes);
	(if starts_tasks { crate::util::task_engine() } else { gc_engine() }, starts_tasks)
}

/// Instantiates `bytes` with the imports `link` adds and calls its `main` (or `_start`): the result, the store and the instance
fn run_main<S: 'static>(
	bytes: &[u8],
	state: S,
	link: impl FnOnce(&mut Linker<S>, &wasmtime::Engine, &Module) -> Result<()>,
) -> Result<(Val, Store<S>, Instance)> {
	let (engine, starts_tasks) = engine_for(bytes);
	let mut store = crate::util::fueled_store(&engine, state);
	if starts_tasks {
		store.set_epoch_deadline(crate::tasks::MAIN_EPOCH_DEADLINE);
	}
	let module = crate::run::module_cache::compiled_module(&engine, bytes)?;
	let mut linker = Linker::new(&engine);
	link(&mut linker, &engine, &module)?;
	define_imported_entities(&mut linker, &mut store, &module)?;
	let instance = linker.instantiate(&mut store, &module)?;
	let main = ENTRY_POINTS
		.iter()
		.find_map(|name| instance.get_func(&mut store, name))
		.ok_or_else(|| anyhow!("No entry point: {:?}", ENTRY_POINTS))?;
	let result_types: Vec<_> = main.ty(&store).results().collect();
	if result_types.len() > MAX_MAIN_RESULTS {
		return Err(anyhow!("main returns {} values, at most {} can be read", result_types.len(), MAX_MAIN_RESULTS));
	}
	// a main without a result leaves nothing to read: the null reference reads as Empty
	let mut results = vec![Val::AnyRef(None); result_types.len()];
	use warp_runtime::system_signals::{stay_while_listening, with_exit_handler};
	// `warp run` keeps a program with a live timer after main (notes/system_signals.md), then `on exit {…}` runs
	let outcome = main.call(&mut store, &[], &mut results).and_then(|_| stay_while_listening(&mut store, &instance));
	let outcome = with_exit_handler(outcome, &mut store, |store, name| instance.get_func(&mut *store, name));
	match with_trap_detail(outcome, &mut store, &instance)? {
		Some(()) => Ok((results.first().copied().unwrap_or(Val::AnyRef(None)), store, instance)),
		None => Ok((Val::AnyRef(None), store, instance)), // `exit` (P121): the run's value is ø
	}
}

/// The memories and tables a module imports (`use { memory, table } from "env"`) that nothing linked provides: fresh
/// ones, as the embedder of a run of its own
fn define_imported_entities<S>(linker: &mut Linker<S>, store: &mut Store<S>, module: &Module) -> Result<()> {
	for import in module.imports() {
		if linker.get_by_import(&mut *store, &import).is_some() {
			continue;
		}
		let entity: wasmtime::Extern = match import.ty() {
			wasmtime::ExternType::Memory(memory) => wasmtime::Memory::new(&mut *store, memory)?.into(),
			wasmtime::ExternType::Table(table) => wasmtime::Table::new(&mut *store, table, wasmtime::Ref::Func(None))?.into(),
			_ => continue,
		};
		linker.define(&*store, import.module(), import.name(), entity)?;
	}
	Ok(())
}

/// Instantiates `bytes` with the import families it needs and runs `main`, keeping the instance for later calls of its
/// exports (headless.rs: a page's handlers)
pub(crate) fn run_main_kept(bytes: &[u8], imports: Imports) -> Result<(Val, Store<crate::host::HostState>, Instance)> {
	run_main(bytes, crate::host::HostState::new(), |linker, engine, module| link_imports(linker, engine, module, imports))
}

/// Load WASM bytes with host function support and return Node
/// Use this for modules that import host.fetch or host.run
pub fn read_bytes_with_host(bytes: &[u8]) -> Result<Node> {
	read_bytes_with_imports(bytes, Imports { host: true, ..Imports::default() })
}

/// The import families a module needs linked
#[derive(Clone, Copy, Default)]
pub struct Imports {
	pub host: bool,
	pub wasi: bool,
	pub ffi: bool,
}

impl Imports {
	/// For a module that does not say which families it needs: a .wasm file, an imported module
	pub const EVERY: Imports = Imports { host: true, wasi: true, ffi: true };
}

/// Link the import families a module needs: the host words, WASI, FFI (the task words come from tasks::link). `linker`
/// is a fresh one: it becomes a copy of the linker this thread keeps for the engine and families (linking libm and
/// the host words took most of a small program's run), plus the dynamic libraries of this module
pub fn link_imports(linker: &mut Linker<crate::host::HostState>, engine: &wasmtime::Engine, module: &Module, imports: Imports) -> Result<()> {
	// the host words (sleep, random, clock) are imported like C functions from the host module
	let host = imports.host || module.imports().any(|import| import.module() == crate::host::HOST_LIBRARY);
	*linker = family_linker(engine, Imports { host, ..imports })?;
	if imports.ffi {
		// the dynamic libraries discovered from the module's imports (raylib, SDL2, etc.)
		crate::ffi::link_module_libraries(linker, engine, module)?;
	}
	crate::wasm_modules::link(linker, module)?;
	Ok(())
}

/// A linker made for an engine and its import families
type FamilyLinker = (wasmtime::Engine, [bool; 3], Linker<crate::host::HostState>);

/// The linkers made on this thread, by engine and import families; past FAMILY_LINKERS_KEPT they start over
const FAMILY_LINKERS_KEPT: usize = 16;
thread_local! {
	static FAMILY_LINKERS: RefCell<Vec<FamilyLinker>> = const { RefCell::new(Vec::new()) };
}

fn family_linker(engine: &wasmtime::Engine, imports: Imports) -> Result<Linker<crate::host::HostState>> {
	use crate::host::{HostState, link_host_functions};
	let families = [imports.host, imports.wasi, imports.ffi];
	let kept = FAMILY_LINKERS.with(|linkers| linkers.borrow().iter()
		.find(|(kept_engine, kept_families, _)| wasmtime::Engine::same(kept_engine, engine) && *kept_families == families)
		.map(|(_, _, linker)| linker.clone()));
	if let Some(linker) = kept {
		return Ok(linker);
	}
	let mut linker = Linker::new(engine);
	if imports.host {
		link_host_functions(&mut linker, engine)?;
	}
	if imports.wasi {
		p1::add_to_linker_sync(&mut linker, |state: &mut HostState| &mut state.wasi)?;
	}
	if imports.ffi {
		crate::ffi::link_ffi_functions(&mut linker, engine)?;
	}
	FAMILY_LINKERS.with(|linkers| {
		let mut linkers = linkers.borrow_mut();
		if linkers.len() >= FAMILY_LINKERS_KEPT {
			linkers.clear();
		}
		linkers.push((engine.clone(), families, linker.clone()));
	});
	Ok(linker)
}

/// Load WASM bytes with every import family it needs in one linker: a program may print, fetch and call C at once.
/// A program that starts tasks (`go f(x)`) gets the task words; the tasks nobody awaited finish before the result.
pub fn read_bytes_with_imports(bytes: &[u8], imports: Imports) -> Result<Node> {
	let mut tasks = None;
	let outcome = run_main(bytes, crate::host::HostState::new(), |linker, engine, module| link_run(linker, engine, module, imports, &mut tasks));
	let (result, mut store, instance) = finish_tasks(outcome, tasks.as_deref())?;
	val_to_node(&result, &mut store, &instance)
}

/// Link a run's imports, with the shared arrays and the task words when the module uses them (`tasks` gets the run's)
fn link_run(linker: &mut Linker<crate::host::HostState>, engine: &wasmtime::Engine, module: &Module, imports: Imports, tasks: &mut Option<std::sync::Arc<crate::tasks::TaskTable>>) -> Result<()> {
	link_imports(linker, engine, module, imports)?;
	// the shared arrays of the run: the program's and every task's instance reach the same ones
	let shared = crate::shared::imports_shared(module).then(|| std::sync::Arc::new(crate::shared::SharedArrays::default()));
	if let Some(shared) = &shared {
		shared.link_into(linker)?;
	}
	if crate::tasks::imports_tasks(module) {
		*tasks = Some(crate::tasks::link(linker, engine, module, imports, shared)?);
	}
	Ok(())
}

/// After main: the tasks nobody awaited finish (their unread failure ends the run), their raises run their handlers
/// and the listeners on shared values see what the tasks left
fn finish_tasks<R>(outcome: Result<(R, Store<crate::host::HostState>, Instance)>, tasks: Option<&crate::tasks::TaskTable>) -> Result<(R, Store<crate::host::HostState>, Instance)> {
	let unread_failure = tasks.map_or(Ok(()), |tasks| tasks.join_all());
	let (result, mut store, instance) = outcome?;
	if let Some(tasks) = tasks {
		tasks.deliver(&mut store, &mut |store, name| instance.get_export(&mut *store, name))?;
		warp_runtime::system_signals::run_due_handlers(&mut store, |store, name| instance.get_func(&mut *store, name))?;
	}
	unread_failure?;
	Ok((result, store, instance))
}

/// Run main, then call the parameterless export `name` (a page's page·html, src/site.rs): its value
pub fn read_export_after_main(bytes: &[u8], imports: Imports, name: &str) -> Result<Node> {
	let mut tasks = None;
	let outcome = run_main(bytes, crate::host::HostState::new(), |linker, engine, module| link_run(linker, engine, module, imports, &mut tasks));
	let (_, mut store, instance) = finish_tasks(outcome, tasks.as_deref())?;
	let export = instance.get_func(&mut store, name).ok_or_else(|| anyhow!("the module exports no {name}"))?;
	let mut results = vec![Val::AnyRef(None); export.ty(&store).results().len()];
	let outcome = export.call(&mut store, &[], &mut results);
	with_trap_detail(outcome, &mut store, &instance)?;
	val_to_node(&results.first().copied().unwrap_or(Val::AnyRef(None)), &mut store, &instance)
}

pub use crate::wasm_emitter::{trap_detail_line, TRAP_DETAIL, TRAP_DETAIL_PREFIX};

/// A trapped run, with the value the program left in `trap_detail` before trapping as the error's context
pub(crate) fn with_trap_detail<T, R>(outcome: wasmtime::Result<R>, store: &mut Store<T>, instance: &Instance) -> Result<R> {
	outcome.map_err(|failure| {
		let failure = anyhow::Error::from(failure);
		let detail = instance.get_global(&mut *store, TRAP_DETAIL).map(|global| global.get(&mut *store));
		match detail.filter(|value| !matches!(value, Val::AnyRef(None))).and_then(|value| val_to_node(&value, store, instance).ok()) {
			Some(value) => failure.context(trap_detail_line(&value)),
			None => failure,
		}
	})
}

/// Convert a `main` result (primitive or GC Node struct) into a Node, for any store state
pub(crate) fn val_to_node<T>(result: &Val, store: &mut Store<T>, instance: &Instance) -> Result<Node> {
	match result {
		Val::I64(n) => Ok(Node::Number(Number::Int(*n))),
		Val::I32(n) => Ok(Node::Number(Number::Int(*n as i64))),
		Val::F64(bits) => Ok(Node::Number(Number::Float(warp_runtime::floats::canonical_nan(f64::from_bits(*bits))))),
		Val::AnyRef(_) => Ok(node_of(result, &mut store.as_context_mut(), instance)),
		_ => Ok(Node::Empty),
	}
}

/// The Node a GC `$Node` struct encodes (layout in CLAUDE.md "Serialization"); null is Empty. Generic over the store
/// state, so runs with WASI or host imports read lists and keys like plain runs do.
pub fn node_of<T>(value: &Val, store: &mut StoreContextMut<'_, T>, instance: &Instance) -> Node {
	match instance.get_memory(&mut *store, "memory") {
		Some(memory) => node_in(value, store, memory),
		None => Node::Empty,
	}
}

/// The Node a GC `$Node` struct encodes, its texts read from `memory`: also from a host function, which has the memory
/// of its caller but no Instance (tasks.rs)
pub fn node_in<T>(value: &Val, store: &mut StoreContextMut<'_, T>, memory: wasmtime::Memory) -> Node {
	use crate::node::Bracket;
	let Some(structref) = value.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok()) else { return Node::Empty };
	let mut field = |index: usize| structref.field(&mut *store, index);
	let (Ok(kind), Ok(data), Ok(child)) = (field(FIELD_KIND), field(FIELD_DATA), field(FIELD_VALUE)) else { return Node::Empty };
	let kind = kind.unwrap_i64();
	let tag = (kind & KIND_MASK) as u8;
	let high_byte = (kind >> 8) & 0xFF;
	let mut node = |val: &Val| node_in(val, store, memory);
	match tag {
		t if t == Kind::Empty as u8 => Node::Empty,
		t if t == Kind::Int as u8 => match read_payload(store, &data) {
			Ok(number) => Node::Number(number),
			Err(failure) => Node::Error(Box::new(Node::Text(format!("unreadable Int: {failure}")))),
		},
		t if t == Kind::Float as u8 => Node::Number(Number::Float(boxed_f64(store, &data).unwrap_or(0.0))),
		t if t == Kind::Codepoint as u8 => {
			let code = data.unwrap_anyref().and_then(|reference| reference.unwrap_i31(&*store).ok()).map(|code| code.get_u32());
			Node::Char(code.and_then(char::from_u32).unwrap_or('\0'))
		}
		t if t == Kind::Text as u8 => Node::Text(text_of(store, &data, memory)),
		t if t == Kind::Symbol as u8 => Node::Symbol(text_of(store, &data, memory)),
		t if t == Kind::Error as u8 => Node::Error(Box::new(Node::Text(text_of(store, &data, memory)))),
		t if t == Kind::Key as u8 => Node::Key(Box::new(node(&data)), crate::operators::code_to_op(high_byte), Box::new(node(&child))),
		t if t == Kind::Block as u8 => list_in(data, child, Bracket::Curly, store, memory),
		t if t == Kind::List as u8 => {
			let bracket = match high_byte {
				0 => Bracket::Curly,
				1 => Bracket::Square,
				2 => Bracket::Round,
				3 => Bracket::Less,
				_ => Bracket::None,
			};
			list_in(data, child, bracket, store, memory)
		}
		t if t == Kind::Data as u8 => {
			let type_name = text_of(store, &data, memory);
			Node::Data(crate::meta::DataValue { data: Box::new(format!("<wasm data: {type_name}>")), type_name, data_type: crate::meta::DataType::Other })
		}
		// a closure reads as the name of its function
		t if t == Kind::Function as u8 => match child.unwrap_anyref() {
			Some(_) => node(&child),
			None => Node::Symbol("function".to_string()),
		},
		t if t == Kind::TypeDef as u8 => Node::Type { name: Box::new(node(&data)), body: Box::new(node(&child)) },
		_ => Node::Text(format!("Unknown Kind: {tag}")),
	}
}

/// The items of a list of cons cells, walked in a loop (a long list must not take a stack frame per item): each cell's
/// first item, then its rest, itself a cell of a list or block or a last single item; ø items stay
fn list_in<T>(mut first: Val, mut rest: Val, bracket: crate::node::Bracket, store: &mut StoreContextMut<'_, T>, memory: wasmtime::Memory) -> Node {
	let mut items = Vec::new();
	loop {
		// a null first item is the empty list's cell; a ø element is a node of kind Empty, kept
		if first.unwrap_anyref().is_some() {
			items.push(node_in(&first, store, memory));
		}
		let Some(cell) = rest.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok()) else { break };
		let (Ok(kind), Ok(data), Ok(child)) = (cell.field(&mut *store, FIELD_KIND), cell.field(&mut *store, FIELD_DATA), cell.field(&mut *store, FIELD_VALUE)) else { break };
		let tag = (kind.unwrap_i64() & KIND_MASK) as u8;
		if tag != Kind::List as u8 && tag != Kind::Block as u8 {
			match node_in(&rest, store, memory) {
				Node::Empty => {}
				last => items.push(last),
			}
			break;
		}
		(first, rest) = (data, child);
	}
	Node::List(items, bracket, crate::node::Separator::None)
}

fn boxed_f64<T>(store: &mut StoreContextMut<'_, T>, data: &Val) -> Option<f64> {
	let float_box = data.unwrap_anyref()?.unwrap_struct(&*store).ok()?;
	float_box.field(&mut *store, 0).ok().map(|value| warp_runtime::floats::canonical_nan(value.unwrap_f64()))
}

/// The letters of a `$String(ptr, len)` in linear memory
fn text_of<T>(store: &mut StoreContextMut<'_, T>, data: &Val, memory: wasmtime::Memory) -> String {
	let read = |store: &mut StoreContextMut<'_, T>| -> Option<String> {
		let string = data.unwrap_anyref()?.unwrap_struct(&*store).ok()?;
		let ptr = string.field(&mut *store, 0).ok()?.unwrap_i32() as usize;
		let len = string.field(&mut *store, 1).ok()?.unwrap_i32() as usize;
		let bytes = memory.data(&*store).get(ptr..ptr + len)?;
		String::from_utf8(bytes.to_vec()).ok()
	};
	read(store).unwrap_or_default()
}

/// Create a node by calling a constructor function
pub fn call_constructor(
	func_name: &str,
	args: &[Val],
	store: Rc<RefCell<Store<()>>>,
	instance: &Instance,
) -> Result<GcObject> {
	let func = {
		let mut s = store.borrow_mut();
		instance
			.get_func(&mut *s, func_name)
			.ok_or_else(|| anyhow!("Function {} not found", func_name))?
	};

	let mut results = vec![Val::I32(0)];
	{
		let mut s = store.borrow_mut();
		func.call(&mut *s, args, &mut results)?;
	}

	Ok(GcObject::new(results[0], store, *instance))
}

/// Load WASM bytes with WASI support (for fd_write, puts, etc.)
pub fn read_bytes_with_wasi(bytes: &[u8]) -> Result<Node> {
	read_bytes_with_imports(bytes, Imports { wasi: true, ..Imports::default() })
}

/// Load WASM bytes with FFI support (for native function imports)
pub fn read_bytes_with_ffi(bytes: &[u8]) -> Result<Node> {
	read_bytes_with_imports(bytes, Imports { ffi: true, ..Imports::default() })
}
