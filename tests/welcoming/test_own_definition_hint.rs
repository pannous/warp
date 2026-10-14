// card hint-own-definition: `parse_json(t)` written by hand is noted as warp's `json.parse(t)`, but not where the
// program defines its own parse_json (`def parse_json(text) {…}`), nor for a library's own calls (lib/extra/netbase.warp)
use warp::errors::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

fn json_parse_notes(code: &str) -> usize {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.canonical == "json.parse").count()
}

#[test]
fn the_program_s_own_parse_json_gets_no_note() {
	let path = "scratch/test_own_definition_hint.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(json_parse_notes("def parse_json(text) { text }\nparse_json(\"1\")"), 0);
		assert_eq!(json_parse_notes("use netbase\n1"), 0);
		assert_eq!(json_parse_notes("parse_json(\"[1]\")"), 1);
	});
	let _ = std::fs::remove_file(path);
}
