//! Warp's GC Nodes (CLAUDE.md "Serialization", warp's type_kinds.rs) as JSON and back, for an executable's foreign calls
//! (card exe-foreign): what warp does with json_of(node_in(…)) and Builders::build(TaskValue::of(node_of(…))), without
//! warp's Node. warp's tests/wasm/test_executable_foreign.rs holds these codes equal to warp's.
use crate::foreign::{BIG_INTEGER_KEY, CODEPOINT_KEY};
use serde_json::{Map, Value};
use wasmtime::{AnyRef, AsContextMut, Func, Global, Memory, Result, Rooted, StoreContextMut, StructRef, Val};

pub const KIND_MASK: i64 = 0xFF;
pub const KIND_BITS: i64 = 8;
pub const EMPTY: u8 = 0;
pub const INT: u8 = 1;
pub const FLOAT: u8 = 2;
pub const TEXT: u8 = 3;
pub const CODEPOINT: u8 = 4;
pub const SYMBOL: u8 = 5;
pub const KEY: u8 = 6;
pub const BLOCK: u8 = 7;
pub const LIST: u8 = 8;
pub const ERROR: u8 = 11;
pub const FUNCTION: u8 = 16;
/// The high bits of a bool's Int kind
pub const BOOL_INFO: i64 = 1;
/// Op::Colon's code, `key: value`
pub const COLON: i64 = 1;
pub const CURLY: i64 = 0;
pub const SQUARE: i64 = 1;
const FIELD_KIND: usize = 0;
const FIELD_DATA: usize = 1;
const FIELD_VALUE: usize = 2;
/// The exported constructors a value is built with in the program
const NEW_EMPTY: &str = "new_empty";
const NEW_INT: &str = "new_int";
const NEW_FLOAT: &str = "new_float";
const NEW_CODEPOINT: &str = "new_codepoint";
const NEW_TEXT: &str = "new_text";
const NEW_SYMBOL: &str = "new_symbol";
const NEW_KEY: &str = "new_key";
const NEW_LIST: &str = "new_list";
/// The exports an integer beyond the fixnums is composed with: shift left, add, subtract (warp's EXACT_BUILDERS)
const SHIFT_LEFT: &str = "int_shift_left";
const ADD: &str = "exact_add";
const SUBTRACT: &str = "exact_sub";
pub const BUILDERS: [&str; 11] =
	[NEW_EMPTY, NEW_INT, NEW_FLOAT, NEW_CODEPOINT, NEW_TEXT, NEW_SYMBOL, NEW_KEY, NEW_LIST, SHIFT_LEFT, ADD, SUBTRACT];
/// The global holding the end of the program's texts (warp's TEXT_HEAP_EXPORT)
pub const TEXT_HEAP: &str = "text_heap";
pub const MEMORY: &str = "memory";
const PAGE_BITS: u32 = 16;
const LIMB_BITS: i64 = 32;
/// Integers in [FIXNUM_MIN, FIXNUM_MAX] stand for themselves in an Int, others are handles (warp's big_int.rs)
const FIXNUM_MIN: i64 = -((1 << 62) - 1);
const FIXNUM_MAX: i64 = 1 << 62;

/// A node's parts: its kind, data and value fields
fn fields<T>(node: &Val, store: &mut StoreContextMut<'_, T>) -> Option<(i64, Val, Val)> {
	let node = as_struct(node, store)?;
	let mut field = |index| node.field(&mut *store, index).ok();
	Some((field(FIELD_KIND)?.unwrap_i64(), field(FIELD_DATA)?, field(FIELD_VALUE)?))
}

fn as_struct<T>(value: &Val, store: &StoreContextMut<'_, T>) -> Option<Rooted<StructRef>> {
	value.unwrap_anyref()?.unwrap_struct(store).ok()
}

/// A program's node as JSON: lists as arrays, a curly list of keys as an object, big integers and ratios as floats
pub fn json_of<T>(node: &Val, store: &mut StoreContextMut<'_, T>, memory: Memory) -> Result<Value> {
	let Some((kind, data, value)) = fields(node, store) else {
		return Ok(Value::Null);
	};
	let high_bits = (kind >> KIND_BITS) & 0xFF;
	Ok(match (kind & KIND_MASK) as u8 {
		EMPTY => Value::Null,
		INT if high_bits == BOOL_INFO => Value::Bool(integer_of(&data, store)? != 0.0),
		INT => match plain_integer(&data, store) {
			Some(integer) => Value::from(integer),
			None => Value::from(integer_of(&data, store)?),
		},
		FLOAT => Value::from(
			as_struct(&data, store).map_or(Ok(Val::F64(0)), |boxed| boxed.field(&mut *store, 0))?.unwrap_f64(),
		),
		CODEPOINT => {
			let code = data.unwrap_anyref().and_then(|reference| reference.unwrap_i31(&*store).ok());
			Value::String(code.and_then(|code| char::from_u32(code.get_u32())).unwrap_or('\0').to_string())
		}
		TEXT | SYMBOL | ERROR => Value::String(text_of(&data, value, store, memory)?),
		KEY => Value::Object(Map::from_iter([entry_of(&data, &value, store, memory)?])),
		BLOCK => object_of(items_of(data, value, store)?, store, memory)?,
		LIST if high_bits == CURLY => object_of(items_of(data, value, store)?, store, memory)?,
		LIST => Value::Array(
			items_of(data, value, store)?.iter().map(|item| json_of(item, store, memory)).collect::<Result<_>>()?,
		),
		FUNCTION => {
			wasmtime::bail!("an executable cannot give a warp function to a foreign call yet; run it with warp")
		}
		other => wasmtime::bail!("a value of kind {other} cannot cross to a foreign call from an executable yet"),
	})
}

/// The items of a list of cons cells: each cell's first item, then its rest, a cell or a last single item
pub fn items_of<T>(mut first: Val, mut rest: Val, store: &mut StoreContextMut<'_, T>) -> Result<Vec<Val>> {
	let mut items = vec![];
	loop {
		if first.unwrap_anyref().is_some() {
			items.push(first);
		}
		match fields(&rest, store) {
			Some((kind, data, value)) if matches!((kind & KIND_MASK) as u8, LIST | BLOCK) => {
				(first, rest) = (data, value)
			}
			Some((kind, ..)) if (kind & KIND_MASK) as u8 == EMPTY => return Ok(items),
			Some(_) => return Ok([items, vec![rest]].concat()),
			None => return Ok(items),
		}
	}
}

/// The parts of a node: a list's items, ø none, anything else itself (a call's arguments)
pub fn parts_of<T>(node: &Val, store: &mut StoreContextMut<'_, T>) -> Result<Vec<Val>> {
	match fields(node, store) {
		Some((kind, data, value)) if matches!((kind & KIND_MASK) as u8, LIST | BLOCK) => items_of(data, value, store),
		Some((kind, ..)) if (kind & KIND_MASK) as u8 != EMPTY => Ok(vec![*node]),
		_ => Ok(vec![]),
	}
}

/// A curly list: its keys as entries, other items numbered by their place
fn object_of<T>(items: Vec<Val>, store: &mut StoreContextMut<'_, T>, memory: Memory) -> Result<Value> {
	let mut object = Map::new();
	for item in items {
		match fields(&item, store) {
			Some((kind, key, value)) if (kind & KIND_MASK) as u8 == KEY => {
				let (key, value) = entry_of(&key, &value, store, memory)?;
				object.insert(key, value);
			}
			_ => {
				object.insert(object.len().to_string(), json_of(&item, store, memory)?);
			}
		}
	}
	Ok(Value::Object(object))
}

fn entry_of<T>(key: &Val, value: &Val, store: &mut StoreContextMut<'_, T>, memory: Memory) -> Result<(String, Value)> {
	let name = match json_of(key, store, memory)? {
		Value::String(name) => name,
		other => other.to_string(),
	};
	Ok((name, json_of(value, store, memory)?))
}

/// The integer of an Int payload `$i64box`, when it is one
fn plain_integer<T>(payload: &Val, store: &mut StoreContextMut<'_, T>) -> Option<i64> {
	match as_struct(payload, store)?.field(&mut *store, 0).ok()? {
		Val::I64(integer) => Some(integer),
		_ => None,
	}
}

/// An Int payload as a float: `$i64box`, `$BigInt(negative, limbs)` base 2^32, or a `$Ratio` of two payloads
fn integer_of<T>(payload: &Val, store: &mut StoreContextMut<'_, T>) -> Result<f64> {
	let payload = as_struct(payload, store).ok_or_else(|| wasmtime::Error::msg("an Int without payload"))?;
	Ok(match payload.field(&mut *store, 0)? {
		Val::I64(integer) => integer as f64,
		numerator @ Val::AnyRef(_) => {
			integer_of(&numerator, store)? / integer_of(&payload.field(&mut *store, 1)?, store)?
		}
		Val::I32(negative) => {
			let limbs = payload.field(&mut *store, 1)?;
			let limbs = limbs
				.unwrap_anyref()
				.ok_or_else(|| wasmtime::Error::msg("a BigInt without limbs"))?
				.unwrap_array(&*store)?;
			let magnitude = limbs
				.elems(&mut *store)?
				.collect::<Vec<_>>()
				.iter()
				.rev()
				.fold(0.0, |high, limb| high * 2f64.powi(LIMB_BITS as i32) + f64::from(limb.unwrap_i32() as u32));
			if negative != 0 {
				-magnitude
			} else {
				magnitude
			}
		}
		other => wasmtime::bail!("an unexpected Int payload {other:?}"),
	})
}

/// The text of a Text node from its data, after the texts before it when it is a rope (text_append: data the last
/// piece, the value field the text before it)
fn text_of<T>(data: &Val, before: Val, store: &mut StoreContextMut<'_, T>, memory: Memory) -> Result<String> {
	let mut pieces = vec![bytes_of(data, store, memory)?];
	let mut before = before;
	while let Some((_, data, earlier)) = fields(&before, store) {
		pieces.push(bytes_of(&data, store, memory)?);
		before = earlier;
	}
	Ok(String::from_utf8_lossy(&pieces.into_iter().rev().flatten().collect::<Vec<u8>>()).into_owned())
}

/// The bytes of a text: a GC byte array `$Text`, or a `$String(ptr, len)` in linear memory
fn bytes_of<T>(string: &Val, store: &mut StoreContextMut<'_, T>, memory: Memory) -> Result<Vec<u8>> {
	if let Some(Ok(array)) = string.unwrap_anyref().map(|reference| reference.unwrap_array(&*store)) {
		return Ok(array.elems(&mut *store)?.map(|byte| byte.unwrap_i32() as u8).collect());
	}
	let Some(string) = as_struct(string, store) else {
		return Ok(vec![]);
	};
	let pointer = string.field(&mut *store, 0)?.unwrap_i32() as u32 as usize;
	let length = string.field(&mut *store, 1)?.unwrap_i32() as u32 as usize;
	let bytes = memory
		.data(&*store)
		.get(pointer..pointer + length)
		.ok_or_else(|| wasmtime::Error::msg("a text beyond memory"))?;
	Ok(bytes.to_vec())
}

/// The constructors, memory and text heap of the program: what JSON is built into a node with
pub struct Builders {
	functions: Vec<(&'static str, Func)>,
	memory: Memory,
	heap: Option<Global>,
}

impl Builders {
	pub fn of(lookup: &mut dyn FnMut(&str) -> Option<wasmtime::Extern>) -> Result<Builders> {
		let functions = BUILDERS.iter().filter_map(|&name| Some((name, lookup(name)?.into_func()?))).collect();
		let memory = lookup(MEMORY)
			.and_then(|export| export.into_memory())
			.ok_or_else(|| wasmtime::Error::msg("the program exports no memory"))?;
		let heap = lookup(TEXT_HEAP).and_then(|export| export.into_global());
		Ok(Builders { functions, memory, heap })
	}

	fn call<T>(&self, name: &str, arguments: &[Val], result: Val, store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		let (_, function) = self
			.functions
			.iter()
			.find(|(known, _)| *known == name)
			.ok_or_else(|| wasmtime::Error::msg(format!("the program exports no {name}")))?;
		let mut results = [result];
		function.call(&mut *store, arguments, &mut results)?;
		Ok(results[0])
	}

	fn node<T>(&self, name: &str, arguments: &[Val], store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		self.call(name, arguments, Val::AnyRef(None), store)
	}

	fn int_operation<T>(&self, name: &str, a: i64, b: i64, store: &mut StoreContextMut<'_, T>) -> Result<i64> {
		Ok(self.call(name, &[Val::I64(a), Val::I64(b)], Val::I64(0), store)?.unwrap_i64())
	}

	/// JSON as a node of the program: null ø, booleans 1/0, objects `{key:value …}`, `{$int: digits}` an exact integer,
	/// `{$char: "b"}` a codepoint
	pub fn build<T>(&self, value: &Value, store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		match value {
			Value::Null => self.node(NEW_EMPTY, &[], store),
			Value::Bool(truth) => self.node(NEW_INT, &[Val::I64(i64::from(*truth))], store),
			Value::Number(number) => match number.as_i64() {
				Some(integer) => self.integer(&integer.to_string(), store),
				None => self.node(NEW_FLOAT, &[Val::F64(number.as_f64().unwrap_or(f64::NAN).to_bits())], store),
			},
			Value::String(text) => self.text(NEW_TEXT, text, store),
			Value::Array(items) => {
				self.list(items.iter().map(|item| self.build(item, store)).collect::<Result<_>>()?, SQUARE, store)
			}
			Value::Object(entries) => match single_text(entries) {
				Some((BIG_INTEGER_KEY, digits)) if is_integer(digits) => self.integer(digits, store),
				Some((CODEPOINT_KEY, letter)) if letter.chars().count() == 1 => {
					self.node(NEW_CODEPOINT, &[Val::I32(letter.chars().next().unwrap_or('\0') as i32)], store)
				}
				_ => {
					let mut keys = vec![];
					for (key, value) in entries {
						let (key, value) = (self.text(NEW_SYMBOL, key, store)?, self.build(value, store)?);
						keys.push(self.node(NEW_KEY, &[key, value, Val::I64(COLON)], store)?);
					}
					self.list(keys, CURLY, store)
				}
			},
		}
	}

	fn text<T>(&self, constructor: &str, text: &str, store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		let heap = self.heap.ok_or_else(|| wasmtime::Error::msg(format!("the program exports no {TEXT_HEAP}")))?;
		let (pointer, length) = write_to_heap(&self.memory, heap, store, text.as_bytes())?;
		self.node(constructor, &[Val::I32(pointer as i32), Val::I32(length as i32)], store)
	}

	/// ø for no items, as the emitter makes `[]`
	fn list<T>(&self, items: Vec<Val>, bracket: i64, store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		if items.is_empty() {
			return self.node(NEW_EMPTY, &[], store);
		}
		items
			.into_iter()
			.rev()
			.try_fold(Val::AnyRef(None), |rest, first| self.node(NEW_LIST, &[first, rest, Val::I64(bracket)], store))
	}

	/// The Int of decimal digits: a fixnum as it is, a larger integer composed from 32-bit limbs
	fn integer<T>(&self, digits: &str, store: &mut StoreContextMut<'_, T>) -> Result<Val> {
		let handle = match digits.parse::<i64>() {
			Ok(small) if (FIXNUM_MIN..=FIXNUM_MAX).contains(&small) => small,
			_ => {
				let (negative, magnitude) = digits.strip_prefix('-').map_or((false, digits), |rest| (true, rest));
				let mut handle = 0;
				for limb in limbs_of(magnitude).iter().rev() {
					handle = self.int_operation(SHIFT_LEFT, handle, LIMB_BITS, store)?;
					handle = self.int_operation(ADD, handle, i64::from(*limb), store)?;
				}
				if negative {
					handle = self.int_operation(SUBTRACT, 0, handle, store)?;
				}
				handle
			}
		};
		self.node(NEW_INT, &[Val::I64(handle)], store)
	}
}

fn single_text(entries: &Map<String, Value>) -> Option<(&str, &str)> {
	match entries.iter().next() {
		Some((key, Value::String(text))) if entries.len() == 1 => Some((key.as_str(), text.as_str())),
		_ => None,
	}
}

fn is_integer(digits: &str) -> bool {
	let magnitude = digits.strip_prefix('-').unwrap_or(digits);
	!magnitude.is_empty() && magnitude.bytes().all(|digit| digit.is_ascii_digit())
}

/// Decimal digits as base-2^32 limbs, least significant first
fn limbs_of(digits: &str) -> Vec<u32> {
	let mut limbs: Vec<u32> = vec![];
	for digit in digits.bytes() {
		let mut carry = u64::from(digit - b'0');
		for limb in limbs.iter_mut() {
			let product = u64::from(*limb) * 10 + carry;
			*limb = product as u32;
			carry = product >> LIMB_BITS;
		}
		if carry > 0 {
			limbs.push(carry as u32);
		}
	}
	limbs
}

/// Copy bytes into a module's memory at its text heap, for any store: fresh pages past the current memory when the heap
/// is unset or full, the module's own emit_text_allocation rule
pub fn write_to_heap<T>(
	memory: &Memory,
	heap: Global,
	store: &mut StoreContextMut<'_, T>,
	bytes: &[u8],
) -> Result<(u32, u32)> {
	let length = bytes.len() as u32;
	let top = heap.get(&mut *store).i32().unwrap_or(0) as u32;
	let memory_end = (memory.size(&*store) as u32) << PAGE_BITS;
	let pointer = if top == 0 || top + length > memory_end {
		memory.grow(&mut *store, ((length >> PAGE_BITS) + 1) as u64)?;
		memory_end
	} else {
		top
	};
	let written = write_at(memory, store, pointer, bytes)?;
	heap.set(&mut *store, Val::I32((pointer + length) as i32))?;
	Ok(written)
}

/// The bytes at `pointer`, the memory grown to hold them
pub fn write_at<T>(
	memory: &Memory,
	store: &mut StoreContextMut<'_, T>,
	pointer: u32,
	bytes: &[u8],
) -> Result<(u32, u32)> {
	let length = bytes.len() as u32;
	let pages_needed = ((pointer + length) as usize).div_ceil(1 << PAGE_BITS);
	let current_pages = memory.size(&*store) as usize;
	if pages_needed > current_pages {
		memory.grow(&mut *store, (pages_needed - current_pages) as u64)?;
	}
	memory.write(&mut *store, pointer as usize, bytes)?;
	Ok((pointer, length))
}

/// The host side of foreign_call(runtime, module, member, call, arguments) in an executable: python and js through the
/// bridge (foreign.rs), the answer built in the program; components and warp functions refused loudly
pub fn foreign_call<T>(
	caller: &mut wasmtime::Caller<'_, T>,
	parts: [Option<Rooted<AnyRef>>; 5],
) -> Result<Option<Rooted<AnyRef>>> {
	let builders = Builders::of(&mut |name| caller.get_export(name))?;
	let memory = builders.memory;
	let mut store = caller.as_context_mut();
	let [runtime, module, member, call, arguments] = parts.map(Val::AnyRef);
	let mut text = |value: &Val| match json_of(value, &mut store, memory)? {
		Value::String(text) => Ok(text),
		other => Ok::<_, wasmtime::Error>(other.to_string()),
	};
	let (runtime, member) = (text(&runtime)?, text(&member)?);
	if !crate::foreign::is_interpreted(&runtime) {
		wasmtime::bail!("{runtime} {member}: an executable calls python and js only; run it with warp");
	}
	let module = json_of(&module, &mut store, memory)?;
	let module_name = module.as_str().map_or_else(|| module.to_string(), str::to_string);
	let arguments = match json_of(&call, &mut store, memory)? {
		Value::Number(called) if called.as_i64() == Some(1) => Value::Array(
			parts_of(&arguments, &mut store)?
				.iter()
				.map(|argument| json_of(argument, &mut store, memory))
				.collect::<Result<_>>()?,
		),
		_ => Value::Null,
	};
	let mut no_callbacks = |_: usize, _: Value| Err("an executable cannot call a warp function back yet".to_string());
	let answer = crate::foreign::call(&runtime, &module_name, module, &member, arguments, &mut no_callbacks)
		.map_err(wasmtime::Error::msg)?;
	Ok(builders.build(&answer, &mut store)?.unwrap_anyref().copied())
}
