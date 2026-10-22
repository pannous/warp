//! A program's GC nodes (CLAUDE.md "Serialization") as host values and back: what std_io reads and answers, what os.keep
//! keeps across runs, what a run's value prints as
use crate::Host;
use warp_runtime::gc_values::{items_of, BLOCK, BOOL_INFO, CODEPOINT, EMPTY, ERROR, FLOAT, INT, KEY, KIND_BITS};
use warp_runtime::gc_values::{write_to_heap, KIND_MASK, LIST, SYMBOL, TEXT, TEXT_HEAP};
use wasmtime::{Extern, Func, Global, Memory, Result, StoreContextMut, Val};

const TYPE_DEF: u8 = 12;
const PAGE_BITS: u32 = 16;
/// A list's bracket code above the Kind (lib/compiler/kinds.warp BracketCode)
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

	/// A text's, symbol's or error's content, anything else as shown
	pub fn content(&self) -> String {
		match self {
			Node::Text(bytes) | Node::Symbol(bytes) | Node::Error(bytes) => String::from_utf8_lossy(bytes).into_owned(),
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

	pub fn shown(&self) -> String {
		let quoted = |bytes: &[u8]| format!("{:?}", String::from_utf8_lossy(bytes));
		match self {
			Node::Empty => "ø".into(),
			Node::Int(n) => n.to_string(),
			Node::Bool(truth) => (if *truth { "true" } else { "false" }).into(),
			Node::Float(x) => format!("{x:?}"),
			Node::Codepoint(code) => format!("'{}'", char::from_u32(*code).unwrap_or('\u{fffd}')),
			Node::Text(bytes) => quoted(bytes),
			Node::Symbol(bytes) => String::from_utf8_lossy(bytes).into_owned(),
			Node::Error(bytes) => format!("Error({})", quoted(bytes)),
			Node::Key(kind, key, value) if kind >> KIND_BITS & 0xFF == 1 => format!("{}: {}", key.shown(), value.shown()),
			Node::Key(_, key, value) => format!("{}={}", key.shown(), value.shown()),
			Node::List(kind, items) => {
				let (open, close) = BRACKETS.get((kind >> KIND_BITS & 0xFF) as usize).copied().unwrap_or(("", ""));
				format!("{open}{}{close}", items.iter().map(Node::shown).collect::<Vec<_>>().join(" "))
			}
			Node::Type(name, body) => format!("{}{}", name.shown(), body.shown()),
			Node::Other(kind) => format!("<a value of kind {}>", kind & KIND_MASK),
		}
	}
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
		INT => match boxed(&data, store) {
			Some(Val::I64(n)) if info == BOOL_INFO => Node::Bool(n != 0),
			Some(Val::I64(n)) => Node::Int(n),
			_ => Node::Other(kind),
		},
		FLOAT => boxed(&data, store).and_then(|x| x.f64()).map_or(Node::Other(kind), Node::Float),
		CODEPOINT => {
			let code = data.unwrap_anyref().and_then(|code| code.unwrap_i31(&*store).ok());
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

/// The constructors a program exports for its host calls (lib/compiler/emitter/host.warp host_builders) and its memory
pub struct Builders {
	memory: Memory,
	heap: Option<Global>,
	functions: Vec<(&'static str, Func)>,
}

const CONSTRUCTORS: [&str; 11] = [
	"new_empty", "new_int", "new_bool", "new_float", "new_codepoint", "new_text", "new_symbol", "new_key", "new_list",
	"new_type", "error_of",
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
/// (compiler.wasm), else in pages past its own, the same pages again while the memory stays that size
pub fn placed(memory: Memory, heap: Option<Global>, bytes: &[u8], store: &mut StoreContextMut<'_, Host>) -> Result<u32> {
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
