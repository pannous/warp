//! P215 (card p215-user): lists are shared (P200b), so covariant lists are sound only for views that read. A view of a
//! declared list under a wider element type that changes the list (`widen(xs: list) := xs.add(420)` given
//! `names: texts`, `ys: list = names; ys.add(420)`, `ys: [Shape] = circles; ys.add(Shape(3))`) would put items of
//! another type into it: where the alias is visible that is a compile error. A view that only reads the list, or whose
//! element type fits the list's (the same type, a subclass), is fine.

use super::*;
use crate::wasm_emitter::list_sharing::changes_list;
use std::collections::HashMap;

pub fn check_widened_list_views(program: &Node) -> Option<Diagnostic> {
	let class_fields = crate::class_methods::class_fields(program);
	let classes: Vec<String> = class_fields.keys().cloned().collect();
	let lists = DeclaredLists { variables: declared_lists(program), instances: crate::class_methods::instance_classes(program, &classes), class_fields };
	if lists.variables.is_empty() && lists.class_fields.is_empty() {
		return None;
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	let mut found = None;
	program.visit(&mut |node| {
		if found.is_none() {
			found = lists.widened_argument(node, &context).or_else(|| lists.widened_alias(node, program));
		}
	});
	found
}

/// A declared list type as written (`texts`, `[Circle]`) and its element type
struct ListType {
	written: String,
	element: String,
}

impl ListType {
	fn of(annotation: &Node) -> Option<ListType> {
		let element = declared_element_type(annotation)?.to_string();
		Some(ListType { written: annotation.serialize().trim().to_string(), element })
	}
}

/// The declared list types: of variables, and of the list fields of instances whose class is known
struct DeclaredLists {
	variables: HashMap<String, ListType>,
	instances: HashMap<String, String>,
	class_fields: HashMap<String, Vec<(String, String)>>,
}

/// Each variable declared with a list type: `names: texts`, `names: list of text`, `xs: [Circle]`
fn declared_lists(program: &Node) -> HashMap<String, ListType> {
	let mut declared = HashMap::new();
	let mut declare = |target: &Node| {
		let Node::Key(name, Op::Colon, annotation) = target.drop_meta() else { return };
		let annotation = match annotation.drop_meta() {
			Node::Text(written) => Node::Symbol(written.clone()), // `names:"list of text"`, as of_type_declaration writes it
			other => other.clone(),
		};
		if let (Node::Symbol(name), Some(list_type)) = (name.drop_meta(), ListType::of(&annotation)) {
			declared.insert(name.clone(), list_type);
		}
	};
	program.visit(&mut |node| match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, _) => declare(target),
		Node::List(items, bracket, separator) => {
			if let Some(Node::Key(target, _, _)) = of_type_declaration(items, bracket, separator).as_ref().map(Node::drop_meta) {
				declare(target);
			}
		}
		_ => {}
	});
	declared
}

impl DeclaredLists {
	/// The list `names` or `b.items` (a field `items: texts` of b's class) as written, and its declared type
	fn declared(&self, list: &Node) -> Option<(String, ListType)> {
		match list.drop_meta() {
			Node::Symbol(name) => {
				let list_type = self.variables.get(name)?;
				Some((name.clone(), ListType { written: list_type.written.clone(), element: list_type.element.clone() }))
			}
			// `b.items`, read as the subscript `b#(items+1)` by now
			Node::Key(instance, op @ (Op::Dot | Op::Hash), field) => {
				let field = if *op == Op::Hash { crate::warp_parser::subscript_key(field)? } else { field }.drop_meta().name();
				let instance = instance.drop_meta().name();
				let (_, type_name) = self.class_fields.get(self.instances.get(&instance)?)?.iter().find(|(name, _)| *name == field)?;
				Some((format!("{instance}.{field}"), ListType::of(&Node::Symbol(type_name.clone()))?))
			}
			_ => None,
		}
	}

	/// `widen(names)` of a function that changes its parameter `xs: list`
	fn widened_argument(&self, call: &Node, context: &Context) -> Option<Diagnostic> {
		let Node::List(items, Bracket::Round, Separator::None) = call.drop_meta() else { return None };
		let (callee, arguments) = items.split_first()?;
		let function = context.user_functions.get(&callee.drop_meta().name())?;
		arguments.iter().zip(&function.params).find_map(|(argument, param)| {
			let view = param.annotation.as_ref()?;
			let view_name = format!("{}({}: {})", function.name, param.name, view.serialize().trim());
			self.widened_view(call, argument, view, &view_name).filter(|_| changes_list(&function.body, &param.name))
		})
	}

	/// `ys: list = names` where ys is changed later
	fn widened_alias(&self, assignment: &Node, program: &Node) -> Option<Diagnostic> {
		let Node::Key(target, Op::Assign | Op::Define, value) = assignment.drop_meta() else { return None };
		let Node::Key(alias, Op::Colon, view) = target.drop_meta() else { return None };
		let Node::Symbol(alias) = alias.drop_meta() else { return None };
		let view_name = format!("{alias}: {}", view.serialize().trim());
		self.widened_view(assignment, value, view, &view_name).filter(|_| changes_list(program, alias))
	}

	/// The error of the view `view_name` of type `view` of the declared list `list`, when the view's elements do not all
	/// fit the list's
	fn widened_view(&self, at: &Node, list: &Node, view: &Node, view_name: &str) -> Option<Diagnostic> {
		let (list, list_type) = self.declared(list)?;
		let view_element = declared_element_type(view);
		let is_list_view = view_element.is_some() || annotated_kind(view) == Some(Kind::List);
		if !is_list_view || self.view_fits(&list_type.element, view_element) {
			return None;
		}
		let declared = list_type.written;
		let message = format!("{view_name} changes {list}, which is declared {declared}: it could add an item of another type, and {list} shares it (P215)");
		Some(Diagnostic::at(at, message).fix(format!("declare it {declared}, or hand it a copy of {list}")))
	}

	/// Does every item the view admits fit the element type `element`? An untyped view (`xs: list`) admits any item;
	/// a class view admits its subclasses, whose fields start with its own (W0's `cls p ≤ cls q`)
	fn view_fits(&self, element: &str, view_element: Option<&str>) -> bool {
		let Some(view_element) = view_element else { return false };
		let fields = |class: &str| self.class_fields.get(class).map(|fields| fields.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>());
		match (fields(element), fields(view_element)) {
			(Some(element_fields), Some(view_fields)) => view_fields.starts_with(&element_fields),
			_ => view_element == element || elements_fit(element, &format!("{LIST_OF_PREFIX}{view_element}")),
		}
	}
}
