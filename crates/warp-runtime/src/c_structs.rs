//! C structs as warp maps (card wasmtime-ffi): a struct a header defines (`wasmtime_instance_t {uint64_t store_id;
//! size_t __private;}`) has a layout by the C rules of LP64 platforms; a call writes a map into its bytes and reads
//! them back as one. Numbers are numbers, pointers handle ids, arrays lists, nested structs and unions maps (a union
//! read back holds each of its members, the same bytes seen as each). A layout travels as text, so an executable
//! carries it with the C function: `{store_id:u64,__private:u64}`, a union in `<…>`, an array `[u8;16]`, a pointer
//! `ptr`; a field without a name is an anonymous member (C11), its fields the struct's own
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum CField {
	Int { bytes: usize, signed: bool },
	Float { bytes: usize },
	Pointer,
	Array(Box<CField>, usize),
	Struct(CStruct),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CStruct {
	pub fields: Vec<(String, CField)>,
	/// all fields at offset 0
	pub is_union: bool,
}

const POINTER_BYTES: usize = 8;
const SCALARS: [(&str, CField); 11] = [
	("i8", CField::Int { bytes: 1, signed: true }),
	("u8", CField::Int { bytes: 1, signed: false }),
	("i16", CField::Int { bytes: 2, signed: true }),
	("u16", CField::Int { bytes: 2, signed: false }),
	("i32", CField::Int { bytes: 4, signed: true }),
	("u32", CField::Int { bytes: 4, signed: false }),
	("i64", CField::Int { bytes: 8, signed: true }),
	("u64", CField::Int { bytes: 8, signed: false }),
	("f32", CField::Float { bytes: 4 }),
	("f64", CField::Float { bytes: 8 }),
	("ptr", CField::Pointer),
];

impl CField {
	pub fn size(&self) -> usize {
		match self {
			CField::Int { bytes, .. } | CField::Float { bytes } => *bytes,
			CField::Pointer => POINTER_BYTES,
			CField::Array(item, count) => item.size() * count,
			CField::Struct(inner) => inner.size(),
		}
	}

	pub fn align(&self) -> usize {
		match self {
			CField::Array(item, _) => item.align(),
			CField::Struct(inner) => inner.align(),
			scalar => scalar.size(),
		}
	}

	/// The field's text: `u64`, `[u8;16]`, a struct's
	pub fn text(&self) -> String {
		match self {
			CField::Array(item, count) => format!("[{};{count}]", item.text()),
			CField::Struct(inner) => inner.text(),
			scalar => SCALARS.iter().find(|(_, known)| known == scalar).map_or("?", |(name, _)| name).to_string(),
		}
	}

	fn read(&self, bytes: &[u8], pointer_id: &mut dyn FnMut(usize) -> i32) -> Value {
		let integer = |count: usize| {
			let mut buffer = [0u8; 8];
			buffer[..count].copy_from_slice(&bytes[..count]);
			u64::from_le_bytes(buffer)
		};
		match self {
			CField::Int { bytes: count, signed: true } => {
				let shift = 64 - 8 * count;
				Value::from(((integer(*count) << shift) as i64) >> shift)
			}
			CField::Int { bytes: count, signed: false } => Value::from(integer(*count)),
			CField::Float { bytes: 4 } => Value::from(f32::from_bits(integer(4) as u32) as f64),
			CField::Float { .. } => Value::from(f64::from_bits(integer(8))),
			CField::Pointer => Value::from(pointer_id(integer(POINTER_BYTES) as usize)),
			CField::Array(item, count) => {
				Value::Array((0..*count).map(|index| item.read(&bytes[index * item.size()..], pointer_id)).collect())
			}
			CField::Struct(inner) => inner.read(bytes, pointer_id),
		}
	}

	fn write(
		&self,
		value: &Value,
		bytes: &mut [u8],
		pointer_of: &mut dyn FnMut(i32) -> Option<usize>,
	) -> Result<(), String> {
		let mut put = |number: u64, count: usize| bytes[..count].copy_from_slice(&number.to_le_bytes()[..count]);
		match self {
			CField::Int { bytes: count, .. } => put(number_of(value)? as i64 as u64, *count),
			CField::Float { bytes: 4 } => put((number_of(value)? as f32).to_bits() as u64, 4),
			CField::Float { .. } => put(number_of(value)?.to_bits(), 8),
			CField::Pointer => {
				let id = number_of(value)? as i32;
				put(pointer_of(id).ok_or(format!("{id} is no C handle of this run"))? as u64, POINTER_BYTES)
			}
			CField::Array(item, count) => {
				let items = value.as_array().ok_or(format!("a list for a C array, not {value}"))?;
				for (index, item_value) in items.iter().take(*count).enumerate() {
					item.write(item_value, &mut bytes[index * item.size()..], pointer_of)?;
				}
			}
			CField::Struct(inner) => inner.write(value, bytes, pointer_of)?,
		}
		Ok(())
	}
}

fn number_of(value: &Value) -> Result<f64, String> {
	match value {
		Value::Bool(truth) => Ok(f64::from(u8::from(*truth))),
		Value::Number(number) => {
			number.as_i64().map(|integer| integer as f64).or(number.as_f64()).ok_or(number.to_string())
		}
		other => Err(format!("a number for a C field, not {other}")),
	}
}

impl CStruct {
	/// Each field with its offset
	fn placed(&self) -> Vec<(usize, &String, &CField)> {
		let mut offset: usize = 0;
		self.fields
			.iter()
			.map(|(name, field)| {
				if self.is_union {
					return (0, name, field);
				}
				offset = offset.next_multiple_of(field.align());
				let placed = (offset, name, field);
				offset += field.size();
				placed
			})
			.collect()
	}

	pub fn align(&self) -> usize {
		self.fields.iter().map(|(_, field)| field.align()).max().unwrap_or(1)
	}

	pub fn size(&self) -> usize {
		let end = self.placed().iter().map(|(offset, _, field)| offset + field.size()).max().unwrap_or(0);
		end.next_multiple_of(self.align())
	}

	pub fn text(&self) -> String {
		let fields: Vec<String> = self.fields.iter().map(|(name, field)| format!("{name}:{}", field.text())).collect();
		let (open, close) = if self.is_union { ('<', '>') } else { ('{', '}') };
		format!("{open}{}{close}", fields.join(","))
	}

	/// The layout a text gives, none for a damaged one
	pub fn of_text(text: &str) -> Option<CStruct> {
		match parse_field(text)? {
			(CField::Struct(layout), "") => Some(layout),
			_ => None,
		}
	}

	/// The struct in `bytes` as a map; `pointer_id` gives a pointer field its handle id
	pub fn read(&self, bytes: &[u8], pointer_id: &mut dyn FnMut(usize) -> i32) -> Value {
		let mut entries = Map::new();
		for (offset, name, field) in self.placed() {
			match field.read(&bytes[offset..], pointer_id) {
				// an anonymous member's fields are the struct's own
				Value::Object(inner) if name.is_empty() => entries.extend(inner),
				value => {
					entries.insert(name.clone(), value);
				}
			}
		}
		Value::Object(entries)
	}

	/// The map `value` written into `bytes` (a struct's size, zeroed): fields it leaves out stay 0
	pub fn write(
		&self,
		value: &Value,
		bytes: &mut [u8],
		pointer_of: &mut dyn FnMut(i32) -> Option<usize>,
	) -> Result<(), String> {
		let entries = value.as_object().ok_or(format!("a map for a C struct, not {value}"))?;
		for (offset, name, field) in self.placed() {
			if name.is_empty() {
				field.write(value, &mut bytes[offset..], pointer_of)?;
			} else if let Some(field_value) = entries.get(name) {
				field.write(field_value, &mut bytes[offset..], pointer_of)?;
			}
		}
		Ok(())
	}
}

/// The field a text starts with, and the text after it
fn parse_field(text: &str) -> Option<(CField, &str)> {
	if let Some(rest) = text.strip_prefix('[') {
		let (item, rest) = parse_field(rest)?;
		let (count, rest) = rest.strip_prefix(';')?.split_once(']')?;
		return Some((CField::Array(Box::new(item), count.parse().ok()?), rest));
	}
	let close = match text.chars().next()? {
		'{' => '}',
		'<' => '>',
		_ => {
			let end = text.find([',', '}', '>', ';', ']']).unwrap_or(text.len());
			let (_, scalar) = SCALARS.iter().find(|(name, _)| *name == &text[..end])?;
			return Some((scalar.clone(), &text[end..]));
		}
	};
	let mut fields = vec![];
	let mut rest = &text[1..];
	while !rest.starts_with(close) {
		let (name, after) = rest.split_once(':')?;
		let (field, after) = parse_field(after)?;
		fields.push((name.to_string(), field));
		rest = after.strip_prefix(',').unwrap_or(after);
	}
	Some((CField::Struct(CStruct { fields, is_union: close == '>' }), &rest[1..]))
}
