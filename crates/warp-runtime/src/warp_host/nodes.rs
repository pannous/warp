//! A program's GC nodes (CLAUDE.md "Serialization") as host values and back: what std_io reads and answers, what os.keep
//! keeps across runs, what a run's value prints as
use crate::Host;
use warp_runtime::gc_values::{items_of, BLOCK, BOOL_INFO, CODEPOINT, EMPTY, ERROR, FLOAT, INT, KEY, KIND_BITS};
use warp_runtime::gc_values::{write_to_heap, KIND_MASK, LIST, SYMBOL, TEXT, TEXT_HEAP};
use wasmtime::{Extern, Func, Global, Memory, Result, StoreContextMut, Val};

const TYPE_DEF: u8 = 12;
const PAGE_BITS: u32 = 16;
/// A list's bracket code above the Kind (core/kinds.warp BracketCode)
const BRACKETS: [(&str, &str); 4] = [("{", "}"), ("[", "]"), ("(", ")"), ("<", ">")];
/// A curly list of `key: value` entries, and its entries' kind (kinds.warp Marked)
pub const MAP: i64 = LIST as i64;
pub const ENTRY: i64 = KEY as i64 + (1 << KIND_BITS);
pub const SQUARE_LIST: i64 = LIST as i64 + (1 << KIND_BITS);

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
	Empty,
	Int(i64),
	Bool(bool),
	Float(f64),
	/// an exact rational, numerator and denominator (above 1): the pure lane's `2.5` and `1/3`, an Int node holding a
	/// $Ratio of two $i64box (core/emitter.warp gc_types)
	Ratio(i64, i64),
	Codepoint(u32),
	Text(Vec<u8>),
	Symbol(Vec<u8>),
	/// the error's message
	Error(Vec<u8>),
	/// a key's and a list's whole kind: its operator, or bracket and separator, above the Kind
	Key(i64, Box<Node>, Box<Node>),
	List(i64, Vec<Node>),
	Type(Box<Node>, Box<Node>),
	/// a value of this kind warp-host neither builds nor keeps (a function, a big integer…), only names
	Other(i64),
}

impl Node {
	pub fn text(text: impl Into<String>) -> Node {
		Node::Text(text.into().into_bytes())
	}

	/// `{status: 3, out: "o"}`
	pub fn map(entries: Vec<(&str, Node)>) -> Node {
		let entry = |(name, value): (&str, Node)| {
			Node::Key(ENTRY, Box::new(Node::Symbol(name.as_bytes().to_vec())), Box::new(value))
		};
		Node::List(MAP, entries.into_iter().map(entry).collect())
	}

	/// `{name: value}` with names made at run time (a module's exports, a record's fields)
	pub fn named_map(entries: Vec<(String, Node)>) -> Node {
		let entry = |(name, value): (String, Node)| {
			Node::Key(ENTRY, Box::new(Node::Symbol(name.into_bytes())), Box::new(value))
		};
		Node::List(MAP, entries.into_iter().map(entry).collect())
	}

	/// A text's, symbol's or error's content, a codepoint's character (a one-letter text such as "." arrives as one),
	/// anything else as shown
	pub fn content(&self) -> String {
		match self {
			Node::Text(bytes) | Node::Symbol(bytes) | Node::Error(bytes) => String::from_utf8_lossy(bytes).into_owned(),
			Node::Codepoint(code) => char::from_u32(*code).unwrap_or('\u{fffd}').to_string(),
			other => other.shown(),
		}
	}

	/// The items of a list, ø none, anything else itself
	pub fn items(self) -> Vec<Node> {
		match self {
			Node::List(_, items) => items,
			Node::Empty => vec![],
			other => vec![other],
		}
	}

	/// The node as warp-rust prints a run's value (src/node/serialization.rs serialize): ints and floats as numbers (4,
	/// not 4.0), yes and no, texts in double quotes with only a quote or `\\` escaped, a character in single quotes,
	/// `key:value` with the key bare
	pub fn shown(&self) -> String {
		match self {
			Node::Empty => "ø".into(),
			Node::Int(n) => n.to_string(),
			Node::Bool(truth) => (if *truth { "yes" } else { "no" }).into(),
			Node::Float(x) => float_text(*x),
			Node::Ratio(numerator, denominator) => ratio_shown(*numerator, *denominator),
			Node::Codepoint(code) => quoted(&char::from_u32(*code).unwrap_or('\u{fffd}').to_string(), '\''),
			Node::Text(bytes) => quoted(&String::from_utf8_lossy(bytes), '"'),
			Node::Symbol(bytes) => String::from_utf8_lossy(bytes).into_owned(),
			Node::Error(bytes) => format!("Error({})", quoted(&String::from_utf8_lossy(bytes), '"')),
			Node::Key(kind, key, value) if kind >> KIND_BITS & 0xFF == 1 => format!("{}:{}", key.content(), value.shown()),
			Node::Key(_, key, value) => format!("{}={}", key.shown(), value.shown()),
			Node::List(kind, items) => {
				let (open, close) = BRACKETS.get((kind >> KIND_BITS & 0xFF) as usize).copied().unwrap_or(("", ""));
				let items = items.iter().map(|item| item.item_shown(!open.is_empty())).collect::<Vec<_>>();
				format!("{open}{}{close}", items.join(" "))
			}
			Node::Type(name, body) => format!("{}{}", name.shown(), body.shown()),
			Node::Other(kind) => format!("<a value of kind {}>", kind & KIND_MASK),
		}
	}

	/// A list's item: a one-letter text in brackets in single quotes, as a character prints, and a character without
	/// brackets in double quotes (serialization.rs item_text)
	fn item_shown(&self, bracketed: bool) -> String {
		match self {
			Node::Text(bytes) if bracketed && String::from_utf8_lossy(bytes).chars().count() == 1 => {
				quoted(&String::from_utf8_lossy(bytes), '\'')
			}
			Node::Codepoint(code) if !bracketed => quoted(&char::from_u32(*code).unwrap_or('\u{fffd}').to_string(), '"'),
			_ => self.shown(),
		}
	}
}

/// A text in quotes, a quote or `\\` in it escaped by a `\\` so it reads back (serialization.rs quoted_by)
fn quoted(text: &str, quote: char) -> String {
	let mut written = String::from(quote);
	for c in text.chars() {
		if c == quote || c == '\\' {
			written.push('\\');
		}
		written.push(c);
	}
	written.push(quote);
	written
}

/// A float as warp writes it (src/extensions/numbers.rs float_text): the shortest digits that read back, positional
/// for decimal exponents in [-5, 15), else `1.5e-7`; a whole float without `.0`; NaN, ∞ and -∞
pub fn float_text(x: f64) -> String {
	const SMALLEST_POSITIONAL: i32 = -5;
	const LARGEST_POSITIONAL: i32 = 15;
	if x.is_nan() {
		return "NaN".into();
	}
	if x.is_infinite() {
		return if x > 0.0 { "∞" } else { "-∞" }.into();
	}
	if x == 0.0 {
		return "0".into();
	}
	let written = format!("{:e}", x.abs()); // Rust writes the shortest digits that read back
	let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
	let digits = mantissa.replace('.', "");
	let exponent: i32 = exponent.parse().unwrap_or(0);
	let count = digits.len() as i32;
	let sign = if x < 0.0 { "-" } else { "" };
	let zeros = |how_many: i32| "0".repeat(how_many.max(0) as usize);
	let placed = if !(SMALLEST_POSITIONAL..LARGEST_POSITIONAL).contains(&exponent) {
		let rest = if count > 1 { format!(".{}", &digits[1..]) } else { String::new() };
		format!("{}{rest}e{exponent}", &digits[..1])
	} else if exponent >= count - 1 {
		digits.clone() + &zeros(exponent - count + 1)
	} else if exponent >= 0 {
		let (before, after) = digits.split_at(exponent as usize + 1);
		format!("{before}.{after}")
	} else {
		format!("0.{}{digits}", zeros(-1 - exponent))
	};
	format!("{sign}{placed}")
}

/// A node's kind, data and value fields; None for null
fn fields(node: &Val, store: &mut StoreContextMut<'_, Host>) -> Option<(i64, Val, Val)> {
	let node = node.anyref().flatten()?.unwrap_struct(&*store).ok()?;
	let mut field = |index| node.field(&mut *store, index).ok();
	Some((field(0)?.i64()?, field(1)?, field(2)?))
}

/// The first field of a boxed number, `$i64box` or `$f64box`
fn boxed(data: &Val, store: &mut StoreContextMut<'_, Host>) -> Option<Val> {
	data.unwrap_anyref()?.unwrap_struct(&*store).ok()?.field(&mut *store, 0).ok()
}

/// The numerator and denominator of a $Ratio(numerator, denominator), each an $i64box
fn ratio_of(data: &Val, store: &mut StoreContextMut<'_, Host>) -> Option<(i64, i64)> {
	let ratio = data.anyref().flatten()?.as_struct(&*store).ok().flatten()?;
	if ratio.ty(&*store).ok()?.fields().len() != 2 {
		return None;
	}
	// each part an $i64box (warp-self's), or the number itself
	let mut part = |index: usize| match ratio.field(&mut *store, index).ok()? {
		Val::I64(n) => Some(n),
		part @ Val::AnyRef(Some(_)) => boxed(&part, store)?.i64(),
		_ => None,
	};
	match (part(0)?, part(1)?) {
		(n, d) if d != 0 => Some((n, d)),
		_ => None,
	}
}

/// An exact rational as warp prints it (src/extensions/numbers.rs Number::Quotient, terminating_decimal): a
/// terminating decimal positionally with all its digits (2.5, 0.3, 0.00000095367431640625), any other as
/// numerator/denominator (1/3)
fn ratio_shown(numerator: i64, denominator: i64) -> String {
	fn gcd(a: i128, b: i128) -> i128 {
		if b == 0 { a.abs() } else { gcd(b, a % b) }
	}
	let divisor = gcd(numerator.into(), denominator.into()).max(1) * i128::from(denominator.signum());
	let (numerator, denominator) = (i128::from(numerator) / divisor, i128::from(denominator) / divisor);
	let (mut rest, mut digits) = (denominator, 0u32);
	for factor in [2, 5] {
		let mut count = 0;
		while rest % factor == 0 {
			rest /= factor;
			count += 1;
		}
		digits = digits.max(count);
	}
	let scaled = 10i128.checked_pow(digits).and_then(|scale| numerator.checked_mul(scale)).map(|n| n / denominator);
	let (true, Some(scaled)) = (rest == 1, scaled) else {
		return format!("{numerator}/{denominator}");
	};
	let sign = if scaled < 0 { "-" } else { "" };
	let padded = format!("{:0>width$}", scaled.unsigned_abs(), width = digits as usize + 1);
	let (whole, fraction) = padded.split_at(padded.len() - digits as usize);
	if fraction.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") }
}

/// The program's node as a host value; a main's plain result as its number
pub fn read(node: &Val, store: &mut StoreContextMut<'_, Host>, memory: Option<Memory>) -> Result<Node> {
	let Some((kind, data, value)) = fields(node, store) else {
		return Ok(match node {
			Val::I64(n) => Node::Int(*n),
			Val::I32(n) => Node::Int(i64::from(*n)),
			Val::F64(bits) => Node::Float(f64::from_bits(*bits)),
			_ => Node::Empty,
		});
	};
	let info = kind >> KIND_BITS & 0xFF;
	Ok(match (kind & KIND_MASK) as u8 {
		EMPTY => Node::Empty,
		INT => match data
			.anyref()
			.flatten()
			.and_then(|small| small.as_i31(&*store).ok().flatten())
			.map(|small| small.get_i32())
		{
			Some(n) if info == BOOL_INFO => Node::Bool(n != 0),
			Some(n) => Node::Int(i64::from(n)),
			None => match boxed(&data, store) {
				Some(Val::I64(n)) if info == BOOL_INFO => Node::Bool(n != 0),
				Some(Val::I64(n)) => Node::Int(n),
				// a number whose kind says int and whose box holds a float (`x = 7.0; x /= 2`): the float
				Some(Val::F64(bits)) => Node::Float(f64::from_bits(bits)),
				// an exact rational (`3.5`, `0.1 + 0.2`; core/runtime/exact.warp): a $Ratio of two $i64box
				_ => match ratio_of(&data, store) {
					Some((n, 1)) => Node::Int(n),
					Some((n, d)) => Node::Ratio(n, d),
					None => Node::Other(kind),
				},
			},
		},
		FLOAT => boxed(&data, store).and_then(|x| x.f64()).map_or(Node::Other(kind), Node::Float),
		CODEPOINT => {
			let code = data.unwrap_anyref().and_then(|code| code.as_i31(&*store).ok().flatten());
			code.map_or(Node::Other(kind), |code| Node::Codepoint(code.get_u32()))
		}
		TEXT => Node::Text(text_of(data, value, store, memory)?),
		SYMBOL => Node::Symbol(text_of(data, value, store, memory)?),
		ERROR => Node::Error(text_of(data, value, store, memory)?),
		KEY => Node::Key(kind, Box::new(read(&data, store, memory)?), Box::new(read(&value, store, memory)?)),
		TYPE_DEF => Node::Type(Box::new(read(&data, store, memory)?), Box::new(read(&value, store, memory)?)),
		LIST | BLOCK => {
			let items = items_of(data, value, store)?;
			Node::List(kind, items.iter().map(|item| read(item, store, memory)).collect::<Result<_>>()?)
		}
		_ => Node::Other(kind),
	})
}

/// A text's bytes: a rope's pieces (text_append: the data the last piece, the value the text before it), each a GC byte
/// array or a `$String(ptr, len)` in linear memory
fn text_of(data: Val, before: Val, store: &mut StoreContextMut<'_, Host>, memory: Option<Memory>) -> Result<Vec<u8>> {
	let mut pieces = vec![bytes_of(&data, store, memory)?];
	let mut before = before;
	while let Some((_, data, earlier)) = fields(&before, store) {
		pieces.push(bytes_of(&data, store, memory)?);
		before = earlier;
	}
	Ok(pieces.into_iter().rev().flatten().collect())
}

fn bytes_of(piece: &Val, store: &mut StoreContextMut<'_, Host>, memory: Option<Memory>) -> Result<Vec<u8>> {
	let Some(piece) = piece.unwrap_anyref() else { return Ok(vec![]) };
	if let Ok(array) = piece.unwrap_array(&*store) {
		return Ok(array.elems(&mut *store)?.map(|byte| byte.unwrap_i32() as u8).collect());
	}
	let string = piece.unwrap_struct(&*store)?;
	let pointer = string.field(&mut *store, 0)?.unwrap_i32() as u32 as usize;
	let length = string.field(&mut *store, 1)?.unwrap_i32() as u32 as usize;
	let memory = memory.ok_or_else(|| wasmtime::Error::msg("a text in memory of a program without memory"))?;
	let bytes = memory.data(&*store).get(pointer..pointer + length);
	Ok(bytes.ok_or_else(|| wasmtime::Error::msg("a text beyond memory"))?.to_vec())
}

/// The constructors a program exports for its host calls (core/emitter/host.warp host_builders) and its memory
pub struct Builders {
	memory: Memory,
	heap: Option<Global>,
	functions: Vec<(&'static str, Func)>,
}

const CONSTRUCTORS: [&str; 11] = [
	"new_empty",
	"new_int",
	"new_bool",
	"new_float",
	"new_codepoint",
	"new_text",
	"new_symbol",
	"new_key",
	"new_list",
	"new_type",
	"error_of",
];

impl Builders {
	pub fn of(mut export: impl FnMut(&str) -> Option<Extern>) -> Option<Builders> {
		let memory = export("memory")?.into_memory()?;
		let functions = CONSTRUCTORS.iter().filter_map(|name| Some((*name, export(name)?.into_func()?))).collect();
		let heap = export(TEXT_HEAP).and_then(Extern::into_global);
		Some(Builders { memory, heap, functions })
	}

	pub fn memory(&self) -> Memory {
		self.memory
	}

	pub fn build(&self, node: &Node, store: &mut StoreContextMut<'_, Host>) -> Result<Val> {
		match node {
			Node::Empty => self.call("new_empty", &[], store),
			Node::Int(n) => self.call("new_int", &[Val::I64(*n)], store),
			Node::Bool(truth) => self.call("new_bool", &[Val::I64(i64::from(*truth))], store),
			Node::Float(x) => self.call("new_float", &[Val::F64(x.to_bits())], store),
			Node::Ratio(numerator, denominator) => self.ratio(*numerator, *denominator, store),
			Node::Codepoint(code) => self.call("new_codepoint", &[Val::I32(*code as i32)], store),
			Node::Text(bytes) => self.text("new_text", bytes, store),
			Node::Symbol(bytes) => self.text("new_symbol", bytes, store),
			Node::Error(bytes) => {
				let message = self.text("new_text", bytes, store)?;
				self.call("error_of", &[message], store)
			}
			Node::Key(kind, key, value) => {
				let (key, value) = (self.build(key, store)?, self.build(value, store)?);
				self.call("new_key", &[key, value, Val::I64(kind >> KIND_BITS)], store)
			}
			Node::List(_, items) if items.is_empty() => self.call("new_empty", &[], store),
			Node::List(kind, items) if (kind & KIND_MASK) as u8 == LIST => {
				let items = items.iter().map(|item| self.build(item, store)).collect::<Result<Vec<_>>>()?;
				if let Some(list) = self.vector_list(&items, kind >> KIND_BITS, store)? {
					return Ok(list);
				}
				items.into_iter().rev().try_fold(Val::AnyRef(None), |rest, first| {
					self.call("new_list", &[first, rest, Val::I64(kind >> KIND_BITS)], store)
				})
			}
			Node::Type(name, body) => {
				let (name, body) = (self.build(name, store)?, self.build(body, store)?);
				self.call("new_type", &[name, body], store)
			}
			other => wasmtime::bail!("warp-host cannot give a program {}", other.shown()),
		}
	}

	/// An Int node holding a $Ratio of two $i64box, as the program makes `2.5`: no builder makes one, so the structs are
	/// made here, of the types the program's own (structural, final) $i64box and $Ratio are, the node of new_int's type
	fn ratio(&self, numerator: i64, denominator: i64, store: &mut StoreContextMut<'_, Host>) -> Result<Val> {
		use wasmtime::{FieldType, Mutability, StorageType, StructRef, StructRefPre, StructType, ValType};
		let int = self.call("new_int", &[Val::I64(numerator)], store)?;
		let node = int.unwrap_anyref().and_then(|node| node.as_struct(&*store).ok().flatten());
		let node = node.ok_or_else(|| wasmtime::Error::msg("new_int made no node"))?;
		let node_type = node.ty(&*store)?;
		let engine = store.engine().clone();
		let field = |value: ValType| FieldType::new(Mutability::Const, StorageType::ValType(value));
		let int_box = StructRefPre::new(&mut *store, StructType::new(&engine, [field(ValType::I64)])?);
		let ratio =
			StructRefPre::new(&mut *store, StructType::new(&engine, [field(ValType::ANYREF), field(ValType::ANYREF)])?);
		let boxed = |n: i64, store: &mut StoreContextMut<'_, Host>| -> Result<Val> {
			Ok(Val::AnyRef(Some(StructRef::new(&mut *store, &int_box, &[Val::I64(n)])?.to_anyref())))
		};
		let parts = [boxed(numerator, store)?, boxed(denominator, store)?];
		let ratio = StructRef::new(&mut *store, &ratio, &parts)?.to_anyref();
		let kind = node.field(&mut *store, 0)?;
		let allocator = StructRefPre::new(&mut *store, node_type);
		let built = StructRef::new(&mut *store, &allocator, &[kind, Val::AnyRef(Some(ratio)), Val::AnyRef(None)])?;
		Ok(Val::AnyRef(Some(built.to_anyref())))
	}

	/// A list of the items at once, for a program whose lists are vectors (a node over a `{count: i32, items: array}`
	/// struct, warp-self's): new_list(first, rest) copies rest's items into a new vector, so building a list a cell
	/// at a time copies n²/2 items (24,000 bytes of read_bytes: 288 million, past the memory cap). Here new_list makes
	/// a one-item list, and its vector gets an array of all the items. None for a program whose lists are cons cells.
	fn vector_list(&self, items: &[Val], info: i64, store: &mut StoreContextMut<'_, Host>) -> Result<Option<Val>> {
		let Some(first) = items.first() else { return Ok(None) };
		let list = self.call("new_list", &[*first, Val::AnyRef(None), Val::I64(info)], store)?;
		let Some((_, data, _)) = fields(&list, store) else { return Ok(None) };
		let Some(vector) = data.unwrap_anyref().and_then(|data| data.as_struct(&*store).ok().flatten()) else {
			return Ok(None);
		};
		let room = vector.field(&mut *store, 1).ok().and_then(|room| room.unwrap_anyref().copied());
		let Some(room) = room.and_then(|room| room.as_array(&*store).ok().flatten()) else { return Ok(None) };
		if !matches!(vector.field(&mut *store, 0), Ok(Val::I32(_))) {
			return Ok(None);
		}
		let room_type = room.ty(&*store)?;
		let allocator = wasmtime::ArrayRefPre::new(&mut *store, room_type);
		let array = wasmtime::ArrayRef::new_fixed(&mut *store, &allocator, items)?;
		if vector.set_field(&mut *store, 1, Val::AnyRef(Some(array.to_anyref()))).is_err() {
			return Ok(None);
		}
		vector.set_field(&mut *store, 0, Val::I32(items.len() as i32))?;
		Ok(Some(list))
	}

	fn call(&self, name: &str, arguments: &[Val], store: &mut StoreContextMut<'_, Host>) -> Result<Val> {
		let function = self.functions.iter().find(|(export, _)| *export == name).map(|(_, function)| function);
		let function = function.ok_or_else(|| wasmtime::Error::msg(format!("the program exports no {name}")))?;
		let mut result = [Val::AnyRef(None)];
		function.call(&mut *store, arguments, &mut result)?;
		Ok(result[0])
	}

	fn text(&self, constructor: &str, bytes: &[u8], store: &mut StoreContextMut<'_, Host>) -> Result<Val> {
		let place = placed(self.memory, self.heap, bytes, store)?;
		self.call(constructor, &[Val::I32(place as i32), Val::I32(bytes.len() as i32)], store)
	}
}

/// Bytes put into the program's memory for it to copy out at once, their address: at its text heap when it has one
/// (warp.wasm), else in pages past its own, the same pages again while the memory stays that size
pub fn placed(
	memory: Memory,
	heap: Option<Global>,
	bytes: &[u8],
	store: &mut StoreContextMut<'_, Host>,
) -> Result<u32> {
	if let Some(heap) = heap {
		return Ok(write_to_heap(&memory, heap, store, bytes)?.0);
	}
	let pages = memory.size(&*store);
	let place = match store.data().scratch {
		Some((at, size)) if size == pages && u64::from(at) + bytes.len() as u64 <= pages << PAGE_BITS => at,
		_ => {
			let end = (pages << PAGE_BITS) as u32;
			memory.grow(&mut *store, (bytes.len() as u64 >> PAGE_BITS) + 1)?;
			store.data_mut().scratch = Some((end, memory.size(&*store)));
			end
		}
	};
	memory.write(&mut *store, place as usize, bytes)?;
	Ok(place)
}

/// A value as bytes the next run reads back (os.keep): a tag, then its parts; None for one warp-host doesn't keep
pub fn encode(node: &Node, out: &mut Vec<u8>) -> Option<()> {
	let bytes = |tag: u8, bytes: &[u8], out: &mut Vec<u8>| {
		out.push(tag);
		out.extend((bytes.len() as u64).to_le_bytes());
		out.extend(bytes);
	};
	match node {
		Node::Empty => out.push(0),
		Node::Int(n) => bytes(1, &n.to_le_bytes(), out),
		Node::Bool(truth) => bytes(2, &[u8::from(*truth)], out),
		Node::Float(x) => bytes(3, &x.to_le_bytes(), out),
		Node::Ratio(numerator, denominator) => {
			bytes(11, &[numerator.to_le_bytes(), denominator.to_le_bytes()].concat(), out)
		}
		Node::Codepoint(code) => bytes(4, &code.to_le_bytes(), out),
		Node::Text(text) => bytes(5, text, out),
		Node::Symbol(text) => bytes(6, text, out),
		Node::Error(text) => bytes(7, text, out),
		Node::Key(kind, key, value) => {
			bytes(8, &kind.to_le_bytes(), out);
			encode(key, out)?;
			encode(value, out)?;
		}
		Node::List(kind, items) if (kind & KIND_MASK) as u8 == LIST => {
			bytes(9, &kind.to_le_bytes(), out);
			out.extend((items.len() as u64).to_le_bytes());
			items.iter().try_for_each(|item| encode(item, out))?;
		}
		Node::Type(name, body) => {
			out.push(10);
			encode(name, out)?;
			encode(body, out)?;
		}
		Node::List(..) | Node::Other(_) => return None,
	}
	Some(())
}

pub fn decode(input: &mut &[u8]) -> Option<Node> {
	fn take<'a>(input: &mut &'a [u8], count: usize) -> Option<&'a [u8]> {
		let (taken, rest) = (input.get(..count)?, input.get(count..)?);
		*input = rest;
		Some(taken)
	}
	fn number(input: &mut &[u8]) -> Option<u64> {
		Some(u64::from_le_bytes(take(input, 8)?.try_into().ok()?))
	}
	fn part<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
		let length = number(input)? as usize;
		take(input, length)
	}
	let tag = take(input, 1)?[0];
	let node = |input: &mut &[u8]| decode(input).map(Box::new);
	Some(match tag {
		0 => Node::Empty,
		1 => Node::Int(i64::from_le_bytes(part(input)?.try_into().ok()?)),
		2 => Node::Bool(part(input)? == [1]),
		3 => Node::Float(f64::from_le_bytes(part(input)?.try_into().ok()?)),
		11 => {
			let (numerator, denominator) = part(input)?.split_at_checked(8)?;
			Node::Ratio(
				i64::from_le_bytes(numerator.try_into().ok()?),
				i64::from_le_bytes(denominator.try_into().ok()?),
			)
		}
		4 => Node::Codepoint(u32::from_le_bytes(part(input)?.try_into().ok()?)),
		5 => Node::Text(part(input)?.to_vec()),
		6 => Node::Symbol(part(input)?.to_vec()),
		7 => Node::Error(part(input)?.to_vec()),
		8 => {
			let kind = i64::from_le_bytes(part(input)?.try_into().ok()?);
			Node::Key(kind, node(input)?, node(input)?)
		}
		9 => {
			let kind = i64::from_le_bytes(part(input)?.try_into().ok()?);
			let count = number(input)?;
			Node::List(kind, (0..count).map(|_| decode(input)).collect::<Option<_>>()?)
		}
		10 => Node::Type(node(input)?, node(input)?),
		_ => return None,
	})
}
