// A case of a plain enum is a word the program defines: `Op.Add` and `Bracket.Round` near a library word (add, round)
// name the case, no "names nothing … did you mean" (the compiler's own sources warned at every Op.Add, Bracket.Round,
// Op.Define); a typo still warns
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings()
		.into_iter()
		.map(|warning| warning.message)
		.filter(|message| message.contains("names nothing"))
		.collect()
}

#[test]
fn enum_cases_near_library_words_warn_nothing() {
	let declared = "enum Op { Add, Sub, Define, Sqrt }\nenum Bracket { Curly, Round }\n";
	for used in ["Op.Add", "Bracket.Round", "Op.Define", "[Op.Sqrt, Op.Sub]"] {
		assert_eq!(warnings_of(&format!("{declared}x = {used}\nx")), Vec::<String>::new(), "{used}");
	}
	assert!(!warnings_of("revrse").is_empty());
}
