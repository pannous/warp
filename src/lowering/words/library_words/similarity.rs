//! `a ≈ b` and `a ~ b`: compared within a relative tolerance
use super::*;
use crate::node::parens;

/// values_similar(a, b, tolerance): `a ≈ b` (wasm_emitter/text/similarity.rs)
pub const VALUES_SIMILAR: &str = "values_similar";
/// values_rough(a, b, tolerance): `a ~ b`, looser (P211)
pub const VALUES_ROUGH: &str = "values_rough";

/// Per level of approximation: its operator, its call, the variable a program sets its relative tolerance with
/// (|a-b| ≤ tolerance·max(|a|, |b|)) and the default tolerance
pub const SIMILARITY_LEVELS: [(Op, &str, &str, &str); 2] =
	[(Op::Similar, VALUES_SIMILAR, "tolerance", "1e-9"), (Op::Rough, VALUES_ROUGH, "rough_tolerance", "0.01")];

/// values_similar or values_rough
pub fn is_similarity_call(name: &str) -> bool {
	SIMILARITY_LEVELS.iter().any(|(_, function, ..)| *function == name)
}

impl Lowering {
	/// `a ≈ b`, `a ~ b`, `a circa b`: compared within the relative tolerance; a side that is more than a plain value or
	/// arithmetic is computed once into a temporary
	/// `a ≈ b`: the emitter's values_similar(a, b, tolerance), numbers inline, any other values field by field
	pub(super) fn lower_similar(&self, op: Op, left: Node, right: Node) -> Node {
		let (_, function, variable, default) =
			SIMILARITY_LEVELS.iter().find(|(level, ..)| *level == op).expect("a similarity operator");
		let tolerance = if self.shadows(variable) { variable } else { default };
		let call = vec![symbol(function), left, right, crate::warp_parser::parse_generated(tolerance)];
		parens(call)
	}
}
