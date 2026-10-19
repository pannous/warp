//! `byte_at` rejects non-text values before reading their byte storage (card traps-wasm-trap).

use crate::common::fails_with;

#[test]
fn byte_at_rejects_a_non_text_argument() {
	fails_with("byte_at(3, 0)", "not a text");
}
