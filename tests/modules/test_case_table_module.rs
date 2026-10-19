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

/// The ints of the list `name = [ … ]` of lib/case_tables.warp
fn table_numbers(name: &str) -> Vec<i64> {
	let source = include_str!("../../lib/case_tables.warp");
	let start = source.find(&format!("{name} = [")).expect("the table") + name.len() + 4;
	let end = start + source[start..].find(']').expect("its end");
	source[start..end].split(',').map(|number| number.trim().parse().expect("an int")).collect()
}

/// The code points `code` maps to by `ranges` (sorted by start): itself when no range covers it
fn mapped_by_table(ranges: &[&[i64]], code: i64) -> Vec<i64> {
	let before = ranges.partition_point(|range| range[0] <= code);
	match before.checked_sub(1).map(|at| ranges[at]) {
		Some(&[start, last, stride, delta, second, third]) if code <= last && (code - start) % stride == 0 => {
			let mut mapped = vec![code + delta];
			mapped.extend([second, third].into_iter().filter(|&more| more != 0));
			mapped
		}
		_ => vec![code],
	}
}

/// The code points of Rust's mapping
fn codes(mapped: impl Iterator<Item = char>) -> Vec<i64> {
	mapped.map(|part| i64::from(u32::from(part))).collect()
}

// the generated tables (scripts/case_tables.py, Python's Unicode) map every code point as Rust's char does
#[test]
fn generated_tables_match_rust_case_mapping() {
	let (upper, lower) = (table_numbers("upper_case_ranges"), table_numbers("lower_case_ranges"));
	let (upper, lower): (Vec<&[i64]>, Vec<&[i64]>) = (upper.chunks(6).collect(), lower.chunks(6).collect());
	for letter in (0..=char::MAX as u32).filter_map(char::from_u32) {
		let code = i64::from(u32::from(letter));
		assert_eq!(mapped_by_table(&upper, code), codes(letter.to_uppercase()), "upper of U+{code:04X}");
		assert_eq!(mapped_by_table(&lower, code), codes(letter.to_lowercase()), "lower of U+{code:04X}");
	}
}
