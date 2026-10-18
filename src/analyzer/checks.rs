//! Analysis and its diagnostics: analyze and diagnose run the checks, each in its file under checks/

use super::*;

mod type_names;
pub use type_names::*;
mod lints;
pub use lints::*;
mod declared_types;
pub use declared_types::*;
mod calls;
mod comparisons;
mod constants;
pub mod discarded_values;
mod function_names;
mod kind_changes;
pub mod match_exhaustive;
mod nulls;
pub mod unit_names;
pub mod unresolved_blocks;
pub(crate) use calls::*;
pub(crate) use comparisons::*;
pub(crate) use constants::*;
pub use function_names::*;
pub(crate) use kind_changes::*;
pub(crate) use nulls::*;

pub fn analyze(raw: Node) -> Node {
	let mut scope = Scope::new();
	if let Some(err) = check_type_errors(&raw, &mut scope) {
		return err;
	}
	raw
}

/// Semantic checks run before emission; the first violation comes back as an error value
pub fn diagnose(program: &Node) -> Option<Node> {
	check_type_word_functions(program)
		.or_else(|| check_parameter_annotations(program))
		.or_else(|| check_variable_annotations(program))
		.or_else(|| check_union_fields(program))
		.or_else(|| check_declared_types(program, &mut HashMap::new(), &mut Scope::new(), &declared_classes(program)))
		.or_else(|| super::list_views::check_widened_list_views(program))
		.or_else(|| check_constants(program, &mut HashMap::new()))
		.or_else(|| check_null_use(program, &mut HashMap::new(), &list_variables(program)))
		.or_else(|| check_subjectless_comparison(program))
		.or_else(|| check_ambiguous_calls(program))
		.or_else(|| check_call_arity(program))
		.map(Diagnostic::into_error)
}
