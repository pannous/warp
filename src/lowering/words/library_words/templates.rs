//! Word templates: placeholders substituted, hidden temporaries named apart
use super::*;

/// `try_tmp_value` of a try template becomes the temporary `try·value·3`
const TRY_TEMPORARY: &str = "try_tmp_";
const TRY_BASE: &str = "try";

/// A compiler temporary's name, `word·sum·3`: the program never writes `·` in a name (`a·b` is a product), so the
/// scope checks pass it by (analyzer::is_compiler_temporary)
pub(crate) fn temporary_name(parts: &[&str]) -> String {
	parts.join(crate::analyzer::TEMPORARY_SEPARATOR)
}

/// A parsed template's temporaries named apart: `prefix_part` becomes `base·part·number`
pub(crate) fn named_apart(node: Node, prefix: &str, base: &str, number: usize) -> Node {
	match node {
		Node::Symbol(name) if name.starts_with(prefix) => {
			Node::Symbol(temporary_name(&[base, &name[prefix.len()..], &number.to_string()]))
		}
		other => other.map_children(|child| named_apart(child, prefix, base, number)),
	}
}

pub(crate) fn substitute(node: Node, placeholder: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == placeholder => replacement.clone(),
		other => children_rewritten(other, |child| substitute(child, placeholder, replacement)),
	}
}

impl Lowering {
	/// A source template with `hidden` variables made unique, its placeholders replaced by the given nodes
	pub(super) fn instantiate_template(&self, template: &str, replacements: &[(&str, &Node)]) -> Node {
		let mut program = named_apart(
			parse_template(template, replacements.first().map_or(&Node::Empty, |(_, at)| *at)),
			TRY_TEMPORARY,
			TRY_BASE,
			self.next_temporary(),
		);
		for (placeholder, replacement) in replacements {
			program = substitute(program, placeholder, replacement);
		}
		program
	}
}
