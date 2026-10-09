// card unit-glue: a unit is written directly on its number, `3km`; written apart, `3 km` still works with a soft hint
// (user 2026-10-09: "units should be written directly to the numbers, but that's a soft hint")
use warp::normalize::{capture_hints, clear_shown_hints, CapturedHint};
use warp::wasm_emitter::eval;

fn hints_of(code: &str) -> (String, Vec<CapturedHint>) {
	clear_shown_hints();
	let (value, hints) = capture_hints(|| eval(code));
	(value.serialize().trim().to_string(), hints)
}

fn glue_hints(hints: &[CapturedHint]) -> Vec<(String, String)> {
	hints.iter().filter(|hint| hint.reason.contains("unit")).map(|hint| (hint.original.clone(), hint.canonical.clone())).collect()
}

#[test]
fn a_unit_apart_from_its_number_hints_the_glued_form() {
	let (value, hints) = hints_of("3 km + 2 m");
	assert_eq!(value, "3002m");
	let glue = glue_hints(&hints);
	assert!(glue.contains(&("3 km".to_string(), "3km".to_string())), "{hints:?}");
	assert!(glue.contains(&("2 m".to_string(), "2m".to_string())), "{hints:?}");
}

#[test]
fn a_glued_unit_or_a_spaced_word_has_no_glue_hint() {
	for code in ["3km + 2m", "1500 meters", "x = 2; 3 * x"] {
		let (_, hints) = hints_of(code);
		assert!(glue_hints(&hints).is_empty(), "{code}: {hints:?}");
	}
}
