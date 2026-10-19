//! card case-table-warp: lib/case_table.warp maps upper, lower and fold in warp over the generated case tables
//! (lib/case_tables.warp), the twin of the builtins of src/wasm_emitter/text/text_unicode.rs
use crate::is;
use warp::wasm_emitter::eval;

/// runs, alternating pairs, ß (upper is two), ΐ (upper is three), İ (lower is two), Deseret (4 bytes), no case
const SAMPLE: &str = "Hello ÀÉõü ĀāŁł ǅ İ ΐΑβΣ ДЖя ＡＢｃ 𐐀𐐨 ß 123 中文";

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn case_words_in_warp() {
	is!("use case_table; case_upper(\"straße\")", "STRASSE");
	is!("use case_table; case_lower(\"ÄBC\")", "äbc");
	is!("use case_table; case_fold(\"Élan\")", "elan");
	is!("use case_table; case_upper(\"123 中文\")", "123 中文");
}

#[test]
fn case_words_match_the_builtins() {
	for (word, builtin) in [("case_upper", "upper"), ("case_lower", "lower")] {
		assert_eq!(shown(&format!("use case_table; {word}(\"{SAMPLE}\")")), shown(&format!("\"{SAMPLE}\".{builtin}")));
	}
}
