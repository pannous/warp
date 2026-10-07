// card web-bundle: upper and lower map every code point as Rust's char::to_uppercase / to_lowercase do (user decision
// #26), through a table of ranges (text_unicode.rs): runs, alternating pairs, 4-byte code points, multi-character and
// titlecase mappings
use warp::wasm_emitter::eval;

/// Latin-1, Latin Extended-A pairs, titlecase ǅ, İ (lower is two code points), Greek with ΐ (upper is three), Cyrillic,
/// Armenian, Georgian, Cherokee, fullwidth, Deseret (4 bytes), Adlam, and characters without case
const SAMPLE: &str = "Hello ÀÉÎõü ĀāĂăĐđŁł ǄǅǆǱ İı ΐΑβΓδΣςω ДЖЯжя ԱբՖ ႠႡⴀ ᏣᏓᏴ ꭰ ＡＢｃ 𐐀𐐨 𞤀𞤢 ß ŉ 123 中文 ✓";

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn upper_and_lower_map_as_rust_does() {
	assert_eq!(shown(&format!("\"{SAMPLE}\".upper")), format!("\"{}\"", SAMPLE.to_uppercase()));
	assert_eq!(shown(&format!("\"{SAMPLE}\".lower")), format!("\"{}\"", SAMPLE.to_lowercase()));
}

// every code point whose case changes, in one text (the escapes the parser reads are left out); the parser keeps a
// text in its composed form (U+1FBB is U+0386), so the expected values come from the text as the program holds it
#[test]
fn every_cased_code_point_maps_as_rust_does() {
	let cased: String = (0..=char::MAX as u32).filter_map(char::from_u32)
		.filter(|letter| letter.to_uppercase().ne([*letter]) || letter.to_lowercase().ne([*letter]))
		.filter(|letter| !matches!(letter, '"' | '\\'))
		.collect();
	let warp::Node::Text(held) = eval(&format!("\"{cased}\"")) else { panic!("a text") };
	assert_eq!(shown(&format!("\"{cased}\".upper")), format!("\"{}\"", held.to_uppercase()));
	assert_eq!(shown(&format!("\"{cased}\".lower")), format!("\"{}\"", held.to_lowercase()));
}
