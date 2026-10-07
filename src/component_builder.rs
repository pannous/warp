//! `warp build --component app.wasp` (card wasm-interop-rest, component worlds step 2): the program compiled to a
//! WebAssembly component of the world its `component` declaration names (component_worlds.rs). Each function a world
//! export lists is the program's `export def` of that name, reached through an adapter of the canonical ABI: `api#add`
//! takes and gives the core types of its WIT signature (s32 as i32) and converts them to the program's (i64). The
//! module, its WIT embedded, becomes the component through wit-component. Scalars only so far; texts are step 3.
use crate::component_worlds::{Signature, World, EXPORT_DIRECTION};
use wasm_encoder::reencode::{utils, Error, Reencode};
use wasm_encoder::{Instruction, ValType};
use wasmparser::{CompositeInnerType, ExternalKind, Parser, Payload, TypeRef};

/// The core type of a WIT scalar, and whether a narrower integer widens with its sign
const CORE_TYPES: [(&str, ValType, bool); 12] = [
	("s8", ValType::I32, true), ("s16", ValType::I32, true), ("s32", ValType::I32, true), ("u8", ValType::I32, false),
	("u16", ValType::I32, false), ("u32", ValType::I32, false), ("bool", ValType::I32, false), ("char", ValType::I32, false),
	("s64", ValType::I64, true), ("u64", ValType::I64, false), ("f32", ValType::F32, true), ("f64", ValType::F64, true),
];
const WIT_PATH: &str = "world.wit";

/// The component of the program `code`
pub fn build(code: &str) -> Result<Vec<u8>, String> {
	let program = crate::wasp_parser::parse(code);
	let world = crate::component_worlds::world(&program)?;
	let wit = crate::component_worlds::world_wit(&program)?;
	let module = crate::pipeline::compile(code).map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
	let mut bytes = with_adapters(&module.bytes, &world)?;
	let failure = |error: anyhow::Error| format!("{error:#}");
	let mut resolve = wit_parser::Resolve::default();
	let package = resolve.push_str(WIT_PATH, &wit).map_err(failure)?;
	let world_id = resolve.select_world(&[package], None).map_err(failure)?;
	wit_component::embed_component_metadata(&mut bytes, &resolve, world_id, wit_component::StringEncoding::UTF8).map_err(failure)?;
	wit_component::ComponentEncoder::default().validate(true).module(&bytes).map_err(failure)?.encode().map_err(failure)
}

/// An adapter: the export it is (`api#add`), the program's function it calls, its core signature and conversions
struct Adapter {
	export: String,
	target: u32,
	parameters: Vec<(ValType, bool)>,
	result: Option<(ValType, bool)>,
	target_parameters: Vec<ValType>,
	target_result: Option<ValType>,
}

/// What the adapters need of a module: its type count, the type of each function, its function exports
#[derive(Default)]
struct ModuleShape {
	types: Vec<Option<wasmparser::FuncType>>,
	function_types: Vec<u32>,
	exports: Vec<(String, u32)>,
	defined: u32,
}

impl ModuleShape {
	fn of(module: &[u8]) -> wasmparser::Result<ModuleShape> {
		let mut shape = ModuleShape::default();
		for payload in Parser::new(0).parse_all(module) {
			match payload? {
				Payload::TypeSection(section) => for group in section {
					shape.types.extend(group?.into_types().map(|sub| match sub.composite_type.inner {
						CompositeInnerType::Func(function) => Some(function),
						_ => None,
					}));
				},
				Payload::ImportSection(imports) => for import in imports.into_imports() {
					if let TypeRef::Func(index) | TypeRef::FuncExact(index) = import?.ty {
						shape.function_types.push(index);
					}
				},
				Payload::FunctionSection(functions) => for function in functions {
					shape.function_types.push(function?);
					shape.defined += 1;
				},
				Payload::ExportSection(exports) => for export in exports {
					let export = export?;
					if export.kind == ExternalKind::Func {
						shape.exports.push((export.name.to_string(), export.index));
					}
				},
				_ => {}
			}
		}
		Ok(shape)
	}

	fn function_type(&self, function: u32) -> Option<&wasmparser::FuncType> {
		self.types.get(*self.function_types.get(function as usize)? as usize)?.as_ref()
	}
}

/// The module with an adapter export for each function the world exports
fn with_adapters(module: &[u8], world: &World) -> Result<Vec<u8>, String> {
	let shape = ModuleShape::of(module).map_err(|failure| failure.to_string())?;
	let shape_of = &shape;
	let adapters = world.items.iter().filter(|item| item.direction == EXPORT_DIRECTION)
		.flat_map(|item| item.functions.iter().map(move |function| adapter(shape_of, &item.name, function)))
		.collect::<Result<Vec<Adapter>, String>>()?;
	let mut adapting = Adapting { adapters, first_type: shape.types.len() as u32, first_function: shape.function_types.len() as u32 };
	let mut adapted = wasm_encoder::Module::new();
	adapting.parse_core_module(&mut adapted, Parser::new(0), module).map_err(|failure| format!("{failure:?}"))?;
	Ok(adapted.finish())
}

fn adapter(shape: &ModuleShape, interface: &str, function: &Signature) -> Result<Adapter, String> {
	let name = &function.name;
	let target = shape.exports.iter().find(|(exported, _)| exported == name).map(|(_, index)| *index)
		.ok_or_else(|| format!("export {interface}: the program has no `export def {name}`"))?;
	let target_type = shape.function_type(target).ok_or_else(|| format!("{name}: no function type"))?;
	let core = |wit: &String| CORE_TYPES.iter().find(|(scalar, _, _)| scalar == wit).map(|(_, core, signed)| (*core, *signed))
		.ok_or_else(|| format!("{name}: {wit} crosses the component boundary in a later step (scalars only so far)"));
	let value_type = |parameter: &wasmparser::ValType| match parameter {
		wasmparser::ValType::I32 => Ok(ValType::I32),
		wasmparser::ValType::I64 => Ok(ValType::I64),
		wasmparser::ValType::F32 => Ok(ValType::F32),
		wasmparser::ValType::F64 => Ok(ValType::F64),
		_ => Err(format!("{name}: declare the types of its parameters and result (`export def {name}(a: i32) -> i32`)")),
	};
	let target_parameters = target_type.params().iter().map(value_type).collect::<Result<Vec<_>, _>>()?;
	if target_parameters.len() != function.parameters.len() {
		return Err(format!("{name} takes {} parameters, its interface says {}", target_parameters.len(), function.parameters.len()));
	}
	Ok(Adapter {
		export: format!("{interface}#{name}"),
		target,
		parameters: function.parameters.iter().map(core).collect::<Result<_, _>>()?,
		result: function.result.as_ref().map(core).transpose()?,
		target_parameters,
		target_result: target_type.results().first().map(value_type).transpose()?,
	})
}

/// `from` as `to`: an integer widens with or without its sign, or narrows; a float is promoted or demoted
fn converted(from: ValType, to: ValType, signed: bool) -> Option<Instruction<'static>> {
	match (from, to) {
		(ValType::I32, ValType::I64) if signed => Some(Instruction::I64ExtendI32S),
		(ValType::I32, ValType::I64) => Some(Instruction::I64ExtendI32U),
		(ValType::I64, ValType::I32) => Some(Instruction::I32WrapI64),
		(ValType::F32, ValType::F64) => Some(Instruction::F64PromoteF32),
		(ValType::F64, ValType::F32) => Some(Instruction::F32DemoteF64),
		_ => None,
	}
}

/// The re-encoding that appends the adapters' types, functions, exports and bodies
struct Adapting {
	adapters: Vec<Adapter>,
	first_type: u32,
	first_function: u32,
}

impl Adapting {
	fn body(adapter: &Adapter) -> wasm_encoder::Function {
		let mut body = wasm_encoder::Function::new([]);
		for (index, ((core, signed), target)) in adapter.parameters.iter().zip(&adapter.target_parameters).enumerate() {
			body.instruction(&Instruction::LocalGet(index as u32));
			if let Some(conversion) = converted(*core, *target, *signed) {
				body.instruction(&conversion);
			}
		}
		body.instruction(&Instruction::Call(adapter.target));
		match (adapter.target_result, adapter.result) {
			(Some(from), Some((to, signed))) => {
				if let Some(conversion) = converted(from, to, signed) {
					body.instruction(&conversion);
				}
			}
			(Some(_), None) => {
				body.instruction(&Instruction::Drop);
			}
			_ => {}
		}
		body.instruction(&Instruction::End);
		body
	}
}

impl Reencode for Adapting {
	type Error = String;

	fn parse_type_section(&mut self, types: &mut wasm_encoder::TypeSection, section: wasmparser::TypeSectionReader<'_>) -> Result<(), Error<String>> {
		utils::parse_type_section(self, types, section)?;
		for adapter in &self.adapters {
			types.ty().function(adapter.parameters.iter().map(|(core, _)| *core), adapter.result.iter().map(|(core, _)| *core));
		}
		Ok(())
	}

	fn parse_function_section(&mut self, functions: &mut wasm_encoder::FunctionSection, section: wasmparser::FunctionSectionReader<'_>) -> Result<(), Error<String>> {
		utils::parse_function_section(self, functions, section)?;
		for index in 0..self.adapters.len() as u32 {
			functions.function(self.first_type + index);
		}
		Ok(())
	}

	fn parse_export_section(&mut self, exports: &mut wasm_encoder::ExportSection, section: wasmparser::ExportSectionReader<'_>) -> Result<(), Error<String>> {
		utils::parse_export_section(self, exports, section)?;
		for (index, adapter) in self.adapters.iter().enumerate() {
			exports.export(&adapter.export, wasm_encoder::ExportKind::Func, self.first_function + index as u32);
		}
		Ok(())
	}

	fn parse_code_section(&mut self, code: &mut wasm_encoder::CodeSection, section: wasmparser::CodeSectionReader<'_>) -> Result<(), Error<String>> {
		utils::parse_code_section(self, code, section)?;
		for adapter in &self.adapters {
			code.function(&Self::body(adapter));
		}
		Ok(())
	}
}
