//! card case-table-warp: lib/case_table.warp maps upper, lower and fold in warp over the generated case tables
//! (lib/case_tables.warp), giving what the builtins upper and lower give
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

/// the cased scripts: Latin to Armenian, Georgian, Cherokee, Latin and Greek extended, Glagolitic and Coptic, Cyrillic
/// and Latin extended, fullwidth, Deseret
const CASED_BLOCKS: [(u32, u32); 8] = [
	(0x41, 0x590),
	(0x10A0, 0x1100),
	(0x13A0, 0x1400),
	(0x1E00, 0x2000),
	(0x2C00, 0x2D30),
	(0xA640, 0xA800),
	(0xFF21, 0xFF5B),
	(0x10400, 0x10450),
];

// the generated tables (scripts/case_tables.py, Python's Unicode) map every letter of the cased scripts as the builtins
#[test]
fn case_words_match_the_builtins_on_every_cased_letter() {
	let letters: String = CASED_BLOCKS
		.iter()
		.flat_map(|&(start, end)| (start..end).filter_map(char::from_u32))
		.filter(|letter| letter.is_alphabetic())
		.collect();
	is!(
		&format!("use case_table; case_upper(\"{letters}\") == \"{letters}\".upper and case_lower(\"{letters}\") == \"{letters}\".lower"),
		true
	);
}
