//! Unit words in patterns (card pattern-unit-name): `x is {host: h}` compares the field with the unit hour, it binds
//! nothing

use crate::errors::diagnostic::Diagnostic;
use crate::lowering::switch::{is_list_pattern, is_map_pattern, map_field};
use crate::node::Node;
use crate::operators::Op;

/// `x is {host: h}` compares host with the unit hour and binds nothing: a weak warning naming the way to bind, `$h`
/// or another name (card pattern-unit-name)
pub fn check_unit_names(program: Node) -> Node {
	let mut warnings = vec![];
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Eq, pattern) = node.drop_meta() {
			unit_names(pattern, &program, &mut warnings);
		}
	});
	match crate::errors::diagnostic::report(&warnings) {
		Err(error) => error,
		Ok(()) => program,
	}
}

/// The unit words that are field values of a map pattern, at any depth; a list item `[_, h]` binds h
fn unit_names(pattern: &Node, program: &Node, warnings: &mut Vec<Diagnostic>) {
	match pattern.drop_meta() {
		Node::List(items, _, _) if is_list_pattern(pattern) => {
			items.iter().for_each(|item| unit_names(item, program, warnings))
		}
		Node::List(fields, _, _) if is_map_pattern(pattern) => {
			// a shorthand `{h}` binds the field h
			let values =
				fields.iter().filter_map(map_field).filter(|(name, part)| part.symbol_name() != Some(name.as_str()));
			for (_, part) in values {
				let unit = part
					.symbol_name()
					.and_then(|name| Some((name, crate::numbers::units::unit_long_name(name, program)?)));
				match unit {
					Some((name, unit)) => warnings.push(
						Diagnostic::at(
							part,
							format!("{name} is the unit {unit}: this compares; bind with ${name} or another name"),
						)
						.weak(),
					),
					None => unit_names(part, program, warnings),
				}
			}
		}
		_ => {}
	}
}
