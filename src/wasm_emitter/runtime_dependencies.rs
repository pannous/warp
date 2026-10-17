//! Which runtime functions call which: each module lists what its functions need (`dependencies()`), and a required
//! function brings the functions it needs, theirs in turn, until nothing new comes

use std::collections::HashSet;

/// When the functions of `when` are required (any one of them, or all with `all`), the functions of `then` are too
pub(in crate::wasm_emitter) struct Dependency {
	when: Vec<&'static str>,
	all: bool,
	then: Vec<&'static str>,
}

/// Any function of `when` needs the functions of `then`
pub(in crate::wasm_emitter) fn any(when: &[&'static str], then: &[&'static str]) -> Dependency {
	Dependency { when: when.to_vec(), all: false, then: then.to_vec() }
}

/// The functions of `when` together need the functions of `then`
pub(in crate::wasm_emitter) fn all(when: &[&'static str], then: &[&'static str]) -> Dependency {
	Dependency { all: true, ..any(when, then) }
}

/// The functions of `then`, needed by every program
pub(in crate::wasm_emitter) fn always(then: &[&'static str]) -> Dependency {
	all(&[], then)
}

impl Dependency {
	fn holds(&self, required: &HashSet<&'static str>) -> bool {
		let is_required = |name: &&str| required.contains(name);
		if self.all {
			self.when.iter().all(is_required)
		} else {
			self.when.iter().any(is_required)
		}
	}
}

/// The dependencies of every module's runtime functions
pub(in crate::wasm_emitter) fn every_dependency() -> Vec<Dependency> {
	use super::{host, lists, numbers, reflection, text};
	[
		host::component_adapters::dependencies(),
		reflection::node_api::dependencies(),
		numbers::uncertain::dependencies(),
		lists::list_ops::node_arithmetic_dependencies(),
		lists::list_ops::type_test_dependencies(),
		text::similarity::dependencies(),
		text::text_builtins::dependencies(),
	]
	.into_iter()
	.flatten()
	.collect()
}

/// Add to `required` what its functions need, and what those need, until nothing new comes
pub(in crate::wasm_emitter) fn require_dependencies(required: &mut HashSet<&'static str>, dependencies: &[Dependency]) {
	loop {
		let before = required.len();
		for dependency in dependencies.iter().filter(|dependency| dependency.holds(required)).collect::<Vec<_>>() {
			required.extend(&dependency.then);
		}
		if required.len() == before {
			return;
		}
	}
}
