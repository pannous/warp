use crate::node::Node;
use crate::type_kinds::{Kind, KIND_MASK};
use crate::util::gc_engine;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::rc::Rc;
use crate::extensions::numbers::Number;
use num_bigint::{BigInt, Sign};
use wasmtime::{AsContextMut, Instance, Linker, Module, Store, Val};
use wasmtime_wasi::{WasiCtxBuilder, p1};

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
				return Ok(field_val.unwrap_f64());
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
		Ok(val.unwrap_f64())
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

/// Instantiates `bytes` with the imports `link` adds and calls its `main` (or `_start`): the result, the store and the instance
fn run_main<S: 'static>(
	bytes: &[u8],
	state: S,
	link: impl FnOnce(&mut Linker<S>, &wasmtime::Engine, &Module) -> Result<()>,
) -> Result<(Val, Store<S>, Instance)> {
	let engine = gc_engine();
	let mut store = crate::util::fueled_store(&engine, state);
	let module = Module::new(&engine, bytes)?;
	let mut linker = Linker::new(&engine);
	link(&mut linker, &engine, &module)?;
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
	let outcome = main.call(&mut store, &[], &mut results);
	with_trap_detail(outcome, &mut store, &instance)?;
	Ok((results.first().copied().unwrap_or(Val::AnyRef(None)), store, instance))
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

/// Load WASM bytes with every import family it needs in one linker: a program may print, fetch and call C at once
pub fn read_bytes_with_imports(bytes: &[u8], imports: Imports) -> Result<Node> {
	use crate::host::{HostState, link_host_functions};
	use crate::ffi::{link_ffi_functions, link_module_libraries};
	let (result, mut store, instance) = run_main(bytes, HostState::new(), |linker, engine, module| {
		// the host words (sleep, random, clock) are imported like C functions from the host module
		if imports.host || module.imports().any(|import| import.module() == crate::host::HOST_LIBRARY) {
			link_host_functions(linker, engine)?;
		}
		if imports.wasi {
			p1::add_to_linker_sync(linker, |state: &mut HostState| &mut state.wasi)?;
		}
		if imports.ffi {
			// FFI functions plus the dynamic libraries discovered from the module's imports (raylib, SDL2, etc.)
			link_ffi_functions(linker, engine)?;
			link_module_libraries(linker, engine, module)?;
		}
		Ok(())
	})?;
	val_to_node(&result, &mut store, &instance)
}

pub use crate::wasm_emitter::{TRAP_DETAIL, TRAP_DETAIL_PREFIX};

/// A trapped run, with the value the program left in `trap_detail` before trapping as the error's context
fn with_trap_detail<T, R>(outcome: wasmtime::Result<R>, store: &mut Store<T>, instance: &Instance) -> Result<R> {
	outcome.map_err(|failure| {
		let failure = anyhow::Error::from(failure);
		let detail = instance.get_global(&mut *store, TRAP_DETAIL).map(|global| global.get(&mut *store));
		match detail.filter(|value| !matches!(value, Val::AnyRef(None))).and_then(|value| val_to_node(&value, store, instance).ok()) {
			Some(value) => failure.context(format!("{TRAP_DETAIL_PREFIX}{}", value.serialize())),
			None => failure,
		}
	})
}

/// Convert a `main` result (primitive or GC Node struct) into a Node, for any store state
fn val_to_node<T>(result: &Val, mut store: &mut Store<T>, instance: &Instance) -> Result<Node> {
	match result {
		Val::I64(n) => Ok(Node::Number(crate::extensions::numbers::Number::Int(*n))),
		Val::I32(n) => Ok(Node::Number(crate::extensions::numbers::Number::Int(*n as i64))),
		Val::F64(bits) => Ok(Node::Number(crate::extensions::numbers::Number::Float(f64::from_bits(*bits)))),
		Val::AnyRef(anyref_opt) => {
			// Handle GC struct result
			if let Some(anyref) = anyref_opt {
				if let Ok(structref) = anyref.unwrap_struct(&store) {
					// Read Node struct: (kind: i64, data: anyref, value: ref null $Node)
					let kind_val = structref.field(&mut store, FIELD_KIND)?;
					let kind = kind_val.unwrap_i64();
					let tag = (kind & KIND_MASK) as u8;

					match tag {
						t if t == Kind::Empty as u8 => Ok(Node::Empty),
						t if t == Kind::Int as u8 => {
							let data_val = structref.field(&mut store, FIELD_DATA)?;
							Ok(Node::Number(read_int_payload(&mut store, &data_val)?))
						}
						t if t == Kind::Float as u8 => {
							let data_val = structref.field(&mut store, FIELD_DATA)?;
							let float_box = data_val.unwrap_anyref().and_then(|data| data.unwrap_struct(&store).ok());
							let value = match float_box {
								Some(float_box) => float_box.field(&mut store, 0)?.unwrap_f64(),
								None => 0.0,
							};
							Ok(Node::Number(crate::extensions::numbers::Number::Float(value)))
						}
						t if t == Kind::Codepoint as u8 => {
							let data_val = structref.field(&mut store, FIELD_DATA)?;
							let code = data_val.unwrap_anyref().and_then(|data| data.as_i31(&store).ok().flatten()).map(|code| code.get_u32());
							Ok(code.and_then(char::from_u32).map_or(Node::Empty, Node::Char))
						}
						// an Error from a host call (fetch) carries its reason as text
						t if t == Kind::Text as u8 || t == Kind::Symbol as u8 || t == Kind::Error as u8 => {
							let textual = |s: String| match tag {
								t if t == Kind::Text as u8 => Node::Text(s),
								t if t == Kind::Error as u8 => Node::Error(Box::new(Node::Text(s))),
								_ => Node::Symbol(s),
							};
							// data field contains $String struct (ptr, len)
							let data_val = structref.field(&mut store, FIELD_DATA)?;
							if let Some(data_anyref) = data_val.unwrap_anyref() {
								if let Ok(str_struct) = data_anyref.unwrap_struct(&store) {
									let ptr_val = str_struct.field(&mut store, 0)?;
									let len_val = str_struct.field(&mut store, 1)?;
									let ptr = ptr_val.unwrap_i32() as usize;
									let len = len_val.unwrap_i32() as usize;

									// Read string from memory
									if let Some(memory) = instance.get_memory(&mut store, "memory") {
										let data = memory.data(&store);
										if ptr + len <= data.len() {
											let string_bytes = &data[ptr..ptr + len];
											if let Ok(s) = std::str::from_utf8(string_bytes) {
												return Ok(textual(s.to_string()));
											}
										}
									}
								}
							}
							Ok(textual(String::new()))
						}
						_ => Ok(Node::Empty),
					}
				} else {
					Ok(Node::Empty)
				}
			} else {
				Ok(Node::Empty)
			}
		}
		_ => Ok(Node::Empty),
	}
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
