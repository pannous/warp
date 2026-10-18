//! Fields of a class's own type (card empty-field)

use crate::errors::diagnostic::Diagnostic;
use crate::lowering::class_methods::field_items;
use crate::node::Node;

/// `class Tree{left: Tree}`: a field of the class's own type without `?` could never be filled, every Tree needing a
/// Tree first (card empty-field, user decision: strict, the fix `left: Tree?`)
pub fn check_self_typed_fields(program: &Node) -> Option<Diagnostic> {
	let mut unfillable = None;
	program.visit(&mut |part| {
		let Node::Type { name, body } = part else { return };
		let class = name.drop_meta().name();
		let field = field_items(body).into_iter().find(|(_, _, type_name)| *type_name == class);
		if let Some((field, field_name, _)) = field.filter(|_| unfillable.is_none()) {
			unfillable = Some(
				Diagnostic::at(
					&field,
					format!("{field_name} can never be filled: every {class} needs a {class} first"),
				)
				.fix(format!("{field_name}: {class}?")),
			);
		}
	});
	unfillable
}
