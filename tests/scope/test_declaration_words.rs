//! card cleanup-closed-lists: every declaration word (let, var, const, val, final …, analyzer is_declaration_word)
//! makes a function's own local, so `const x = 2` inside f asks no local-or-global question
use crate::common::warnings_of;
use warp::Node;

#[test]
fn every_declaration_word_declares_a_local() {
	for word in ["let", "var", "const", "val", "final", "constant"] {
		let code = format!("x = 1\nf() := {{ {word} x = 2; x }}\nf() + x");
		let (value, warnings) = warnings_of(&code);
		assert_eq!(value, Node::from(3), "{code}");
		assert!(!warnings.iter().any(|warning| warning.contains("make a new local")), "{code}: {warnings:?}");
	}
}
