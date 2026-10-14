//! Values crossing between instances: a TaskValue is read out of one instance and rebuilt in another through its exported constructors
use super::*;

pub(crate) const CAPTURED_FIELD: usize = 1;

/// The exported constructors a value is rebuilt with in an instance
pub(crate) const CONSTRUCTORS: [&str; 11] = [
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
pub(crate) const LIMB_BITS: i64 = 32;
/// The node fields (CLAUDE.md "Serialization") and the closure's captured values (wasm_emitter/closures.rs)
pub(crate) const KIND_FIELD: usize = 0;
pub(crate) const DATA_FIELD: usize = 1;
pub(crate) const VALUE_FIELD: usize = 2;

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
		let memory =
			lookup("memory").and_then(|export| export.into_memory()).ok_or_else(|| anyhow!("no memory export"))?;
		Ok(Builders { functions, memory, heap: lookup(TEXT_HEAP_EXPORT).and_then(|export| export.into_global()) })
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
	pub(super) fn integer(&self, n: i64, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		if crate::wasm_emitter::is_fixnum(n) {
			return Ok(TaskValue::Int(n));
		}
		let node = self.call("new_int", &[Val::I64(n)], store)?;
		TaskValue::of(&crate::run::wasm_reader::node_in(&node, store, self.memory))
	}

	/// A value as an Int of this instance: a fixnum, or the handle of an exact number
	pub(super) fn int_of(&self, value: &TaskValue, store: &mut StoreContextMut<'_, HostState>) -> Result<i64> {
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
	pub(super) fn read_value(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		let Some(node) = value.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok()) else {
			return self.read(value, store);
		};
		let kind = node.field(&mut *store, KIND_FIELD)?.unwrap_i64();
		match (kind & crate::type_kinds::KIND_MASK) as u8 {
			tag if tag == crate::type_kinds::Kind::Function as u8 => {
				let name =
					crate::run::wasm_reader::node_in(&node.field(&mut *store, VALUE_FIELD)?, store, self.memory).name();
				let closure = node.field(&mut *store, DATA_FIELD)?;
				let captured = match closure.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok())
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
					cell = rest.unwrap_anyref().and_then(|reference| reference.unwrap_struct(&*store).ok());
				}
				Ok(TaskValue::List(items, ((kind >> 8) & 0xFF) | ((kind & crate::type_kinds::SEPARATOR_MASK) >> 8)))
			}
			_ => self.read(value, store),
		}
	}

	pub(super) fn read(&self, value: &Val, store: &mut StoreContextMut<'_, HostState>) -> Result<TaskValue> {
		match value {
			Val::I64(n) => self.integer(*n, store),
			Val::F64(bits) => Ok(TaskValue::Float(warp_runtime::floats::canonical_nan(f64::from_bits(*bits)))),
			Val::AnyRef(_) => TaskValue::of(&crate::run::wasm_reader::node_in(value, store, self.memory)),
			other => Err(anyhow!("a task gave {other:?}")),
		}
	}
}

/// A task value as the calling module's GC value
pub(crate) fn build_in(
	caller: &mut Caller<'_, HostState>,
	value: &TaskValue,
) -> wasmtime::Result<Option<Rooted<AnyRef>>> {
	let builders = Builders::of(&mut |export| caller.get_export(export)).map_err(host_error)?;
	let built = builders.build(value, &mut caller.as_context_mut()).map_err(host_error)?;
	Ok(built.unwrap_anyref().copied())
}

/// The calling module's GC value as a task value
pub(crate) fn read_in(caller: &mut Caller<'_, HostState>, value: Option<Rooted<AnyRef>>) -> Result<TaskValue> {
	Builders::of(&mut |export| caller.get_export(export))?.read_value(&Val::AnyRef(value), &mut caller.as_context_mut())
}
