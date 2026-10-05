//! Dead-function elimination of an emitted module (notes/aot.md): the emitter adds runtime functions a program may
//! need (big ints, errors, texts …); every defined function that no export, start function, element segment or
//! global initializer reaches through calls and `ref.func` is dropped, and the rest renumbered. The host finds functions
//! only through exports, so a dropped function is one nothing can call. Imports stay: the linker provides them.
use std::collections::HashMap;
use wasm_encoder::reencode::{utils, Error, Reencode};
use wasmparser::{ConstExpr, ElementItems, ExternalKind, Name, Operator, Parser, Payload, TypeRef};

/// `module` without the functions nothing reaches; the very bytes when every function is live
pub fn without_dead_functions(module: &[u8]) -> Result<Vec<u8>, String> {
	let graph = CallGraph::of(module).map_err(|failure| failure.to_string())?;
	let live = graph.live();
	if live.iter().all(|&is_live| is_live) {
		return Ok(module.to_vec());
	}
	let mut renumbering = Renumbering::new(graph.imported, &live);
	let mut pruned = wasm_encoder::Module::new();
	renumbering.parse_core_module(&mut pruned, Parser::new(0), module).map_err(|failure| format!("{failure:?}"))?;
	Ok(pruned.finish())
}

/// The functions each defined function reaches directly, and the roots
#[derive(Default)]
struct CallGraph {
	imported: u32,
	callees: Vec<Vec<u32>>,
	roots: Vec<u32>,
}

impl CallGraph {
	fn of(module: &[u8]) -> wasmparser::Result<CallGraph> {
		let mut graph = CallGraph::default();
		for payload in Parser::new(0).parse_all(module) {
			match payload? {
				Payload::ImportSection(imports) => {
					for import in imports.into_imports() {
						graph.imported += matches!(import?.ty, TypeRef::Func(_) | TypeRef::FuncExact(_)) as u32;
					}
				}
				Payload::ExportSection(exports) => {
					for export in exports {
						let export = export?;
						if export.kind == ExternalKind::Func {
							graph.roots.push(export.index);
						}
					}
				}
				Payload::StartSection { func, .. } => graph.roots.push(func),
				Payload::ElementSection(elements) => {
					for element in elements {
						match element?.items {
							ElementItems::Functions(functions) => {
								for function in functions {
									graph.roots.push(function?);
								}
							}
							ElementItems::Expressions(_, expressions) => {
								for expression in expressions {
									graph.roots.extend(referenced_functions(&expression?)?);
								}
							}
						}
					}
				}
				Payload::GlobalSection(globals) => {
					for global in globals {
						graph.roots.extend(referenced_functions(&global?.init_expr)?);
					}
				}
				Payload::CodeSectionEntry(body) => {
					let mut callees = Vec::new();
					for operator in body.get_operators_reader()? {
						if let Some(function) = function_operand(&operator?) {
							callees.push(function);
						}
					}
					graph.callees.push(callees);
				}
				_ => {}
			}
		}
		Ok(graph)
	}

	/// Per defined function: whether a root reaches it
	fn live(&self) -> Vec<bool> {
		let mut live = vec![false; self.callees.len()];
		let mut pending = self.roots.clone();
		while let Some(function) = pending.pop() {
			let Some(defined) = function.checked_sub(self.imported) else { continue };
			if !std::mem::replace(&mut live[defined as usize], true) {
				pending.extend(&self.callees[defined as usize]);
			}
		}
		live
	}
}

/// The function an instruction calls or takes a reference to
fn function_operand(operator: &Operator) -> Option<u32> {
	match operator {
		Operator::Call { function_index } | Operator::ReturnCall { function_index } | Operator::RefFunc { function_index } => Some(*function_index),
		_ => None,
	}
}

fn referenced_functions(expression: &ConstExpr) -> wasmparser::Result<Vec<u32>> {
	let mut functions = Vec::new();
	for operator in expression.get_operators_reader() {
		functions.extend(function_operand(&operator?));
	}
	Ok(functions)
}

/// Re-encodes a module without its dead functions: every function index maps to the index of its live function
struct Renumbering {
	/// old index → new index, for imports and live functions
	new_index: HashMap<u32, u32>,
	live: Vec<bool>,
}

impl Renumbering {
	fn new(imported: u32, live: &[bool]) -> Renumbering {
		let mut new_index: HashMap<u32, u32> = (0..imported).map(|index| (index, index)).collect();
		let live_indices = live.iter().enumerate().filter(|(_, &is_live)| is_live).map(|(defined, _)| imported + defined as u32);
		new_index.extend(live_indices.zip(imported..));
		Renumbering { new_index, live: live.to_vec() }
	}

	fn is_live(&self, defined: usize) -> bool {
		self.live[defined]
	}

	/// Names of dropped functions are dropped with them
	fn live_names(&self, map: wasmparser::NameMap) -> Result<wasm_encoder::NameMap, Error<String>> {
		let mut names = wasm_encoder::NameMap::new();
		for naming in map {
			let naming = naming?;
			if let Some(&index) = self.new_index.get(&naming.index) {
				names.append(index, naming.name);
			}
		}
		Ok(names)
	}

	fn live_indirect_names(&self, map: wasmparser::IndirectNameMap) -> Result<wasm_encoder::IndirectNameMap, Error<String>> {
		let mut names = wasm_encoder::IndirectNameMap::new();
		for naming in map {
			let naming = naming?;
			if let Some(&index) = self.new_index.get(&naming.index) {
				names.append(index, &utils::name_map(naming.names, Ok)?);
			}
		}
		Ok(names)
	}
}

impl Reencode for Renumbering {
	type Error = String;

	fn function_index(&mut self, function: u32) -> Result<u32, Error<String>> {
		self.new_index.get(&function).copied().ok_or_else(|| Error::UserError(format!("dead function {function} is still referenced")))
	}

	fn parse_function_section(&mut self, functions: &mut wasm_encoder::FunctionSection, section: wasmparser::FunctionSectionReader<'_>) -> Result<(), Error<String>> {
		for (defined, function_type) in section.into_iter().enumerate() {
			let function_type = function_type?;
			if self.is_live(defined) {
				functions.function(self.type_index(function_type)?);
			}
		}
		Ok(())
	}

	fn parse_code_section(&mut self, code: &mut wasm_encoder::CodeSection, section: wasmparser::CodeSectionReader<'_>) -> Result<(), Error<String>> {
		for (defined, body) in section.into_iter().enumerate() {
			let body = body?;
			if self.is_live(defined) {
				self.parse_function_body(code, body)?;
			}
		}
		Ok(())
	}

	fn parse_custom_name_subsection(&mut self, names: &mut wasm_encoder::NameSection, section: Name<'_>) -> Result<(), Error<String>> {
		match section {
			Name::Function(map) => names.functions(&self.live_names(map)?),
			Name::Local(map) => names.locals(&self.live_indirect_names(map)?),
			Name::Label(map) => names.labels(&self.live_indirect_names(map)?),
			other => utils::parse_custom_name_subsection(self, names, other)?,
		}
		Ok(())
	}
}
