// Card hard-coded: the near-miss warning knows the language's words from the registry of builtin methods
// (members::MEMBERS_BY_TYPE), not from a hard-coded list of six words
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn near_miss_suggestion(code: &str) -> Option<String> {
	take_warnings();
	eval(code);
	take_warnings().into_iter().map(|warning| warning.message).find(|message| message.contains("did you mean"))
}

#[test]
fn test_a_typo_of_a_builtin_method_warns() {
	for (typo, method) in [("revrse", "reverse"), ("filtr", "filter"), ("uppr", "upper"), ("kees", "keys")] {
		let suggestion = near_miss_suggestion(typo).unwrap_or_default();
		assert!(suggestion.contains(&format!("did you mean {method}")), "{typo}: {suggestion}");
	}
}

#[test]
fn test_every_builtin_method_is_offered_once() {
	let words = warp::members::builtin_member_words();
	assert!(words.contains(&"reverse") && words.contains(&"keys") && words.contains(&"round"));
	assert_eq!(words.iter().filter(|word| **word == "count").count(), 1);
}
