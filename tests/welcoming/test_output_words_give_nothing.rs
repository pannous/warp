//! card library-signature (decision output-words-nothing): the output words give nothing, as their prelude lines say,
//! so a braceless call of one taking arithmetic has one reading: no "taking the whole expression" warning
use crate::common::warnings_of;

#[test]
fn an_output_word_taking_arithmetic_does_not_warn() {
	for code in ["puts \"a\" + \"b\"", "puti 1 + 2", "putf 1.5 + 2.0"] {
		let (_, warnings) = warnings_of(code);
		assert!(
			warnings.iter().all(|warning| !warning.contains("taking the whole expression")),
			"{code}: {warnings:?}"
		);
	}
}
