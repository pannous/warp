//! The structs and unions headers define, and the scalar types they name (`typedef uint8_t wasmtime_valkind_t;`), as
//! the layout texts of warp_runtime::c_structs (card wasmtime-ffi): a pointer to one crosses as a warp map. A struct
//! with a field of a type no header defines has no layout: a pointer to it stays a handle

use std::collections::HashMap;
use std::sync::Mutex;

/// What a header says a type name is
#[derive(Clone, Debug)]
enum CTypeDefinition {
	/// `typedef uint8_t wasmtime_v128[16];`: another type, an array of it with a count
	Alias(String, Option<usize>),
	/// a struct or union body: its fields (type, name, array count)
	Body { is_union: bool, fields: Vec<(String, String, Option<usize>)> },
}

/// The C scalar types: their layout texts
const SCALAR_LAYOUTS: [(&str, &str); 27] = [
	("char", "i8"),
	("signed char", "i8"),
	("unsigned char", "u8"),
	("bool", "u8"),
	("_Bool", "u8"),
	("int8_t", "i8"),
	("uint8_t", "u8"),
	("short", "i16"),
	("unsigned short", "u16"),
	("int16_t", "i16"),
	("uint16_t", "u16"),
	("int", "i32"),
	("unsigned", "u32"),
	("unsigned int", "u32"),
	("int32_t", "i32"),
	("uint32_t", "u32"),
	("long", "i64"),
	("unsigned long", "u64"),
	("long long", "i64"),
	("unsigned long long", "u64"),
	("int64_t", "i64"),
	("uint64_t", "u64"),
	("size_t", "u64"),
	("ssize_t", "i64"),
	("uintptr_t", "u64"),
	("float", "f32"),
	("double", "f64"),
];
/// Nested definitions deeper than this have no layout (a struct containing itself)
const MAX_NESTING: usize = 8;

static DEFINITIONS: Mutex<Option<HashMap<String, CTypeDefinition>>> = Mutex::new(None);

/// A body being read: whether a union, its tag name, its fields so far
type OpenBody = (bool, String, Vec<(String, String, Option<usize>)>);

/// The type name an anonymous member (`struct { uint64_t store_id; uint32_t __private1; };`, C11) is kept under;
/// its fields are the enclosing struct's (c_structs.rs: a field without a name)
const ANONYMOUS_PREFIX: &str = "anonymous ";

/// Reads a header's struct bodies and typedefs line by line (as macros.rs leaves them: one declaration per line)
#[derive(Default)]
pub(super) struct StructBodies {
	/// the bodies being read, an anonymous member's inside its struct's
	open: Vec<OpenBody>,
	/// a nested body or a field this reader cannot place: the struct being read gets no layout
	spoiled: bool,
	found: Vec<(String, CTypeDefinition)>,
}

impl StructBodies {
	pub(super) fn read(&mut self, line: &str) {
		let line = line.trim();
		let anonymous_count = self.found.len();
		if let Some((is_union, tag, fields)) = self.open.last_mut() {
			if let Some(after) = line.strip_prefix('}') {
				let alias = after.trim().trim_end_matches(';').trim();
				let definition = CTypeDefinition::Body { is_union: *is_union, fields: std::mem::take(fields) };
				let tag = std::mem::take(tag);
				self.open.pop();
				if let Some((_, _, enclosing_fields)) = self.open.last_mut() {
					let name = format!("{ANONYMOUS_PREFIX}{anonymous_count}");
					enclosing_fields.push((name.clone(), String::new(), None));
					self.found.push((name, definition));
					return;
				}
				let names = [tag, alias.to_string()];
				if !std::mem::take(&mut self.spoiled) {
					self.found.extend(
						names.into_iter().filter(|name| !name.is_empty()).map(|name| (name, definition.clone())),
					);
				}
			} else if line == "struct {" || line == "union {" {
				self.open.push((line.starts_with("union"), String::new(), vec![]));
			} else if line.contains('{') || line.contains('(') {
				self.spoiled = true;
			} else if let Some(field) = line.strip_suffix(';').and_then(declared) {
				fields.push(field);
			} else if !line.is_empty() {
				self.spoiled = true;
			}
			return;
		}
		let words: Vec<&str> = line.split_whitespace().collect();
		let opens = |keyword: &str| {
			let rest = line.strip_prefix("typedef ").unwrap_or(line).strip_prefix(keyword)?;
			let tag = rest.strip_suffix('{')?.trim();
			tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_').then(|| tag.to_string())
		};
		if let Some(tag) = opens("struct ") {
			self.open.push((false, tag, vec![]));
		} else if let Some(tag) = opens("union ") {
			self.open.push((true, tag, vec![]));
		} else if words.first() == Some(&"typedef") && !line.contains('(') && !line.contains('{') {
			let Some((c_type, name, count)) = line["typedef".len()..].trim().strip_suffix(';').and_then(declared)
			else {
				return;
			};
			// `typedef struct wasm_engine_t wasm_engine_t;` names an opaque struct: no alias of itself
			let target = c_type.trim_start_matches("struct ").trim_start_matches("union ").to_string();
			if target != name {
				self.found.push((name, CTypeDefinition::Alias(target, count)));
			}
		}
	}

	/// Keeps what was read for every header read after
	pub(super) fn remember(self) {
		DEFINITIONS
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.get_or_insert_with(Default::default)
			.extend(self.found);
	}
}

/// `uint8_t v128[16]` → ("uint8_t", "v128", Some(16)); `void *__private` → ("void *", "__private", None)
fn declared(declaration: &str) -> Option<(String, String, Option<usize>)> {
	let (declaration, count) = match declaration.trim().strip_suffix(']') {
		Some(before) => {
			let (declaration, count) = before.split_once('[')?;
			(declaration, Some(count.trim().parse().ok()?))
		}
		None => (declaration.trim(), None),
	};
	let (type_words, name) = declaration.rsplit_once(|c: char| c.is_whitespace() || c == '*')?;
	let stars = declaration[type_words.len()..declaration.len() - name.len()].trim();
	let c_type = format!("{} {stars}", type_words.trim()).trim().to_string();
	let is_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
	(is_name && !c_type.is_empty()).then(|| (c_type, name.to_string(), count))
}

/// The layout text of the struct or union `name`, none for a type without a body or with a field of unknown type
pub fn struct_layout(name: &str) -> Option<String> {
	let definitions = DEFINITIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let definitions = definitions.as_ref()?;
	match field_layout(name, definitions, 0)? {
		layout if layout.starts_with(['{', '<']) => Some(layout),
		_ => None,
	}
}

fn field_layout(c_type: &str, definitions: &HashMap<String, CTypeDefinition>, depth: usize) -> Option<String> {
	let c_type = super::bare_c_type(c_type);
	let c_type = if c_type.starts_with(ANONYMOUS_PREFIX) {
		c_type
	} else {
		c_type.trim_start_matches("struct ").trim_start_matches("union ").trim()
	};
	if depth > MAX_NESTING {
		return None;
	}
	if c_type.ends_with('*') {
		return Some("ptr".to_string());
	}
	if c_type.starts_with("enum ") {
		return Some("i32".to_string());
	}
	if let Some((_, layout)) = SCALAR_LAYOUTS.iter().find(|(name, _)| *name == c_type) {
		return Some(layout.to_string());
	}
	match definitions.get(c_type)? {
		CTypeDefinition::Alias(target, None) => field_layout(target, definitions, depth + 1),
		CTypeDefinition::Alias(target, Some(count)) => {
			Some(format!("[{};{count}]", field_layout(target, definitions, depth + 1)?))
		}
		CTypeDefinition::Body { is_union, fields } => {
			let fields: Option<Vec<String>> = fields
				.iter()
				.map(|(field_type, name, count)| {
					let layout = field_layout(field_type, definitions, depth + 1)?;
					Some(match count {
						Some(count) => format!("{name}:[{layout};{count}]"),
						None => format!("{name}:{layout}"),
					})
				})
				.collect();
			let (open, close) = if *is_union { ('<', '>') } else { ('{', '}') };
			Some(format!("{open}{}{close}", fields?.join(",")))
		}
	}
}
