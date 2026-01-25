//! Node emission context and helpers for trait-based dispatch

use crate::analyzer::Scope;
use crate::context::Context;
use crate::node::Node;
use crate::operators::Op;
use crate::type_kinds::Kind;
use wasm_encoder::*;

use super::config::EmitterConfig;
use super::string_table::StringTable;
use super::type_manager::TypeManager;

/// Context for emitting WASM instructions from nodes
/// Provides access to all emitter state needed during code generation
pub struct EmitContext<'a> {
	pub func: &'a mut Function,
	pub type_manager: &'a TypeManager,
	pub string_table: &'a mut StringTable,
	pub scope: &'a Scope,
	pub ctx: &'a Context,
	pub config: &'a EmitterConfig,
	pub next_temp_local: u32,
}

/// Create EmitContext from WasmGcEmitter
impl<'a> EmitContext<'a> {
	pub fn new_from_emitter(
		func: &'a mut Function,
		emitter: &'a mut super::WasmGcEmitter,
	) -> Self {
		EmitContext {
			func,
			type_manager: &emitter.type_manager,
			string_table: &mut emitter.string_table,
			scope: &emitter.scope,
			ctx: &emitter.ctx,
			config: &emitter.config,
			next_temp_local: emitter.next_temp_local,
		}
	}
}

impl<'a> EmitContext<'a> {
	/// Emit a call to a named function
	pub fn emit_call(&mut self, name: &str) {
		if let Some(f) = self.ctx.func_registry.get(name) {
			self.func.instruction(&Instruction::Call(f.call_index as u32));
		} else {
			panic!("Function {} not found in registry", name);
		}
	}

	/// Emit the Kind tag as i64 constant
	pub fn emit_kind(&mut self, kind: Kind) {
		self.func.instruction(&Instruction::I64Const(kind as i64));
	}

	/// Get a RefType for Node with specified nullability
	pub fn node_ref(&self, nullable: bool) -> RefType {
		self.type_manager.node_ref(nullable)
	}

	/// Get the type (Kind) of a node
	pub fn get_type(&self, node: &Node) -> Kind {
		crate::analyzer::infer_type(node, self.scope)
	}

	/// Check if node is numeric (can be evaluated to i64/f64)
	pub fn is_numeric(&self, node: &Node) -> bool {
		let kind = self.get_type(node);
		kind.is_int() || kind.is_float()
	}

	/// Allocate a string in the data section, returns (ptr, len)
	pub fn allocate_string(&mut self, s: &str) -> (u32, u32) {
		self.string_table.allocate(s)
	}

	/// Emit a string as a constructor call (new_text or new_symbol)
	pub fn emit_string_call(&mut self, s: &str, constructor: &str) {
		let (ptr, len) = self.allocate_string(s);
		self.func.instruction(&Instruction::I32Const(ptr as i32));
		self.func.instruction(&Instruction::I32Const(len as i32));
		self.emit_call(constructor);
	}
}

/// Trait for types that can emit themselves as WASM instructions
/// This allows dispatch-based code generation instead of monolithic match statements
pub trait NodeEmitter {
	/// Emit this node as a Node reference (ref $Node)
	fn emit_node(&self, ctx: &mut EmitContext) -> Result<(), String>;

	/// Emit this node as a primitive value (i64/f64) if possible
	/// Falls back to emit_node if not a primitive type
	fn emit_value(&self, ctx: &mut EmitContext) -> Result<(), String> {
		self.emit_node(ctx)
	}
}
