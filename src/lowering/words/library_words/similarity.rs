//! `a ≈ b` and `a ~ b`: compared within a relative tolerance
use super::*;
use crate::node::prelude_call;

/// values_similar(a, b, tolerance): `a ≈ b` of two values that are no list, map, pair or instance
/// (wasm_emitter/text/similarity.rs)
pub const VALUES_SIMILAR: &str = "values_similar";
/// values_rough(a, b, tolerance): the same for `a ~ b`, looser (P211)
pub const VALUES_ROUGH: &str = "values_rough";
/// lib/prelude.warp's alike(a, b, tolerance, rough): `a ≈ b` and `a ~ b` of any two values
pub const ALIKE_WORD: &str = "alike";
/// alike by its name in the program, which the emitter matches: two plain numbers it compares inline
pub const ALIKE_FUNCTION: &str = "prelude·alike";

/// Per level of approximation: its operator, its runtime function, the variable a program sets its relative tolerance
/// with (|a-b| ≤ tolerance·max(|a|, |b|)) and the default tolerance
pub const SIMILARITY_LEVELS: [(Op, &str, &str, &str); 2] =
	[(Op::Similar, VALUES_SIMILAR, "tolerance", "1e-9"), (Op::Rough, VALUES_ROUGH, "rough_tolerance", "0.01")];

/// values_similar or values_rough
pub fn is_similarity_call(name: &str) -> bool {
	SIMILARITY_LEVELS.iter().any(|(_, function, ..)| *function == name)
}

/// The prelude word `a ≈ b` and `a ~ b` call, for modules::resolve, which runs before they are lowered
pub(crate) fn implied_similarity_words(statements: &[Node]) -> Vec<String> {
	let mut words = Vec::new();
	for statement in statements {
		statement.visit(&mut |node| {
			if matches!(node, Node::Key(_, Op::Similar | Op::Rough, _)) && words.is_empty() {
				words.push(ALIKE_WORD.to_string());
			}
		});
	}
	words
}

impl Lowering {
	/// `a ≈ b`, `a ~ b`, `a circa b`: the prelude's alike(a, b, tolerance, rough), whose tolerance is the program's
	/// variable when it sets one; the emitter compares two plain numbers inline instead
	pub(super) fn lower_similar(&self, op: Op, left: Node, right: Node) -> Node {
		let (_, _, variable, default) =
			SIMILARITY_LEVELS.iter().find(|(level, ..)| *level == op).expect("a similarity operator");
		let tolerance = if self.shadows(variable) { variable } else { default };
		let rough = if op == Op::Rough { Node::True } else { Node::False };
		prelude_call(ALIKE_WORD, vec![left, right, crate::warp_parser::parse_generated(tolerance), rough])
	}
}
