//! `use wasm "lib.wasm" as lib; lib.f(x)`: a WebAssembly component (a Rust crate built for wasm32-wasip2 with
//! wit-bindgen, componentize-py's output, anything with a WIT interface) called through foreign_call
//! (notes/stdlib_connectors.md). Each component is loaded once per path and process, on an engine of its own with the
//! component model (the programs' engines run core modules only), with WASI p2 for its imports. Values cross as JSON,
//! like the other foreign runtimes: a Node argument becomes the WIT type the function declares, its result comes back as
//! JSON (records objects, variants `{case: payload}`, enums and chars texts, an `err` of a result an error).
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Func, Instance, Linker, ResourceTable, Type, Val};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

/// The state of a component's store: its WASI context and resources
struct ComponentState {
	wasi: WasiCtx,
	table: ResourceTable,
}

impl WasiView for ComponentState {
	fn ctx(&mut self) -> WasiCtxView<'_> {
		WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
	}
}

/// A loaded component: its instance and the store it lives in
struct Loaded {
	store: Store<ComponentState>,
	instance: Instance,
	component: Component,
}

/// The components loaded in this process, by path
static LOADED: Mutex<Option<HashMap<String, Loaded>>> = Mutex::new(None);

fn engine() -> &'static Engine {
	static ENGINE: OnceLock<Engine> = OnceLock::new();
	ENGINE.get_or_init(|| {
		let mut config = Config::new();
		config.wasm_component_model(true);
		Engine::new(&config).expect("an engine for components")
	})
}

/// `path.member(arguments)` of the component at `path`; JSON in, JSON out
pub fn call(path: &str, member: &str, arguments: &[Value]) -> Result<Value, String> {
	let mut loaded = LOADED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let components = loaded.get_or_insert_with(HashMap::new);
	if !components.contains_key(path) {
		components.insert(path.to_string(), load(path)?);
	}
	let component = components.get_mut(path).expect("loaded");
	let function = exported_function(component, member)?;
	let signature = function.ty(&component.store);
	let parameters: Vec<(String, Type)> = signature.params().map(|(name, ty)| (name.to_string(), ty)).collect();
	if parameters.len() != arguments.len() {
		let names: Vec<&str> = parameters.iter().map(|(name, _)| name.as_str()).collect();
		return Err(format!("{member} takes {} arguments ({}), got {}", parameters.len(), names.join(", "), arguments.len()));
	}
	let values = parameters.iter().zip(arguments)
		.map(|((name, ty), argument)| value_of(argument, ty).map_err(|failure| format!("{member} {name}: {failure}")))
		.collect::<Result<Vec<Val>, String>>()?;
	let mut results = vec![Val::Bool(false); signature.results().len()];
	function.call(&mut component.store, &values, &mut results).map_err(|failure| format!("{member}: {failure:#}"))?;
	match results.as_slice() {
		[] => Ok(Value::Null),
		[result] => json_of(result),
		several => several.iter().map(json_of).collect::<Result<Vec<Value>, String>>().map(Value::Array),
	}
}

fn load(path: &str) -> Result<Loaded, String> {
	let component = Component::from_file(engine(), path).map_err(|failure| format!("cannot load the component {path}: {failure:#}"))?;
	let mut linker = Linker::new(engine());
	wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|failure| failure.to_string())?;
	let state = ComponentState { wasi: WasiCtx::builder().inherit_stdio().build(), table: ResourceTable::new() };
	let mut store = Store::new(engine(), state);
	let instance = linker.instantiate(&mut store, &component).map_err(|failure| format!("cannot instantiate {path}: {failure:#}"))?;
	Ok(Loaded { store, instance, component })
}

/// The function `member` the component exports, at its top level or in one of the interfaces it exports; WIT names are
/// kebab-case, so `stats_of` finds `stats-of`
fn exported_function(loaded: &mut Loaded, member: &str) -> Result<Func, String> {
	let mut known = vec![];
	let mut found = None;
	for (name, item) in loaded.component.component_type().exports(engine()) {
		match item.ty {
			ComponentItem::ComponentFunc(_) if found.is_none() && names_match(name, member) => found = loaded.component.get_export_index(None, name),
			ComponentItem::ComponentFunc(_) => known.push(name.to_string()),
			ComponentItem::ComponentInstance(interface) => {
				let Some(interface_index) = loaded.component.get_export_index(None, name) else { continue };
				for (function, item) in interface.exports(engine()) {
					match item.ty {
						ComponentItem::ComponentFunc(_) if found.is_none() && names_match(function, member) => {
							found = loaded.component.get_export_index(Some(&interface_index), function);
						}
						ComponentItem::ComponentFunc(_) => known.push(function.to_string()),
						_ => {}
					}
				}
			}
			_ => {}
		}
	}
	let index = found.ok_or_else(|| format!("the component exports no function {member}; it exports {}", known.join(", ")))?;
	loaded.instance.get_func(&mut loaded.store, index).ok_or_else(|| format!("{member} is no function"))
}

fn names_match(exported: &str, member: &str) -> bool {
	exported == member || exported.replace('-', "_") == member
}

/// A JSON argument as the WIT type `ty`
fn value_of(argument: &Value, ty: &Type) -> Result<Val, String> {
	let integer = || argument.as_i64().ok_or_else(|| format!("{argument} is no integer"));
	let unsigned = || argument.as_u64().ok_or_else(|| format!("{argument} is no natural number"));
	let fits = |what: &str| format!("{argument} does not fit a {what}");
	Ok(match ty {
		Type::Bool => Val::Bool(argument.as_bool().or_else(|| argument.as_i64().map(|number| number != 0)).ok_or_else(|| format!("{argument} is no boolean"))?),
		Type::S8 => Val::S8(integer()?.try_into().map_err(|_| fits("s8"))?),
		Type::S16 => Val::S16(integer()?.try_into().map_err(|_| fits("s16"))?),
		Type::S32 => Val::S32(integer()?.try_into().map_err(|_| fits("s32"))?),
		Type::S64 => Val::S64(integer()?),
		Type::U8 => Val::U8(unsigned()?.try_into().map_err(|_| fits("u8"))?),
		Type::U16 => Val::U16(unsigned()?.try_into().map_err(|_| fits("u16"))?),
		Type::U32 => Val::U32(unsigned()?.try_into().map_err(|_| fits("u32"))?),
		Type::U64 => Val::U64(unsigned()?),
		Type::Float32 => Val::Float32(argument.as_f64().ok_or_else(|| format!("{argument} is no number"))? as f32),
		Type::Float64 => Val::Float64(argument.as_f64().ok_or_else(|| format!("{argument} is no number"))?),
		Type::Char => {
			let text = argument.as_str().ok_or_else(|| format!("{argument} is no character"))?;
			let mut characters = text.chars();
			match (characters.next(), characters.next()) {
				(Some(character), None) => Val::Char(character),
				_ => return Err(format!("{argument} is no single character")),
			}
		}
		Type::String => Val::String(argument.as_str().ok_or_else(|| format!("{argument} is no text"))?.to_string()),
		Type::List(list) => Val::List(items(argument)?.iter().map(|item| value_of(item, &list.ty())).collect::<Result<_, _>>()?),
		Type::Tuple(tuple) => {
			let items = items(argument)?;
			if items.len() != tuple.types().len() {
				return Err(format!("{argument} has {} items, the tuple {}", items.len(), tuple.types().len()));
			}
			Val::Tuple(items.iter().zip(tuple.types()).map(|(item, ty)| value_of(item, &ty)).collect::<Result<_, _>>()?)
		}
		Type::Record(record) => {
			let fields = argument.as_object().ok_or_else(|| format!("{argument} is no object of the record's fields"))?;
			Val::Record(record.fields().map(|field| {
				let value = fields.get(field.name).ok_or_else(|| format!("{argument} has no field {}", field.name))?;
				Ok((field.name.to_string(), value_of(value, &field.ty)?))
			}).collect::<Result<_, String>>()?)
		}
		Type::Enum(cases) => {
			let name = argument.as_str().ok_or_else(|| format!("{argument} is no case name"))?;
			if !cases.names().any(|case| case == name) {
				return Err(format!("{name} is none of {}", cases.names().collect::<Vec<_>>().join(", ")));
			}
			Val::Enum(name.to_string())
		}
		Type::Variant(variant) => {
			let (name, payload) = match argument {
				Value::String(name) => (name.as_str(), None),
				Value::Object(entries) if entries.len() == 1 => entries.iter().next().map(|(name, payload)| (name.as_str(), Some(payload))).expect("one"),
				_ => return Err(format!("{argument} is no case: a name or {{case: value}}")),
			};
			let case = variant.cases().find(|case| case.name == name).ok_or_else(|| format!("{name} is no case of the variant"))?;
			let payload = match (case.ty, payload) {
				(Some(ty), Some(payload)) => Some(Box::new(value_of(payload, &ty)?)),
				(None, _) => None,
				(Some(_), None) => return Err(format!("the case {name} takes a value")),
			};
			Val::Variant(name.to_string(), payload)
		}
		Type::Option(option) => match argument {
			Value::Null => Val::Option(None),
			present => Val::Option(Some(Box::new(value_of(present, &option.ty())?))),
		},
		Type::Flags(flags) => {
			let names: Vec<String> = items(argument)?.iter().map(|name| name.as_str().map(str::to_string).ok_or_else(|| format!("{name} is no flag name"))).collect::<Result<_, _>>()?;
			if let Some(unknown) = names.iter().find(|name| !flags.names().any(|flag| flag == name.as_str())) {
				return Err(format!("{unknown} is none of the flags {}", flags.names().collect::<Vec<_>>().join(", ")));
			}
			Val::Flags(names)
		}
		other => return Err(format!("arguments of the WIT type {other:?} are not supported yet")),
	})
}

fn items(argument: &Value) -> Result<&Vec<Value>, String> {
	argument.as_array().ok_or_else(|| format!("{argument} is no list"))
}

/// A component's value as JSON; the `err` of a result is the failure
fn json_of(value: &Val) -> Result<Value, String> {
	Ok(match value {
		Val::Bool(truth) => Value::Bool(*truth),
		Val::S8(number) => Value::from(*number),
		Val::U8(number) => Value::from(*number),
		Val::S16(number) => Value::from(*number),
		Val::U16(number) => Value::from(*number),
		Val::S32(number) => Value::from(*number),
		Val::U32(number) => Value::from(*number),
		Val::S64(number) => Value::from(*number),
		Val::U64(number) => Value::from(*number),
		Val::Float32(number) => Value::from(f64::from(*number)),
		Val::Float64(number) => Value::from(*number),
		Val::Char(character) => Value::String(character.to_string()),
		Val::String(text) => Value::String(text.clone()),
		Val::List(items) | Val::Tuple(items) | Val::FixedLengthList(items) => Value::Array(items.iter().map(json_of).collect::<Result<_, _>>()?),
		Val::Record(fields) => Value::Object(fields.iter().map(|(name, value)| Ok((name.clone(), json_of(value)?))).collect::<Result<Map<_, _>, String>>()?),
		Val::Enum(name) => Value::String(name.clone()),
		Val::Variant(name, None) => Value::String(name.clone()),
		Val::Variant(name, Some(payload)) => Value::Object(Map::from_iter([(name.clone(), json_of(payload)?)])),
		Val::Option(None) => Value::Null,
		Val::Option(Some(present)) => json_of(present)?,
		Val::Result(Ok(value)) => value.as_deref().map(json_of).transpose()?.unwrap_or(Value::Null),
		Val::Result(Err(failure)) => {
			let reason = failure.as_deref().map(json_of).transpose()?;
			return Err(match reason {
				Some(Value::String(reason)) => reason,
				Some(reason) => reason.to_string(),
				None => "the component's call failed".to_string(),
			});
		}
		Val::Flags(names) => Value::Array(names.iter().cloned().map(Value::String).collect()),
		Val::Map(entries) => Value::Array(entries.iter().map(|(key, value)| Ok(Value::Array(vec![json_of(key)?, json_of(value)?]))).collect::<Result<_, String>>()?),
		other => return Err(format!("results of the kind {other:?} are not supported yet (resources need handles)")),
	})
}
