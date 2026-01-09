use crate::node::Node;
use crate::type_kinds::Kind;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::rc::Rc;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store, Val};

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
				return structref.field(&mut *store, idx);
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
		Ok((self.kind()? & 0xFF) as u8)
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
			self.instance.clone(),
		))
	}

	/// Check if value field is null
	pub fn value_is_null(&self) -> bool {
		match self.get_field(FIELD_VALUE) {
			Ok(val) => val.unwrap_anyref().is_none(),
			Err(_) => true,
		}
	}

	/// Read i64 from boxed i64 in data field (for Int nodes)
	pub fn read_boxed_i64(&self) -> Result<i64> {
		let data_val = self.data()?;
		let mut store = self.store.borrow_mut();
		if let Some(anyref) = data_val.unwrap_anyref() {
			if let Ok(structref) = anyref.unwrap_struct(&*store) {
				let field_val = structref.field(&mut *store, 0)?;
				return Ok(field_val.unwrap_i64());
			}
		}
		Err(anyhow!("Cannot read boxed i64"))
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
		if ptr == 0 || len == 0 {
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
			self.instance.clone(),
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
		Ok(GcObject::new(val, store_rc.clone(), instance.clone()))
	}
}

/// Load a WASM module with GC support and return root GcObject
pub fn run_wasm_gc_object(path: &str) -> Result<GcObject> {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);

	let engine = Engine::new(&config)?;
	let store = Store::new(&engine, ());
	let store_rc = Rc::new(RefCell::new(store));

	let wasm_bytes = std::fs::read(path)?;
	let module = Module::new(&engine, wasm_bytes)?;

	let linker = Linker::new(&engine);
	let instance = {
		let mut s = store_rc.borrow_mut();
		linker.instantiate(&mut *s, &module)?
	};

	let mut results = vec![Val::I32(0)];
	{
		let mut s = store_rc.borrow_mut();
		let names = ["main", "wasp_main", "warp_main", "_start"];
		let main = names
			.iter()
			.find_map(|&n| instance.get_func(&mut *s, n))
			.ok_or_else(|| anyhow!("No entry point: {:?}", names))?;

		main.call(&mut *s, &[], &mut results)?;
	}

	Ok(GcObject::new(results[0].clone(), store_rc, instance))
}

/// Load WASM bytes and return Node (calls from_gc_object)
pub fn read_bytes(bytes: &[u8]) -> Result<Node> {
	let obj = read_bytes_gc(bytes)?;
	Ok(Node::from_gc_object(&obj))
}

/// Load WASM bytes and return GcObject
pub fn read_bytes_gc(bytes: &[u8]) -> Result<GcObject> {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);

	let engine = Engine::new(&config)?;
	let store = Store::new(&engine, ());
	let store_rc = Rc::new(RefCell::new(store));

	let module = Module::new(&engine, bytes)?;

	let linker = Linker::new(&engine);
	let instance = {
		let mut s = store_rc.borrow_mut();
		linker.instantiate(&mut *s, &module)?
	};

	let main = {
		let mut s = store_rc.borrow_mut();
		instance
			.get_func(&mut *s, "main")
			.ok_or_else(|| anyhow!("No main function"))?
	};

	let mut results = vec![Val::I32(0)];
	{
		let mut s = store_rc.borrow_mut();
		main.call(&mut *s, &[], &mut results)?;
	}

	Ok(GcObject::new(results[0].clone(), store_rc, instance))
}

/// Load WASM bytes with host function support and return Node
/// Use this for modules that import host.fetch or host.run
pub fn read_bytes_with_host(bytes: &[u8]) -> Result<Node> {
	use crate::host::{HostState, link_host_functions};

	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);

	let engine = Engine::new(&config)?;
	let store: Store<HostState> = Store::new(&engine, HostState::new());
	let store_rc = Rc::new(RefCell::new(store));

	let module = Module::new(&engine, bytes)?;

	// Create linker with host functions
	let mut linker = Linker::new(&engine);
	link_host_functions(&mut linker, &engine)?;

	let instance = {
		let mut s = store_rc.borrow_mut();
		linker.instantiate(&mut *s, &module)?
	};

	let main = {
		let mut s = store_rc.borrow_mut();
		instance
			.get_func(&mut *s, "main")
			.ok_or_else(|| anyhow!("No main function"))?
	};

	let mut results = vec![Val::I32(0)];
	{
		let mut s = store_rc.borrow_mut();
		main.call(&mut *s, &[], &mut results)?;
	}

	// Convert result to Node
	// For host functions, the result is typically i64 (numeric value)
	let result = &results[0];
	match result {
		Val::I64(n) => Ok(Node::Number(crate::extensions::numbers::Number::Int(*n))),
		Val::I32(n) => Ok(Node::Number(crate::extensions::numbers::Number::Int(*n as i64))),
		Val::F64(bits) => Ok(Node::Number(crate::extensions::numbers::Number::Float(f64::from_bits(*bits)))),
		_ => {
			// For GC objects, we can't easily convert without the full Node infrastructure
			// Return empty for now - this path is for simple host function testing
			Ok(Node::Empty)
		}
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

	Ok(GcObject::new(results[0].clone(), store, instance.clone()))
}
