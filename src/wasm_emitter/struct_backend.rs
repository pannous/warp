//! A variable that only ever holds instances of one class is a GC struct: `p = Point(3, 4)` is a `Point·instance`
//! (an int field i64, a float field f64, any other a Node) built by struct.new, and `p.x` a struct.get by field index
//! instead of searching the field names of the instance Node (struct_body, map_find). The struct never leaves its
//! variable: wherever the program needs the instance as a value it becomes the same Node the construction builds,
//! `Point{x:3 y:4}`, made from the fields (notes/classes.md "Representation").

use super::map_backend::is_update;
use super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use crate::type_constructor::{entry_name, instance_parts};
use crate::type_kinds::{field_storage, FieldStorage, Kind, TypeDef};
use std::collections::HashMap;
use wasm_encoder::*;
use Instruction as I;

pub(super) const INSTANCE_SUFFIX: &str = "·instance";

/// The struct type of the instances of one class held as GC structs, and the kind each field is stored as
#[derive(Clone)]
pub(super) struct InstanceType {
	pub(super) type_index: u32,
	pub(super) fields: Vec<(String, Kind)>,
}

/// A field of a struct variable: its slot, struct type, field index and stored kind
struct FieldAccess {
	slot: u32,
	type_index: u32,
	field_index: u32,
	kind: Kind,
}

impl WasmGcEmitter {
	/// One mutable `P·instance` struct type per class, after the user types
	pub(super) fn emit_instance_types(&mut self) {
		let types: Vec<TypeDef> = self.ctx.type_registry.types().to_vec();
		for type_def in types {
			let fields: Vec<(String, Kind)> = type_def.fields.iter().map(|field| (field.name.clone(), stored_kind(&field.type_name))).collect();
			let wasm_fields = fields.iter().map(|(_, kind)| FieldType { element_type: StorageType::Val(self.storage_type(*kind)), mutable: true }).collect();
			let type_index = self.type_manager.add_struct_type(wasm_fields);
			self.instance_types.insert(type_def.name.clone(), InstanceType { type_index, fields });
		}
	}

	/// The variables of `program` held as GC structs, with their class: locals assigned only constructions of one class
	/// that give every field in declared order, with values of the field's kind
	pub(super) fn find_typed_structs(&self, program: &Node) -> HashMap<String, String> {
		let (mut classes, mut excluded): (HashMap<String, String>, Vec<String>) = (HashMap::new(), vec![]);
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) => match self.constructed_class(value) {
					Some(class) if classes.get(name).is_none_or(|known| *known == class) => { classes.insert(name.clone(), class); }
					_ => excluded.push(name.clone()),
				},
				Node::Symbol(name) if is_update(op) => excluded.push(name.clone()),
				Node::Key(instance, Op::Hash, _) if matches!(op, Op::Assign | Op::Define) || is_update(op) => excluded.push(instance.drop_meta().name()),
				_ => {}
			}
		});
		excluded.extend(self.names_held_as_nodes(program));
		classes.retain(|name, _| !excluded.contains(name) && self.scope.lookup(name).is_some_and(|local| !local.is_param));
		classes
	}

	/// The class whose instance `value` constructs, when a struct can hold it
	fn constructed_class(&self, value: &Node) -> Option<String> {
		let (class, fields) = instance_parts(value)?;
		let instance = self.instance_types.get(&class.name())?;
		let Node::List(entries, _, _) = fields.drop_meta() else { return None };
		let fits = entries.len() == instance.fields.len()
			&& entries.iter().zip(&instance.fields).all(|(entry, (field, kind))| {
				entry_name(entry).as_ref() == Some(field) && (kind.is_ref() || self.get_type(entry_value(entry)) == *kind)
			});
		fits.then(|| class.name())
	}

	pub(super) fn instance_ref(&self, class: &str) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.instance_types[class].type_index) }
	}

	/// The local slot and class of `target` when it is a struct variable
	pub(super) fn typed_struct(&self, target: &Node) -> Option<(u32, &InstanceType)> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		let class = self.typed_structs.get(name)?;
		Some((self.scope.lookup(name)?.position, self.instance_types.get(class)?))
	}

	/// `target#index` when target is a struct variable and the index one of its fields by name
	fn field_access(&self, target: &Node, index: &Node) -> Option<FieldAccess> {
		let (slot, instance) = self.typed_struct(target)?;
		let name = crate::analyzer::constant_field_name(crate::wasp_parser::subscript_key(index)?)?;
		let field_index = instance.fields.iter().position(|(field, _)| *field == name)?;
		Some(FieldAccess { slot, type_index: instance.type_index, field_index: field_index as u32, kind: instance.fields[field_index].1 })
	}

	fn emit_field_get(&self, func: &mut Function, access: &FieldAccess) {
		Self::emit_list(func, &[I::LocalGet(access.slot), I::StructGet { struct_type_index: access.type_index, field_index: access.field_index }]);
	}

	/// Push `target#index` as a Node when it reads a field of a struct variable; false otherwise
	pub(super) fn emit_struct_field_node(&mut self, func: &mut Function, target: &Node, index: &Node) -> bool {
		let Some(access) = self.field_access(target, index) else { return false };
		self.emit_field_get(func, &access);
		if !access.kind.is_ref() {
			self.emit_primitive_as_node(func, access.kind);
		}
		true
	}

	/// Push `target#index` as an i64 when it reads an int field of a struct variable; false otherwise
	pub(super) fn emit_struct_field_int(&mut self, func: &mut Function, target: &Node, index: &Node) -> bool {
		let Some(access) = self.field_access(target, index).filter(|access| access.kind == Kind::Int) else { return false };
		self.emit_field_get(func, &access);
		true
	}

	/// `p = P{…}`: the struct of the field values in p's local, left on the stack
	pub(super) fn emit_typed_struct_store(&mut self, func: &mut Function, slot: u32, class: &str, value: &Node) {
		let (_, fields) = instance_parts(value).expect("find_typed_structs admits only constructions");
		let Node::List(entries, _, _) = fields.drop_meta() else { unreachable!("find_typed_structs admits only entry lists") };
		let values: Vec<Node> = entries.iter().map(|entry| entry_value(entry).clone()).collect();
		self.emit_struct_new(func, class, &values);
		func.instruction(&I::LocalTee(slot));
	}

	/// A new struct of the class from the values of its fields in declared order
	fn emit_struct_new(&mut self, func: &mut Function, class: &str, values: &[Node]) {
		let instance = self.instance_types[class].clone();
		for (value, (_, kind)) in values.iter().zip(&instance.fields) {
			self.emit_value_of_kind(func, value, *kind);
		}
		func.instruction(&I::StructNew(instance.type_index));
	}

	/// Push a struct variable as the instance Node its construction builds
	pub(super) fn emit_typed_struct_as_node(&mut self, func: &mut Function, name: &str) {
		let class = self.typed_structs[name].clone();
		let entries = self.instance_types[&class].fields.iter()
			.map(|(field, _)| Node::Key(Box::new(Node::Symbol(field.clone())), Op::Colon, Box::new(field_read(name, field))))
			.collect();
		let instance = crate::type_constructor::instance_node(&class, entries);
		self.emit_node_instructions(func, &instance);
	}

	/// Per user function: the class of each parameter it takes as a struct. A parameter `p:P` qualifies when the body
	/// only reads fields of it and every call passes a variable or a construction of P: such an argument becomes the
	/// struct without making an instance Node (a struct variable is passed as it is)
	pub(super) fn find_struct_abi(&self, program: &Node) -> HashMap<String, Vec<Option<String>>> {
		let functions = self.directly_called_functions();
		let trees: Vec<&Node> = std::iter::once(program).chain(functions.iter().map(|function| function.body.as_ref())).collect();
		let mut abi = HashMap::new();
		for function in &functions {
			let classes: Vec<Option<String>> = function.params.iter().map(|param| {
				let class = param.annotation.as_ref().map(Node::name).filter(|class| self.instance_types.contains_key(class))?;
				(param.default.is_none() && reads_only_fields(&function.body, &param.name, &self.instance_types[&class])).then_some(class)
			}).collect();
			if classes.iter().all(Option::is_none) {
				continue;
			}
			let passes_structs = |call: &[Node]| call.len() == classes.len() + 1 && call[1..].iter().zip(&classes).zip(&function.params).all(|((argument, class), param)| match class {
				None => true,
				Some(class) => match argument.drop_meta() {
					Node::Symbol(_) => true,
					Node::Key(name, Op::Colon, _) if name.drop_meta().name() == param.name => true, // the definition's head
					_ => self.constructed_class(argument).as_ref() == Some(class),
				},
			});
			let (mut mentions, mut struct_calls) = (0, 0);
			for tree in &trees {
				tree.visit(&mut |part: &Node| match part {
					Node::Symbol(name) if *name == function.name => mentions += 1,
					Node::List(items, _, _) if super::list_abi::called_function(part) == Some(function.name.as_str()) && passes_structs(items) => struct_calls += 1,
					_ => {}
				});
			}
			if mentions == struct_calls {
				abi.insert(function.name.clone(), classes);
			}
		}
		abi
	}

	/// The class of the parameter `index` of `function` when it takes it as a struct
	pub(super) fn struct_parameter(&self, function: &str, index: usize) -> Option<&String> {
		self.struct_abi.get(function)?.get(index)?.as_ref()
	}

	/// An argument where a struct of `class` is wanted: a struct variable as it is, a construction or another
	/// variable's fields made into one
	pub(super) fn emit_struct_argument(&mut self, func: &mut Function, argument: &Node, class: &str) {
		if let Some((slot, instance)) = self.typed_struct(argument) {
			if self.instance_types[class].type_index == instance.type_index {
				func.instruction(&I::LocalGet(slot));
				return;
			}
		}
		let values: Vec<Node> = match (instance_parts(argument), argument.drop_meta()) {
			(Some((_, fields)), _) => match fields.drop_meta() {
				Node::List(entries, _, _) => entries.iter().map(|entry| entry_value(entry).clone()).collect(),
				other => unreachable!("find_struct_abi admits constructions of entry lists, not {other:?}"),
			},
			(None, Node::Symbol(name)) => self.instance_types[class].fields.iter().map(|(field, _)| field_read(name, field)).collect(),
			(None, other) => unreachable!("find_struct_abi admits variables and constructions, not {other:?}"),
		};
		self.emit_struct_new(func, class, &values);
	}
}

/// `name#field`, the lowered `name.field`
fn field_read(name: &str, field: &str) -> Node {
	let one_based = Node::Key(Box::new(Node::Text(field.to_string())), Op::Add, Box::new(crate::node::int(1)));
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Hash, Box::new(one_based))
}

/// Is every use of the parameter in the body a read of one of the fields
fn reads_only_fields(body: &Node, param: &str, instance: &InstanceType) -> bool {
	let (mut uses, mut reads, mut writes) = (0, 0, 0);
	body.visit(&mut |part| match part {
		Node::Symbol(name) if name == param => uses += 1,
		Node::Key(target, op, _) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
			if names(target, param) || matches!(target.drop_meta(), Node::Key(instance, Op::Hash, _) if names(instance, param)) {
				writes += 1;
			}
		}
		_ => {}
	});
	body.visit(&mut |part| if let Node::Key(target, Op::Hash, index) = part {
		let field = crate::wasp_parser::subscript_key(index).and_then(crate::analyzer::constant_field_name);
		if names(target, param) && field.is_some_and(|field| instance.fields.iter().any(|(name, _)| *name == field)) {
			reads += 1;
		}
	});
	writes == 0 && uses == reads
}

/// The kind a field of the declared type is stored as: int and float unboxed, anything else a Node
fn stored_kind(type_name: &str) -> Kind {
	match field_storage(type_name) {
		FieldStorage::I64 | FieldStorage::I32 => Kind::Int,
		FieldStorage::F64 | FieldStorage::F32 => Kind::Float,
		_ => Kind::Key,
	}
}

fn entry_value(entry: &Node) -> &Node {
	match entry.drop_meta() {
		Node::Key(_, _, value) => value,
		other => other,
	}
}

fn names(node: &Node, variable: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == variable)
}
