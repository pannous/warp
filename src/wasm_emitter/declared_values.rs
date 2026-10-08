//! P204: a value whose type only the run time knows (static type any, held as a Node) or whose static kind the declared
//! type does not admit, going into an annotated place (`x: int = y`, `t: text = y`, a parameter `f(p: P)`), is checked
//! when it runs: it has the declared type, else the error "not a <type>". Unannotated places stay lax.
use super::WasmGcEmitter;
use crate::node::Node;
use crate::type_kinds::{Kind, BOOL_MASK_BIT};
use wasm_encoder::*;
use Instruction as I;

/// What a declared type admits of a value at run time
enum Admitted {
	/// the run-time kinds of a builtin type (type_tests::runtime_kind_mask), or of a declared type (OBJECT_KINDS)
	Kinds(i64),
	/// a bool, or the Int 1 or 0 (P199)
	Bool,
}

/// The run-time kinds of a value of a declared type: an object, also one that fits by its fields (`like`), or a variant
const OBJECT_KINDS: [Kind; 5] = [Kind::Key, Kind::List, Kind::Block, Kind::Data, Kind::Symbol];

/// Meta key marking the value of a field with the field's declared type, for the check when the instance is built
const DECLARED_TYPE: &str = "declared type";

/// The value and declared type of a field value marked by with_declared_field_types
pub(super) fn declared_field_value(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Meta { node, data } = node else { return None };
	match data.as_ref() {
		Node::Key(key, _, declared) if key.name() == DECLARED_TYPE => Some((node, declared)),
		_ => None,
	}
}

impl WasmGcEmitter {
	fn admitted(&self, declared: &Node) -> Option<Admitted> {
		let type_name = declared.name();
		let object_mask = OBJECT_KINDS.iter().fold(0, |mask, kind| mask | 1 << *kind as i64);
		let spec = match crate::analyzer::annotated_kind(declared) {
			Some(Kind::Empty) => return None, // any, an inline union
			Some(Kind::Key) => return Some(Admitted::Kinds(object_mask)),
			Some(Kind::List) => "list", // `xs: [int]`, `xs: ints`
			_ if crate::analyzer::is_bool_type(&type_name) => return Some(Admitted::Bool),
			_ if self.ctx.type_registry.get_by_name(&type_name).is_some() => return Some(Admitted::Kinds(object_mask)),
			Some(Kind::Float) => "number", // an int fits a float
			_ => &type_name,
		};
		crate::type_tests::runtime_kind_mask(spec).map(Admitted::Kinds)
	}

	/// The static kind of `value` leaves open whether it fits: held as a Node, or of a kind the type does not admit
	fn needs_run_time_check(&self, admitted: &Admitted, value: &Node) -> bool {
		let given = self.get_type(value);
		match admitted {
			_ if matches!(given, Kind::Empty | Kind::Data) => true,
			Admitted::Kinds(mask) => mask & (1 << given as i64) == 0,
			Admitted::Bool => !crate::analyzer::is_boolean(value, &self.scope),
		}
	}

	/// The fields `{x: v …}` of an instance of `class`, each value marked with its field's declared type
	pub(super) fn with_declared_field_types(&self, class: &Node, fields: &Node) -> Node {
		let Some(type_def) = self.ctx.type_registry.get_by_name(&class.name()) else { return fields.clone() };
		let Node::List(entries, bracket, separator) = fields.drop_meta() else { return fields.clone() };
		let marked = entries.iter().map(|entry| match entry.drop_meta() {
			Node::Key(field, op, value) => match type_def.fields.iter().find(|declared| declared.name == field.name()) {
				Some(declared) => {
					let data = Node::key(DECLARED_TYPE, Node::Symbol(declared.type_name.clone()));
					Node::Key(field.clone(), *op, Box::new(Node::Meta { node: value.clone(), data: Box::new(data) }))
				}
				None => entry.clone(),
			},
			_ => entry.clone(),
		});
		Node::List(marked.collect(), bracket.clone(), separator.clone())
	}

	/// The declared type of the variable `target` (`x: int`, a parameter `f(x: int)` of the function compiled), if it has one
	pub(super) fn declared_type_of(&self, target: &Node) -> Option<Node> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		let local = self.scope.lookup(name)?;
		if let Some(type_node) = &local.type_node {
			return Some(type_node.as_ref().clone());
		}
		let function = self.ctx.user_functions.get(self.compiling.as_deref()?).filter(|_| local.is_param)?;
		function.params.iter().find(|param| param.name == *name)?.annotation.clone()
	}

	/// `value` stored into a place declared `declared` that holds a `kind`: checked at run time when its static kind leaves
	/// open whether it fits (P204)
	pub(super) fn emit_declared_value(&mut self, func: &mut Function, declared: Option<&Node>, value: &Node, kind: Kind) {
		let check = declared.and_then(|declared| self.admitted(declared)).filter(|admitted| self.needs_run_time_check(admitted, value));
		let (Some(admitted), Some(declared)) = (check, declared) else { return self.emit_value_of_kind(func, value, kind) };
		self.emit_node_instructions(func, value);
		self.emit_admitted_node(func, &admitted, declared, kind);
	}

	/// The Node on top of the stack into a number `kind` local: unboxed when its run-time kind is a number, else the
	/// error "not an int" (`a = 0; a, b = xs`)
	pub(super) fn emit_node_as_number(&mut self, func: &mut Function, kind: Kind) {
		let declared = Node::Symbol(if kind.is_float() { "float" } else { "int" }.to_string());
		let admitted = self.admitted(&declared).expect("int and float are builtin types");
		self.emit_admitted_node(func, &admitted, &declared, kind);
	}

	/// The Node on top of the stack, as a `kind` value, when `admitted` admits it; else the error "not a <declared>"
	fn emit_admitted_node(&mut self, func: &mut Function, admitted: &Admitted, declared: &Node, kind: Kind) {
		func.instruction(&I::RefAsNonNull);
		let held = self.node_scratch();
		func.instruction(&I::LocalSet(held));
		self.emit_admits(func, held, admitted);
		Self::emit_list(func, &[I::I64Eqz, I::If(BlockType::Empty)]);
		self.emit_trap_detail(func, &Node::Text(format!("not {}", crate::analyzer::with_article(&declared.name()))));
		self.emit_runtime_error(func, super::list_ops::RETURNED_ERROR);
		Self::emit_list(func, &[I::End, I::LocalGet(held), I::RefAsNonNull]);
		if kind.is_float() {
			self.emit_held_node_as_f64(func);
		} else if !kind.is_ref() {
			self.emit_call(func, "get_int_value");
		}
	}

	/// i64 1 when the Node in local `held` is a value the declared type admits
	fn emit_admits(&mut self, func: &mut Function, held: u32, admitted: &Admitted) {
		match admitted {
			Admitted::Kinds(mask) => self.emit_kind_in(func, held, *mask),
			Admitted::Bool => {
				self.emit_kind_in(func, held, 1 << BOOL_MASK_BIT);
				self.emit_kind_in(func, held, 1 << Kind::Int as i64);
				Self::emit_list(func, &[I::I32WrapI64, I::If(BlockType::Result(ValType::I64)), I::LocalGet(held), I::RefAsNonNull]);
				self.emit_call(func, "get_int_value");
				Self::emit_list(func, &[I::I64Const(1), I::I64LeU, I::I64ExtendI32U, I::Else, I::I64Const(0), I::End, I::I64Or]);
			}
		}
	}

	fn emit_kind_in(&mut self, func: &mut Function, held: u32, mask: i64) {
		Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I::I64Const(mask)]);
		self.emit_call(func, crate::type_tests::NODE_KIND_IN);
	}
}

/// list_mark(list, mark) -> list: the head cell of a declared list takes its element mark (type_kinds::element_mark), so a
/// write through any alias, a lax variable or an unannotated parameter, checks its item at run time (P215); so does ø,
/// which keeps the mark when its first item makes it a cell; any other node stays as it is
pub(super) const LIST_MARK: &str = "list_mark";
/// list_item_check(list, item): the run-time error when the list's element mark does not admit the item
pub(super) const LIST_ITEM_CHECK: &str = "list_item_check";
/// list_items_check(list, items): list_item_check of each item of the list `items`
pub(super) const LIST_ITEMS_CHECK: &str = "list_items_check";
pub(super) const LIST_CANNOT_HOLD: &str = "list_cannot_hold_this_item";
/// The in-place writers of a list, which check their items against its element mark
const MARK_CHECKING_WRITERS: [&str; 3] = [super::list_ops::LIST_EXTEND, crate::analyzer::INSERT_AT_CALL, "node_with_at"];

/// Whether the program declares a list type anywhere (`names: texts`, its lowered form the type as the target's Meta, a
/// field `items: [text]`); a program without one emits no element marks and no item checks
pub(super) fn declares_list_types(node: &Node) -> bool {
	let is_list_type = |declared: &Node| crate::analyzer::list_element_type(&declared.name()).is_some();
	match node {
		Node::Meta { node, data } => is_list_type(data) || declares_list_types(node),
		Node::Key(left, op, right) => (*op == crate::operators::Op::Colon && is_list_type(right)) || declares_list_types(left) || declares_list_types(right),
		Node::List(items, _, _) => items.iter().any(declares_list_types),
		_ => false,
	}
}

impl WasmGcEmitter {
	/// Whether this program's lists carry element marks (LIST_MARK): only then the writers check and keep them
	pub(super) fn marks_lists(&self) -> bool {
		self.should_emit_function(LIST_MARK)
	}

	/// The element mark of a list declared `declared` (`names: texts`), when a writer could check it
	pub(super) fn element_mark_of(&self, declared: &Node) -> Option<i64> {
		if !self.should_emit_function(LIST_MARK) {
			return None;
		}
		let type_name = declared.name();
		let element = crate::analyzer::list_element_type(&type_name)?;
		let mask = match self.admitted(&Node::Symbol(element.to_string()))? {
			Admitted::Kinds(mask) => mask,
			Admitted::Bool => 1 << BOOL_MASK_BIT,
		};
		Some(crate::type_kinds::element_mark(mask))
	}

	pub(super) fn require_list_marks(&mut self) {
		if self.declares_list_types && MARK_CHECKING_WRITERS.iter().any(|writer| self.should_emit_function(writer)) {
			self.ctx.required_functions.extend([crate::type_tests::NODE_KIND_IN, LIST_MARK, LIST_ITEM_CHECK, LIST_ITEMS_CHECK]);
		}
	}

	pub(super) fn emit_list_marks(&mut self) {
		use crate::type_kinds::{ELEMENT_MARK_SHIFT, KIND_MASK, MARK_BOOL_BIT, MARK_KINDS_MASK};
		if !self.should_emit_function(LIST_MARK) {
			return;
		}
		let node_type = self.type_manager.node_type;
		let (node_ref, nullable) = (ValType::Ref(self.node_ref(false)), ValType::Ref(self.node_ref(true)));
		let is_list = [I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::List as i64), I::I64Eq];
		let is_list_or_empty = [I::I64Const(KIND_MASK), I::I64And, I::LocalTee(2), I::I64Const(Kind::List as i64), I::I64Eq,
			I::LocalGet(2), I::I64Const(Kind::Empty as i64), I::I64Eq, I::I32Or];
		self.runtime_function(LIST_MARK, vec![node_ref, ValType::I64], vec![node_ref], vec![ValType::I64], |s, f| {
			let (list, mark) = (0, 1);
			s.emit_field(f, list, 0);
			Self::emit_list(f, &is_list_or_empty);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(list)]);
			s.emit_field(f, list, 0);
			Self::emit_list(f, &[I::LocalGet(mark), I::I64Or, I::StructSet { struct_type_index: node_type, field_index: 0 }, I::End, I::LocalGet(list)]);
		});
		self.runtime_function(LIST_ITEM_CHECK, vec![nullable, node_ref], vec![], vec![ValType::I64], |s, f| {
			let (list, item, mark) = (0, 1, 2);
			Self::emit_list(f, &[I::LocalGet(list), I::RefIsNull, I::If(BlockType::Empty), I::Return, I::End]);
			s.emit_field(f, list, 0);
			Self::emit_list(f, &is_list_or_empty);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::Return, I::End]);
			s.emit_field(f, list, 0);
			Self::emit_list(f, &[I::I64Const(ELEMENT_MARK_SHIFT), I::I64ShrU, I::LocalTee(mark), I::I64Eqz, I::If(BlockType::Empty), I::Return, I::End]);
			// the node_kind_in mask of the mark: its kind bits, its bool bit back at BOOL_MASK_BIT
			Self::emit_list(f, &[I::LocalGet(item), I::LocalGet(mark), I::I64Const(MARK_KINDS_MASK), I::I64And,
				I::LocalGet(mark), I::I64Const(MARK_BOOL_BIT), I::I64ShrU, I::I64Const(1), I::I64And, I::I64Const(BOOL_MASK_BIT), I::I64Shl, I::I64Or]);
			s.call(f, crate::type_tests::NODE_KIND_IN);
			f.instruction(&I::I64Eqz);
			s.emit_fail_if(f, LIST_CANNOT_HOLD);
		});
		self.runtime_function(LIST_ITEMS_CHECK, vec![nullable, nullable], vec![], vec![], |s, f| {
			let (list, cell) = (0, 1);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
			s.emit_field(f, cell, 0);
			Self::emit_list(f, &is_list);
			Self::emit_list(f, &[I::I32Eqz, I::BrIf(1), I::LocalGet(list)]);
			s.emit_field(f, cell, 1);
			f.instruction(&I::RefCastNonNull(HeapType::Concrete(node_type)));
			s.call(f, LIST_ITEM_CHECK);
			s.emit_field(f, cell, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
		});
	}
}
