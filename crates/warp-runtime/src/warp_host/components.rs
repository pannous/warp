//! WebAssembly components a program loads (`load_wasm("c.wasm")`, wasm_modules.rs): each on an engine of its own with
//! the component model on (programs' engines are core-only), in a store of its own kept for the run. Values cross by
//! the declared WIT types (the canonical ABI is wasmtime's):
//! - bool, s8 to u64 (range-checked), f32, f64, char (a codepoint), string (a text)
//! - list and tuple: a list; record: a map `{field: value}`; option: ø or the value; enum: its name;
//!   variant: its name or `{case: value}`; flags: a list of names; result: the ok value, the err an Error
//! - names: `stats_of` finds `stats-of`, `text_tools.words` or plain `words` a function of an exported interface
//! - not yet: resources, WASI p2 (a component's imports trap when called)
use crate::nodes::{self, Node};
use std::sync::OnceLock;
use wasmtime::component::types::{self, ComponentItem};
use wasmtime::component::{Component, ComponentExportIndex, Instance, Linker, Type, Val};
use wasmtime::{Config, Engine, Store};

pub struct Loaded {
	store: Store<()>,
	instance: Instance,
	/// every function it exports: its name, `interface.name` inside an interface, with its WIT type
	functions: Vec<(String, ComponentExportIndex, String)>,
}

/// A component's binary: the preamble's layer 1 (a core module's is 0)
pub fn is_component(bytes: &[u8]) -> bool {
	bytes.len() >= 8 && bytes.starts_with(b"\0asm") && bytes[6..8] == [1, 0]
}

fn engine() -> wasmtime::Result<&'static Engine> {
	static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();
	let engine = ENGINE.get_or_init(|| {
		let mut config = Config::new();
		config.wasm_component_model(true);
		Engine::new(&config).map_err(|problem| format!("{problem:#}"))
	});
	engine.as_ref().map_err(|problem| wasmtime::Error::msg(problem.clone()))
}

pub fn load(bytes: &[u8]) -> wasmtime::Result<Loaded> {
	let engine = engine()?;
	let component = Component::new(engine, bytes)?;
	let mut linker = Linker::<()>::new(engine);
	linker.define_unknown_imports_as_traps(&component)?;
	let mut store = Store::new(engine, ());
	crate::cli::fuel(&mut store);
	let instance = linker.instantiate(&mut store, &component)?;
	let mut functions = Vec::new();
	for (name, item) in component.component_type().exports(engine) {
		match &item.ty {
			ComponentItem::ComponentFunc(function) => {
				if let Some(index) = component.get_export_index(None, name) {
					functions.push((name.to_string(), index, function_type(function)));
				}
			}
			ComponentItem::ComponentInstance(interface) => {
				let Some(outer) = component.get_export_index(None, name) else { continue };
				for (inner, item) in interface.exports(engine) {
					let ComponentItem::ComponentFunc(function) = &item.ty else { continue };
					if let Some(index) = component.get_export_index(Some(&outer), inner) {
						functions.push((format!("{name}.{inner}"), index, function_type(function)));
					}
				}
			}
			_ => {}
		}
	}
	Ok(Loaded { store, instance, functions })
}

/// `warp:demo/text-tools@1.0.0.words` shown as `text-tools.words`
fn short_name(name: &str) -> String {
	match name.rsplit_once('.') {
		Some((interface, inner)) if interface.contains([':', '/', '@']) => {
			let interface = interface.split('@').next().unwrap_or(interface);
			format!("{}.{inner}", interface.rsplit(['/', ':']).next().unwrap_or(interface))
		}
		_ => name.to_string(),
	}
}

/// `stats_of` names `stats-of`; `text_tools.words` and `words` the function of the interface text-tools
fn names_match(exported: &str, wanted: &str) -> bool {
	let plain = |name: &str| name.replace('_', "-");
	let wanted = plain(wanted);
	let exported = plain(exported);
	let short = exported.rsplit(['.', '/', '#']).next().unwrap_or(&exported);
	let interface = exported.rsplit_once('.').map(|(interface, inner)| {
		let interface = interface.rsplit(['/', ':']).next().unwrap_or(interface);
		let interface = interface.split('@').next().unwrap_or(interface);
		format!("{interface}.{inner}")
	});
	exported == wanted || short == wanted || interface.as_deref() == Some(wanted.as_str())
}

impl Loaded {
	/// `{fib: "func(n: u32) -> u64", "text-tools.words": "func(text: string) -> list<string>"}`
	pub fn exports(&self) -> Node {
		Node::named_map(self.functions.iter().map(|(name, _, ty)| (short_name(name), Node::text(ty))).collect())
	}

	pub fn call(&mut self, name: &str, arguments: &[Node]) -> Result<Node, String> {
		let found = self.functions.iter().filter(|(exported, ..)| names_match(exported, name)).collect::<Vec<_>>();
		let (exported, index) = match found.as_slice() {
			[(exported, index, _)] => (exported.clone(), *index),
			[] => {
				let names = self.functions.iter().map(|(name, ..)| name.as_str()).collect::<Vec<_>>().join(", ");
				return Err(format!("the component has no function {name} (it has {names})"));
			}
			several => {
				let names = several.iter().map(|(name, ..)| name.as_str()).collect::<Vec<_>>().join(", ");
				return Err(format!("{name} is ambiguous: {names}"));
			}
		};
		let function = self.instance.get_func(&mut self.store, index).ok_or(format!("{exported} is no function"))?;
		let ty = function.ty(&self.store);
		let params: Vec<(String, Type)> = ty.params().map(|(name, ty)| (name.to_string(), ty)).collect();
		if params.len() != arguments.len() {
			return Err(format!("{exported} takes {} arguments, got {}", params.len(), arguments.len()));
		}
		let given = arguments
			.iter()
			.zip(&params)
			.map(|(argument, (parameter, ty))| {
				value_of(argument, ty).map_err(|problem| format!("{exported}({parameter}): {problem}"))
			})
			.collect::<Result<Vec<_>, _>>()?;
		let mut results = vec![Val::Bool(false); ty.results().len()];
		function.call(&mut self.store, &given, &mut results).map_err(|problem| {
			match problem.downcast_ref::<wasmtime::Trap>() {
				Some(trap) => format!("{exported} failed: {trap}"),
				None => format!("{exported} failed: {problem:#}"),
			}
		})?;
		let mut out = results.iter().map(node_of).collect::<Result<Vec<_>, _>>()?;
		Ok(match out.len() {
			0 => Node::Empty,
			1 => out.pop().unwrap_or(Node::Empty),
			_ => Node::List(nodes::SQUARE_LIST, out),
		})
	}
}

/// A warp value as the WIT type wants it
fn value_of(argument: &Node, ty: &Type) -> Result<Val, String> {
	let shown = || argument.shown();
	let integer = || match argument {
		Node::Int(n) => Ok(*n),
		Node::Bool(truth) => Ok(i64::from(*truth)),
		_ => Err(format!("{} is no integer", shown())),
	};
	let fits = |what: &str| format!("{} does not fit a {what}", shown());
	let number = || match argument {
		Node::Float(x) => Ok(*x),
		Node::Ratio(numerator, denominator) => Ok(*numerator as f64 / *denominator as f64),
		Node::Int(n) => Ok(*n as f64),
		_ => Err(format!("{} is no number", shown())),
	};
	Ok(match ty {
		Type::Bool => Val::Bool(match argument {
			Node::Bool(truth) => *truth,
			Node::Int(n) => *n != 0,
			_ => return Err(format!("{} is no boolean", shown())),
		}),
		Type::S8 => Val::S8(integer()?.try_into().map_err(|_| fits("s8"))?),
		Type::S16 => Val::S16(integer()?.try_into().map_err(|_| fits("s16"))?),
		Type::S32 => Val::S32(integer()?.try_into().map_err(|_| fits("s32"))?),
		Type::S64 => Val::S64(integer()?),
		Type::U8 => Val::U8(integer()?.try_into().map_err(|_| fits("u8"))?),
		Type::U16 => Val::U16(integer()?.try_into().map_err(|_| fits("u16"))?),
		Type::U32 => Val::U32(integer()?.try_into().map_err(|_| fits("u32"))?),
		Type::U64 => Val::U64(integer()?.try_into().map_err(|_| fits("u64"))?),
		Type::Float32 => Val::Float32(number()? as f32),
		Type::Float64 => Val::Float64(number()?),
		Type::Char => Val::Char(match argument {
			Node::Codepoint(code) => char::from_u32(*code).ok_or_else(|| format!("{code} is no character"))?,
			Node::Text(text) | Node::Symbol(text) => {
				let text = String::from_utf8_lossy(text);
				let mut characters = text.chars();
				match (characters.next(), characters.next()) {
					(Some(character), None) => character,
					_ => return Err(format!("{} is no single character", shown())),
				}
			}
			_ => return Err(format!("{} is no character", shown())),
		}),
		Type::String => match argument {
			Node::Text(text) | Node::Symbol(text) => Val::String(String::from_utf8_lossy(text).into_owned()),
			_ => return Err(format!("{} is no text", shown())),
		},
		// a text where bytes are wanted: its UTF-8
		Type::List(list) if matches!(list.ty(), Type::U8) && matches!(argument, Node::Text(_)) => {
			let Node::Text(text) = argument else { unreachable!("a text") };
			Val::List(text.iter().copied().map(Val::U8).collect())
		}
		Type::List(list) => {
			Val::List(items(argument)?.iter().map(|item| value_of(item, &list.ty())).collect::<Result<_, _>>()?)
		}
		Type::Tuple(tuple) => {
			let items = items(argument)?;
			if items.len() != tuple.types().len() {
				return Err(format!("{} has {} items, the tuple {}", shown(), items.len(), tuple.types().len()));
			}
			Val::Tuple(items.iter().zip(tuple.types()).map(|(item, ty)| value_of(item, &ty)).collect::<Result<_, _>>()?)
		}
		Type::Record(record) => Val::Record(
			record
				.fields()
				.map(|field| {
					let value = entry(argument, field.name)
						.ok_or_else(|| format!("{} has no field {}", shown(), field.name))?;
					Ok((field.name.to_string(), value_of(value, &field.ty)?))
				})
				.collect::<Result<_, String>>()?,
		),
		Type::Enum(cases) => {
			let name = case_name(argument).ok_or_else(|| format!("{} is no case name", shown()))?;
			if !cases.names().any(|case| case == name) {
				return Err(format!("{name} is none of {}", cases.names().collect::<Vec<_>>().join(", ")));
			}
			Val::Enum(name)
		}
		Type::Variant(variant) => variant_of(argument, variant)?,
		Type::Option(option) => match argument {
			Node::Empty => Val::Option(None),
			present => Val::Option(Some(Box::new(value_of(present, &option.ty())?))),
		},
		Type::Result(result) => match argument {
			Node::Error(reason) => Val::Result(Err(match result.err() {
				Some(ty) => Some(Box::new(value_of(&Node::Text(reason.clone()), &ty)?)),
				None => None,
			})),
			value => Val::Result(Ok(match result.ok() {
				Some(ty) => Some(Box::new(value_of(value, &ty)?)),
				None => None,
			})),
		},
		Type::Flags(flags) => {
			let names = items(argument)?
				.iter()
				.map(|name| case_name(name).ok_or_else(|| format!("{} is no flag name", name.shown())))
				.collect::<Result<Vec<_>, _>>()?;
			if let Some(unknown) = names.iter().find(|name| !flags.names().any(|flag| flag == name.as_str())) {
				return Err(format!("{unknown} is none of the flags {}", flags.names().collect::<Vec<_>>().join(", ")));
			}
			Val::Flags(names)
		}
		other => return Err(format!("parameters of the WIT type {} are not supported yet", wit(other))),
	})
}

fn variant_of(argument: &Node, variant: &types::Variant) -> Result<Val, String> {
	let (name, payload) = match argument {
		Node::List(_, entries) if entries.len() == 1 => match &entries[0] {
			Node::Key(_, key, value) => (key.content(), Some(value.as_ref())),
			_ => return Err(format!("{} is no case: a name or {{case: value}}", argument.shown())),
		},
		other => {
			(case_name(other).ok_or_else(|| format!("{} is no case: a name or {{case: value}}", other.shown()))?, None)
		}
	};
	let case =
		variant.cases().find(|case| case.name == name).ok_or_else(|| format!("{name} is no case of the variant"))?;
	let payload = match (case.ty, payload) {
		(Some(ty), Some(payload)) => Some(Box::new(value_of(payload, &ty)?)),
		(None, _) => None,
		(Some(_), None) => return Err(format!("the case {name} takes a value")),
	};
	Ok(Val::Variant(name, payload))
}

fn case_name(node: &Node) -> Option<String> {
	match node {
		Node::Text(name) | Node::Symbol(name) => Some(String::from_utf8_lossy(name).replace('_', "-")),
		_ => None,
	}
}

fn items(argument: &Node) -> Result<Vec<Node>, String> {
	match argument {
		Node::List(_, items) => Ok(items.clone()),
		Node::Empty => Ok(vec![]),
		other => Err(format!("{} is no list", other.shown())),
	}
}

/// A map's value of the key `name` (or its snake_case spelling)
fn entry<'a>(map: &'a Node, name: &str) -> Option<&'a Node> {
	let Node::List(_, entries) = map else { return None };
	entries.iter().find_map(|entry| match entry {
		Node::Key(_, key, value) if key.content() == name || key.content().replace('_', "-") == name => {
			Some(value.as_ref())
		}
		_ => None,
	})
}

/// A component's value as warp's; the err of a result is an Error
fn node_of(value: &Val) -> Result<Node, String> {
	Ok(match value {
		Val::Bool(truth) => Node::Bool(*truth),
		Val::S8(n) => Node::Int((*n).into()),
		Val::U8(n) => Node::Int((*n).into()),
		Val::S16(n) => Node::Int((*n).into()),
		Val::U16(n) => Node::Int((*n).into()),
		Val::S32(n) => Node::Int((*n).into()),
		Val::U32(n) => Node::Int((*n).into()),
		Val::S64(n) => Node::Int(*n),
		Val::U64(n) => Node::Int(i64::try_from(*n).map_err(|_| format!("{n} is beyond warp-host's Int"))?),
		Val::Float32(x) => Node::Float(f64::from(*x)),
		Val::Float64(x) => Node::Float(*x),
		Val::Char(character) => Node::Codepoint(u32::from(*character)),
		Val::String(text) => Node::text(text.clone()),
		Val::List(items) | Val::Tuple(items) | Val::FixedLengthList(items) => {
			Node::List(nodes::SQUARE_LIST, items.iter().map(node_of).collect::<Result<_, _>>()?)
		}
		Val::Record(fields) => Node::named_map(
			fields.iter().map(|(name, value)| Ok((name.clone(), node_of(value)?))).collect::<Result<_, String>>()?,
		),
		Val::Enum(name) | Val::Variant(name, None) => Node::text(name.clone()),
		Val::Variant(name, Some(payload)) => Node::named_map(vec![(name.clone(), node_of(payload)?)]),
		Val::Option(None) => Node::Empty,
		Val::Option(Some(value)) => node_of(value)?,
		Val::Result(Ok(value)) => value.as_deref().map(node_of).transpose()?.unwrap_or(Node::Empty),
		Val::Result(Err(reason)) => {
			let reason = reason.as_deref().map(node_of).transpose()?;
			Node::Error(reason.map_or("the component's call failed".into(), |reason| reason.content()).into_bytes())
		}
		Val::Flags(names) => {
			Node::List(nodes::SQUARE_LIST, names.iter().map(|name| Node::text(name.clone())).collect())
		}
		Val::Map(entries) => Node::named_map(
			entries
				.iter()
				.map(|(key, value)| Ok((node_of(key)?.content(), node_of(value)?)))
				.collect::<Result<_, String>>()?,
		),
		other => return Err(format!("results like {other:?} are not supported yet")),
	})
}

/// `func(text: string) -> list<string>`
fn function_type(function: &types::ComponentFunc) -> String {
	let params = function.params().map(|(name, ty)| format!("{name}: {}", wit(&ty))).collect::<Vec<_>>().join(", ");
	let results = function.results().map(|ty| wit(&ty)).collect::<Vec<_>>();
	match results.as_slice() {
		[] => format!("func({params})"),
		[result] => format!("func({params}) -> {result}"),
		several => format!("func({params}) -> ({})", several.join(", ")),
	}
}

fn wit(ty: &Type) -> String {
	let of = |ty: Option<Type>| ty.map_or("_".to_string(), |ty| wit(&ty));
	match ty {
		Type::Bool => "bool".into(),
		Type::S8 => "s8".into(),
		Type::U8 => "u8".into(),
		Type::S16 => "s16".into(),
		Type::U16 => "u16".into(),
		Type::S32 => "s32".into(),
		Type::U32 => "u32".into(),
		Type::S64 => "s64".into(),
		Type::U64 => "u64".into(),
		Type::Float32 => "f32".into(),
		Type::Float64 => "f64".into(),
		Type::Char => "char".into(),
		Type::String => "string".into(),
		Type::List(list) => format!("list<{}>", wit(&list.ty())),
		Type::Tuple(tuple) => format!("tuple<{}>", tuple.types().map(|ty| wit(&ty)).collect::<Vec<_>>().join(", ")),
		Type::Record(record) => {
			format!(
				"record {{ {} }}",
				record
					.fields()
					.map(|field| format!("{}: {}", field.name, wit(&field.ty)))
					.collect::<Vec<_>>()
					.join(", ")
			)
		}
		Type::Enum(cases) => format!("enum {{ {} }}", cases.names().collect::<Vec<_>>().join(", ")),
		Type::Variant(variant) => format!(
			"variant {{ {} }}",
			variant
				.cases()
				.map(|case| match case.ty {
					Some(ty) => format!("{}({})", case.name, wit(&ty)),
					None => case.name.to_string(),
				})
				.collect::<Vec<_>>()
				.join(", ")
		),
		Type::Option(option) => format!("option<{}>", wit(&option.ty())),
		Type::Result(result) => format!("result<{}, {}>", of(result.ok()), of(result.err())),
		Type::Flags(flags) => format!("flags {{ {} }}", flags.names().collect::<Vec<_>>().join(", ")),
		Type::Own(_) => "own<resource>".into(),
		Type::Borrow(_) => "borrow<resource>".into(),
		other => format!("{other:?}"),
	}
}
