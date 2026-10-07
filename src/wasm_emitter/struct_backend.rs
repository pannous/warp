//! A variable that only ever holds instances of one class is a GC struct: `p = Point(3, 4)` is a `Point·instance`
//! (an int field i64, a float field f64, any other a Node) built by struct.new, and `p.x` a struct.get by field index
//! instead of searching the field names of the instance Node (struct_body, map_find). The struct never leaves its
//! variable: wherever the program needs the instance as a value it becomes the same Node the construction builds,
//! `Point{x:3 y:4}`, made from the fields (notes/classes.md "Representation").

use super::map_backend::{entry_update, is_update};
use super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use crate::type_constructor::{entry_name, entry_value, instance_parts};
use crate::type_kinds::{field_storage, FieldStorage, Kind, TypeDef};
use std::collections::HashMap;
use wasm_encoder::*;
use Instruction as I;

pub(super) const INSTANCE_SUFFIX: &str = "·instance";

/// The struct ABI of a program (the emitter's struct_abi, struct_results): by function, which parameters are structs of
/// which class, and the class of the struct it gives back
pub(super) type StructAbi = (HashMap<String, Vec<Option<String>>>, HashMap<String, String>);

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
	/// that give every field in declared order, and field writes, all with values of the field's kind
	pub(super) fn find_typed_structs(&self, program: &Node) -> HashMap<String, String> {
		let mut classes = self.struct_variables(program, &self.struct_results);
		classes.retain(|name, _| self.scope.lookup(name).is_some_and(|local| !local.is_param));
		classes
	}

	/// find_typed_structs before the scope is known: the variables a program, or a function body except its
	/// parameters, holds as structs
	fn struct_variables(&self, program: &Node, results: &HashMap<String, String>) -> HashMap<String, String> {
		let (mut classes, mut excluded): (HashMap<String, String>, Vec<String>) = (HashMap::new(), vec![]);
		// (variable, the one-based field index, the value written)
		let mut writes: Vec<(String, Node, Node)> = vec![];
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if *op == Op::Assign && entry_update(name, value).is_some() => {
					let (index, written) = entry_update(name, value).expect("guarded");
					writes.push((name.clone(), index, written));
				}
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) => match self.constructed_class(value).or_else(|| struct_call_class(value, results)) {
					Some(class) if classes.get(name).is_none_or(|known| *known == class) => { classes.insert(name.clone(), class); }
					_ => excluded.push(name.clone()),
				},
				Node::Symbol(name) if is_update(op) => excluded.push(name.clone()),
				Node::Key(instance, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || op.is_compound_assign() => {
					let written = if *op == Op::Assign { value.as_ref().clone() } else { Node::Key(target.clone(), op.base_op(), value.clone()) };
					writes.push((instance.drop_meta().name(), index.as_ref().clone(), written));
				}
				Node::Key(instance, Op::Hash, _) if is_update(op) => excluded.push(instance.drop_meta().name()),
				_ => {}
			}
		});
		for (name, index, written) in writes {
			// a value that certainly does not fit keeps the struct: writing it is a type error (emit_struct_field_set)
			let fits = classes.get(&name).and_then(|class| field_of(&self.instance_types[class], &index))
				.is_some_and(|(_, kind)| self.fits(&written, kind) || self.misfits(&written, kind));
			if !fits {
				excluded.push(name);
			}
		}
		excluded.extend(self.names_held_as_nodes(program));
		classes.retain(|name, _| !excluded.contains(name));
		classes
	}

	/// Does a value of this node go into a field of the kind: any into a Node field, a number only of its own kind
	fn fits(&self, value: &Node, kind: Kind) -> bool {
		kind.is_ref() || self.get_type(value) == kind
	}

	/// The kind of a written value; `p.y += 0.5` adds a float, so its result is one
	fn given_kind(&self, value: &Node) -> Kind {
		match (value.drop_meta(), self.get_type(value)) {
			(Node::Key(_, op, right), Kind::Int) if op.is_arithmetic() && self.get_type(right) == Kind::Float => Kind::Float,
			(_, Kind::Codepoint) => Kind::Text, // "a" is a one-letter text
			(_, given) => given,
		}
	}

	/// Is the value certainly no value of the number field's kind: a text, a list, a float for an int field
	fn misfits(&self, value: &Node, kind: Kind) -> bool {
		let given = self.given_kind(value);
		let known = matches!(given, Kind::Int | Kind::Float | Kind::Text | Kind::Codepoint | Kind::List);
		!kind.is_ref() && known && given != kind && !(kind == Kind::Float && given == Kind::Int)
	}

	/// The class whose instance `value` constructs, when a struct can hold it
	fn constructed_class(&self, value: &Node) -> Option<String> {
		let (class, fields) = instance_parts(value)?;
		let instance = self.instance_types.get(&class.name())?;
		let Node::List(entries, _, _) = fields.drop_meta() else { return None };
		let fits = entries.len() == instance.fields.len()
			&& entries.iter().zip(&instance.fields).all(|(entry, (field, kind))| {
				entry_name(entry).as_ref() == Some(field) && self.fits(entry_value(entry), *kind)
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
		let (field_index, kind) = field_of(instance, index)?;
		Some(FieldAccess { slot, type_index: instance.type_index, field_index, kind })
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

	/// `target#index = value` when it writes a field of a struct variable, leaving the i64 an index assignment gives
	/// (emit_assigned_entry_value); false otherwise
	pub(super) fn emit_struct_field_set(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) -> bool {
		let Some(access) = self.field_access(target, index) else { return false };
		if self.misfits(value, access.kind) {
			let class = self.typed_structs[&target.drop_meta().name()].clone();
			let field = &self.ctx.type_registry.get_by_name(&class).expect("a declared class").fields[access.field_index as usize];
			let given = crate::analyzer::kind_with_article(self.given_kind(value));
			let message = format!("{} of {class} is {} field, got {} ({given})", field.name, crate::analyzer::with_article(&field.type_name), value.serialize());
			self.emit_type_error(func, message);
			return true;
		}
		func.instruction(&I::LocalGet(access.slot));
		let class = self.typed_structs[&target.drop_meta().name()].clone();
		self.emit_field_value(func, &class, access.field_index as usize, value, access.kind);
		func.instruction(&I::StructSet { struct_type_index: access.type_index, field_index: access.field_index });
		let key = crate::wasp_parser::subscript_key(index).expect("a field by name").clone();
		self.emit_assigned_entry_value(func, target, &key, value);
		true
	}

	/// `p = P{…}`: the struct of the field values in p's local; `p = field_with(p, "x", v)`: the field set in place.
	/// The struct is left on the stack
	pub(super) fn emit_typed_struct_store(&mut self, func: &mut Function, name: &str, slot: u32, class: &str, value: &Node) {
		if let Some((index, written)) = entry_update(name, value) {
			let target = Node::Symbol(name.to_string());
			self.emit_struct_field_set(func, &target, &index, &written);
			Self::emit_list(func, &[I::Drop, I::LocalGet(slot)]);
			return;
		}
		if let Some(callee) = super::list_abi::called_function(value).filter(|callee| self.struct_results.contains_key(*callee)) {
			let Node::List(items, _, _) = value.drop_meta() else { unreachable!("a call") };
			let user_fn = self.ctx.user_functions[callee].clone();
			self.emit_user_function_call_inner(func, &user_fn, &items[1..]);
			func.instruction(&I::LocalTee(slot));
			return;
		}
		let (_, fields) = instance_parts(value).expect("find_typed_structs admits only constructions and struct calls");
		let Node::List(entries, _, _) = fields.drop_meta() else { unreachable!("find_typed_structs admits only entry lists") };
		let values: Vec<Node> = entries.iter().map(|entry| entry_value(entry).clone()).collect();
		self.emit_struct_new(func, class, &values);
		func.instruction(&I::LocalTee(slot));
	}

	/// The value of a field as its kind stores it; an int field never takes a fraction (`p.x = 0.5` traps)
	fn emit_field_value(&mut self, func: &mut Function, class: &str, field_index: usize, value: &Node, kind: Kind) {
		self.emit_value_of_kind(func, value, kind);
		let field = self.ctx.type_registry.get_by_name(class).and_then(|type_def| type_def.fields.get(field_index));
		// an optional int is stored as a Node (ø or the number): no whole check there
		if kind == Kind::Int && field.is_some_and(|field| crate::analyzer::is_whole_type(&field.type_name)) {
			self.emit_whole_check(func);
		}
	}

	/// A new struct of the class from the values of its fields in declared order
	fn emit_struct_new(&mut self, func: &mut Function, class: &str, values: &[Node]) {
		let instance = self.instance_types[class].clone();
		for (field_index, (value, (_, kind))) in values.iter().zip(&instance.fields).enumerate() {
			self.emit_field_value(func, class, field_index, value, *kind);
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

	/// The calling convention of structs, decided before the signatures are registered (struct_abi, struct_results).
	/// A parameter `p:P` is a struct when the body only reads and writes fields of it (and may give p back as its
	/// result) and every call passes a struct variable or a construction of P. Any other value keeps the Node parameter:
	/// a value that only looks like a P (duck typing) may lack fields the body never reads. A function that changes p or
	/// gives it back is called only as `x = f(x, …)` of a struct variable x: the struct it changes is the one replaced,
	/// so no caller sees a change by reference, and the result is a struct (`c.inc()` of a method changing c)
	pub(super) fn find_struct_abi(&self, program: &Node) -> StructAbi {
		let functions = self.directly_called_functions();
		let mut candidates: Vec<StructFunction> = functions.iter().filter_map(|function| self.struct_function(function)).collect();
		// greatest fixpoint: a function giving a struct needs callers whose variables are structs, which they are only
		// while the functions they are assigned from give structs
		loop {
			let results: HashMap<String, String> = candidates.iter().filter_map(|candidate| Some((candidate.name.clone(), candidate.result.clone()?))).collect();
			let trees: Vec<(&Node, HashMap<String, String>)> = std::iter::once((program, self.struct_variables(program, &results)))
				.chain(functions.iter().map(|function| {
					let mut variables = self.struct_variables(&function.body, &results);
					variables.retain(|name, _| function.params.iter().all(|param| param.name != *name));
					(function.body.as_ref(), variables)
				}))
				.collect();
			let kept: Vec<StructFunction> = candidates.iter().filter(|candidate| candidate.called_as_such(&trees, self)).cloned().collect();
			if kept.len() == candidates.len() {
				break;
			}
			candidates = kept;
		}
		let abi = candidates.iter().map(|candidate| (candidate.name.clone(), candidate.classes.clone())).collect();
		let results = candidates.into_iter().filter_map(|candidate| Some((candidate.name, candidate.result?))).collect();
		(abi, results)
	}

	/// The struct convention a function could take, judged by its body alone
	fn struct_function(&self, function: &crate::context::UserFunctionDef) -> Option<StructFunction> {
		let statements = super::list_abi::body_statements(&function.body);
		let mut returns = false;
		function.body.visit(&mut |part| returns |= matches!(part, Node::List(items, _, _) if items.first().is_some_and(|word| names(word, super::list_abi::RETURN))));
		let gives = |name: &str| !returns && statements.last().is_some_and(|last| names(last, name));
		let mut changed = None;
		let mut result = None;
		let classes: Vec<Option<String>> = function.params.iter().map(|param| {
			let class = param.annotation.as_ref().map(Node::name).filter(|class| self.instance_types.contains_key(class))?;
			let given_back = gives(&param.name);
			let written = param.default.as_ref().map_or_else(|| field_uses(&function.body, &param.name, &self.instance_types[&class], given_back), |_| None)?;
			if written || given_back {
				// one changed or given back struct per function: the one its call replaces
				if changed.is_some() {
					return None;
				}
				changed = Some(param.name.clone());
			}
			if given_back {
				result = Some(class.clone());
			}
			Some(class)
		}).collect();
		if classes.iter().all(Option::is_none) {
			return None;
		}
		let changed = changed.and_then(|name| function.params.iter().position(|param| param.name == name).filter(|index| classes[*index].is_some()));
		let result = result.filter(|_| changed.is_some());
		let params = function.params.iter().map(|param| param.name.clone()).collect();
		Some(StructFunction { name: function.name.clone(), params, classes, changed, result })
	}

	/// The body of a function giving back its struct parameter: its statements, then that struct
	pub(super) fn emit_struct_result_body(&mut self, func: &mut Function, body: &Node) {
		self.emit_body_then(func, body, |emitter, func, last| {
			let (slot, _) = emitter.typed_struct(last).expect("find_struct_abi: the result is the struct parameter");
			func.instruction(&I::LocalGet(slot));
		});
	}

	/// The class of the parameter `index` of `function` when it takes it as a struct
	pub(super) fn struct_parameter(&self, function: &str, index: usize) -> Option<&String> {
		self.struct_abi.get(function)?.get(index)?.as_ref()
	}

	/// An argument where a struct of `class` is wanted: a struct variable as it is, a construction made into one
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
			(None, other) => unreachable!("find_struct_abi admits struct variables and constructions of {class}, not {other:?}"),
		};
		self.emit_struct_new(func, class, &values);
	}
}

/// The class of the struct a call of a function giving one back gives
fn struct_call_class(value: &Node, results: &HashMap<String, String>) -> Option<String> {
	results.get(super::list_abi::called_function(value)?).cloned()
}

/// A function taking structs: the class of each struct parameter, the parameter it changes or gives back, and the
/// class of the struct it gives back
#[derive(Clone)]
struct StructFunction {
	name: String,
	params: Vec<String>,
	classes: Vec<Option<String>>,
	changed: Option<usize>,
	result: Option<String>,
}

impl StructFunction {
	/// Is every mention of the function a call passing structs (`trees`: each tree with its struct variables), and, when
	/// it changes or gives back a struct, the assignment `x = f(…, x, …)` to the struct variable x it passes there
	fn called_as_such(&self, trees: &[(&Node, HashMap<String, String>)], emitter: &WasmGcEmitter) -> bool {
		let passes = |call: &[Node], variables: &HashMap<String, String>| call.len() == self.classes.len() + 1
			&& call[1..].iter().zip(&self.classes).all(|(argument, class)| match class {
				None => true,
				Some(class) => match argument.drop_meta() {
					Node::Symbol(name) => variables.get(name) == Some(class),
					_ => emitter.constructed_class(argument).as_ref() == Some(class),
				},
			});
		// the definition's head `f(p:P, k)`
		let is_head = |call: &[Node]| call.len() == self.params.len() + 1 && call[1..].iter().zip(&self.params).all(|(argument, param)| match argument.drop_meta() {
			Node::Key(name, Op::Colon, _) => names(name, param),
			other => names(other, param),
		}) && call[1..].iter().any(|argument| matches!(argument.drop_meta(), Node::Key(_, Op::Colon, _)));
		let (mut mentions, mut heads, mut calls, mut replacing) = (0, 0, 0, 0);
		for (tree, variables) in trees {
			tree.visit(&mut |part: &Node| match part {
				Node::Symbol(name) if *name == self.name => mentions += 1,
				Node::List(items, _, _) if super::list_abi::called_function(part) == Some(self.name.as_str()) => {
					if is_head(items) {
						heads += 1;
					} else if passes(items, variables) {
						calls += 1;
					}
				}
				Node::Key(target, Op::Assign, value) if super::list_abi::called_function(value) == Some(self.name.as_str()) => {
					let Some(changed) = self.changed else { return };
					let Node::List(items, _, _) = value.drop_meta() else { return };
					let Node::Symbol(variable) = target.drop_meta() else { return };
					let passed_once = items[1..].iter().filter(|argument| names(argument, variable)).count() == 1;
					if passed_once && names(&items[changed + 1], variable) && passes(items, variables) {
						replacing += 1;
					}
				}
				_ => {}
			});
		}
		let replaced = if self.changed.is_some() { replacing } else { calls };
		calls == replaced && mentions == heads + replaced
	}
}

/// The index and stored kind of the field a one-based subscript `"x" + 1` names
fn field_of(instance: &InstanceType, index: &Node) -> Option<(u32, Kind)> {
	let name = match crate::wasp_parser::subscript_key(index)?.drop_meta() {
		Node::Text(name) => name.clone(),
		Node::Char(letter) => letter.to_string(),
		_ => return None,
	};
	let position = instance.fields.iter().position(|(field, _)| *field == name)?;
	Some((position as u32, instance.fields[position].1))
}

/// `name#field`, the lowered `name.field`
fn field_read(name: &str, field: &str) -> Node {
	let one_based = Node::Key(Box::new(Node::Text(field.to_string())), Op::Add, Box::new(crate::node::int(1)));
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Hash, Box::new(one_based))
}

/// Whether the body changes the parameter, when every use of it reads or writes one of its fields (and, `given_back`,
/// the one result that is the parameter itself); None for any other use
fn field_uses(body: &Node, param: &str, instance: &InstanceType, given_back: bool) -> Option<bool> {
	let (mut uses, mut fields, mut writes) = (0, 0, 0);
	let mut replaced = false;
	body.visit(&mut |part| match part {
		Node::Symbol(name) if name == param => uses += 1,
		Node::Key(target, op, _) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
			replaced |= names(target, param);
			if matches!(target.drop_meta(), Node::Key(instance, Op::Hash, _) if names(instance, param)) {
				writes += 1;
			}
		}
		_ => {}
	});
	body.visit(&mut |part| if let Node::Key(target, Op::Hash, index) = part {
		if names(target, param) && field_of(instance, index).is_some() {
			fields += 1;
		}
	});
	(!replaced && uses == fields + usize::from(given_back)).then_some(writes > 0)
}

/// The kind a field of the declared type is stored as: int and float unboxed, anything else a Node
fn stored_kind(type_name: &str) -> Kind {
	match field_storage(type_name) {
		FieldStorage::I64 | FieldStorage::I32 => Kind::Int,
		FieldStorage::F64 | FieldStorage::F32 => Kind::Float,
		_ => Kind::Key,
	}
}

fn names(node: &Node, variable: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == variable)
}
