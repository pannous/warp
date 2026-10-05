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
use wasmtime::component::{Component, ComponentExportIndex, Func, Instance, Linker, ResourceAny, ResourceTable, ResourceType, Type, Val};
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

/// A loaded component: its instance, the store it lives in, the resources it handed out
struct Loaded {
	store: Store<ComponentState>,
	instance: Instance,
	/// every function it exports, by name (`fib`, `stats-of`, `[constructor]counter`, `[method]counter.increment`)
	functions: Vec<(String, ComponentExportIndex)>,
	/// its resource types with their WIT names
	kinds: Vec<(ResourceType, String)>,
	held: Vec<ResourceAny>,
}

/// The components loaded in this process, by path
static LOADED: Mutex<Option<HashMap<String, Loaded>>> = Mutex::new(None);

/// A handle's id and the path of its component, keys of the record warp holds
const HANDLE_KEY: &str = "$handle";
const COMPONENT_KEY: &str = "component";

fn engine() -> &'static Engine {
	static ENGINE: OnceLock<Engine> = OnceLock::new();
	ENGINE.get_or_init(|| {
		let mut config = Config::new();
		config.wasm_component_model(true);
		Engine::new(&config).expect("an engine for components")
	})
}

/// `member(arguments)` of the component at `path`: a function, a resource's constructor (`counter(5)` is
/// `[constructor]counter`) or static function; JSON in, JSON out
pub fn call(path: &str, member: &str, arguments: &[Value]) -> Result<Value, String> {
	with_component(path, |loaded| {
		let function = loaded.function(member, None)?;
		loaded.invoke(path, member, function, None, arguments)
	})
}

/// `handle.member(arguments)`: the method of the resource a handle stands for
pub fn call_method(handle: &Value, member: &str, arguments: &[Value]) -> Result<Value, String> {
	let path = handle.get(COMPONENT_KEY).and_then(Value::as_str).ok_or_else(|| format!("{handle} is no handle of a component's resource"))?;
	let kind = handle.get("type").and_then(Value::as_str).unwrap_or_default().to_string();
	with_component(path, |loaded| {
		let function = loaded.function(member, Some(&kind))?;
		loaded.invoke(path, member, function, Some(handle), arguments)
	})
}

fn with_component<R>(path: &str, body: impl FnOnce(&mut Loaded) -> Result<R, String>) -> Result<R, String> {
	let mut loaded = LOADED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let components = loaded.get_or_insert_with(HashMap::new);
	if !components.contains_key(path) {
		components.insert(path.to_string(), load(path)?);
	}
	body(components.get_mut(path).expect("loaded"))
}

fn load(path: &str) -> Result<Loaded, String> {
	let component = Component::from_file(engine(), path).map_err(|failure| format!("cannot load the component {path}: {failure:#}"))?;
	let mut linker = Linker::new(engine());
	wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|failure| failure.to_string())?;
	let state = ComponentState { wasi: WasiCtx::builder().inherit_stdio().build(), table: ResourceTable::new() };
	let mut store = Store::new(engine(), state);
	let instance = linker.instantiate(&mut store, &component).map_err(|failure| format!("cannot instantiate {path}: {failure:#}"))?;
	let (mut functions, mut kinds) = (vec![], vec![]);
	let mut add = |store: &mut Store<ComponentState>, name: &str, item: &ComponentItem, index: Option<ComponentExportIndex>| {
		let Some(index) = index else { return };
		match item {
			ComponentItem::ComponentFunc(_) => functions.push((name.to_string(), index)),
			ComponentItem::Resource(_) => kinds.extend(instance.get_resource(&mut *store, &index).map(|ty| (ty, name.to_string()))),
			_ => {}
		}
	};
	for (name, item) in component.component_type().exports(engine()) {
		if let ComponentItem::ComponentInstance(interface) = &item.ty {
			let Some(interface_index) = component.get_export_index(None, name) else { continue };
			for (inner, item) in interface.exports(engine()) {
				add(&mut store, inner, &item.ty, component.get_export_index(Some(&interface_index), inner));
			}
		} else {
			add(&mut store, name, &item.ty, component.get_export_index(None, name));
		}
	}
	Ok(Loaded { store, instance, functions, kinds, held: vec![] })
}

impl Loaded {
	/// The function `member` names: a method of the resource `kind` for a handle, else a function, a constructor or a
	/// static function; WIT names are kebab-case, so `stats_of` finds `stats-of` and `Counter` `counter`
	fn function(&mut self, member: &str, kind: Option<&str>) -> Result<Func, String> {
		let found = self.functions.iter().find(|(name, _)| match (kind, name.strip_prefix('[')) {
			(Some(kind), Some(bracketed)) => bracketed.strip_prefix("method]").and_then(|rest| rest.split_once('.'))
				.is_some_and(|(resource, method)| resource == kind && names_match(method, member)),
			(Some(_), None) => false,
			(None, None) => names_match(name, member),
			(None, Some(bracketed)) => match bracketed.split_once(']') {
				Some(("constructor", resource)) => names_match(resource, &member.to_lowercase()),
				Some(("static", function)) => function.split_once('.').is_some_and(|(_, function)| names_match(function, member)),
				_ => false,
			},
		});
		let Some((_, index)) = found else {
			let known: Vec<&str> = self.functions.iter().map(|(name, _)| name.as_str()).filter(|name| kind.is_none_or(|kind| name.contains(&format!("]{kind}.")))).collect();
			return Err(match kind {
				None => format!("the component exports no function {member}; it exports {}", known.join(", ")),
				Some(kind) => format!("a {kind} has no method {member}; it has {}", known.join(", ")),
			});
		};
		self.instance.get_func(&mut self.store, index).ok_or_else(|| format!("{member} is no function"))
	}

	/// Call `function` with the receiver's resource first (a method) and the arguments as its WIT parameters want them
	fn invoke(&mut self, path: &str, member: &str, function: Func, receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, String> {
		let signature = function.ty(&self.store);
		let parameters: Vec<(String, Type)> = signature.params().map(|(name, ty)| (name.to_string(), ty)).skip(receiver.is_some() as usize).collect();
		if parameters.len() != arguments.len() {
			let names: Vec<&str> = parameters.iter().map(|(name, _)| name.as_str()).collect();
			return Err(format!("{member} takes {} arguments ({}), got {}", parameters.len(), names.join(", "), arguments.len()));
		}
		let handles = Handles { path, held: &mut self.held, kinds: &self.kinds };
		let mut values = vec![];
		if let (Some(receiver), Some((_, self_type))) = (receiver, signature.params().next()) {
			values.push(handles.value_of(receiver, &self_type)?);
		}
		for ((name, ty), argument) in parameters.iter().zip(arguments) {
			values.push(handles.value_of(argument, ty).map_err(|failure| format!("{member} {name}: {failure}"))?);
		}
		let mut results = vec![Val::Bool(false); signature.results().len()];
		function.call(&mut self.store, &values, &mut results).map_err(|failure| format!("{member}: {failure:#}"))?;
		let mut handles = Handles { path, held: &mut self.held, kinds: &self.kinds };
		match results.as_slice() {
			[] => Ok(Value::Null),
			[result] => handles.json_of(result),
			several => several.iter().map(|result| handles.json_of(result)).collect::<Result<Vec<Value>, String>>().map(Value::Array),
		}
	}
}

fn names_match(exported: &str, member: &str) -> bool {
	exported == member || exported.replace('-', "_") == member
}

/// The resources of a component its results handed out, by handle (id 1 is the first): a resource stays in the
/// component, warp holds `{$handle: id, type: "counter", text: "counter#1", component: path}` (the shared handle rules:
/// ids never addresses, one table per runtime, kept until the process ends)
struct Handles<'a> {
	path: &'a str,
	held: &'a mut Vec<ResourceAny>,
	kinds: &'a [(ResourceType, String)],
}

impl Handles<'_> {
	/// A JSON argument as the WIT type `ty`
	fn value_of(&self, argument: &Value, ty: &Type) -> Result<Val, String> {
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
			Type::List(list) => Val::List(items(argument)?.iter().map(|item| self.value_of(item, &list.ty())).collect::<Result<_, _>>()?),
			Type::Tuple(tuple) => {
				let items = items(argument)?;
				if items.len() != tuple.types().len() {
					return Err(format!("{argument} has {} items, the tuple {}", items.len(), tuple.types().len()));
				}
				Val::Tuple(items.iter().zip(tuple.types()).map(|(item, ty)| self.value_of(item, &ty)).collect::<Result<_, _>>()?)
			}
			Type::Record(record) => {
				let fields = argument.as_object().ok_or_else(|| format!("{argument} is no object of the record's fields"))?;
				Val::Record(record.fields().map(|field| {
					let value = fields.get(field.name).ok_or_else(|| format!("{argument} has no field {}", field.name))?;
					Ok((field.name.to_string(), self.value_of(value, &field.ty)?))
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
					(Some(ty), Some(payload)) => Some(Box::new(self.value_of(payload, &ty)?)),
					(None, _) => None,
					(Some(_), None) => return Err(format!("the case {name} takes a value")),
				};
				Val::Variant(name.to_string(), payload)
			}
			Type::Option(option) => match argument {
				Value::Null => Val::Option(None),
				present => Val::Option(Some(Box::new(self.value_of(present, &option.ty())?))),
			},
			Type::Flags(flags) => {
				let names: Vec<String> = items(argument)?.iter().map(|name| name.as_str().map(str::to_string).ok_or_else(|| format!("{name} is no flag name"))).collect::<Result<_, _>>()?;
				if let Some(unknown) = names.iter().find(|name| !flags.names().any(|flag| flag == name.as_str())) {
					return Err(format!("{unknown} is none of the flags {}", flags.names().collect::<Vec<_>>().join(", ")));
				}
				Val::Flags(names)
			}
			// a handle the component gave: its resource again
			Type::Own(resource) | Type::Borrow(resource) => Val::Resource(self.held_resource(argument, resource)?),
			other => return Err(format!("arguments of the WIT type {other:?} are not supported yet")),
		})
	}


	/// A component's value as JSON; the `err` of a result is the failure
	fn json_of(&mut self, value: &Val) -> Result<Value, String> {
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
			Val::List(items) | Val::Tuple(items) | Val::FixedLengthList(items) => Value::Array(items.iter().map(|item| self.json_of(item)).collect::<Result<_, _>>()?),
			Val::Record(fields) => Value::Object(fields.iter().map(|(name, value)| Ok((name.clone(), self.json_of(value)?))).collect::<Result<Map<_, _>, String>>()?),
			Val::Enum(name) => Value::String(name.clone()),
			Val::Variant(name, None) => Value::String(name.clone()),
			Val::Variant(name, Some(payload)) => Value::Object(Map::from_iter([(name.clone(), self.json_of(payload)?)])),
			Val::Option(None) => Value::Null,
			Val::Option(Some(present)) => self.json_of(present)?,
			Val::Result(Ok(value)) => value.as_deref().map(|item| self.json_of(item)).transpose()?.unwrap_or(Value::Null),
			Val::Result(Err(failure)) => {
				let reason = failure.as_deref().map(|item| self.json_of(item)).transpose()?;
				return Err(match reason {
					Some(Value::String(reason)) => reason,
					Some(reason) => reason.to_string(),
					None => "the component's call failed".to_string(),
				});
			}
			Val::Flags(names) => Value::Array(names.iter().cloned().map(Value::String).collect()),
			Val::Map(entries) => Value::Array(entries.iter().map(|(key, value)| Ok(Value::Array(vec![self.json_of(key)?, self.json_of(value)?]))).collect::<Result<_, String>>()?),
			Val::Resource(resource) => self.handle(*resource),
			other => return Err(format!("results of the kind {other:?} are not supported yet")),
		})
	}

	/// The resource of a handle, of the WIT resource type the parameter wants
	fn held_resource(&self, argument: &Value, wanted: &ResourceType) -> Result<ResourceAny, String> {
		let id = argument.get(HANDLE_KEY).and_then(Value::as_u64).ok_or_else(|| format!("{argument} is no handle of a resource"))?;
		if argument.get(COMPONENT_KEY).and_then(Value::as_str) != Some(self.path) {
			return Err(format!("{argument} is a handle of another component"));
		}
		let resource = *self.held.get(id as usize - 1).ok_or_else(|| format!("no handle {id}"))?;
		if resource.ty() != *wanted {
			return Err(format!("{argument} is a {}, the parameter another resource", self.kind_of(&resource)));
		}
		Ok(resource)
	}

	fn handle(&mut self, resource: ResourceAny) -> Value {
		self.held.push(resource);
		let (id, kind) = (self.held.len(), self.kind_of(&resource));
		serde_json::json!({ HANDLE_KEY: id, "type": kind, "text": format!("{kind}#{id}"), COMPONENT_KEY: self.path })
	}

	fn kind_of(&self, resource: &ResourceAny) -> String {
		self.kinds.iter().find(|(ty, _)| *ty == resource.ty()).map_or_else(|| "resource".to_string(), |(_, name)| name.clone())
	}
}

fn items(argument: &Value) -> Result<&Vec<Value>, String> {
	argument.as_array().ok_or_else(|| format!("{argument} is no list"))
}
