// The user's issues #11, #12, #13: forms they consider legitimate get no hint: single quotes ('faster'), `let t = …`,
// `str(x)` and a number joining a text ("Contains 40? " + n)
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

fn hints_of(code: &str) -> Vec<String> {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().map(|hint| format!("{} → {}", hint.original, hint.canonical)).collect()
}

#[test]
fn legitimate_forms_get_no_hint() {
	for code in ["x = 'faster'; x", "let t = 1000; t", "n = 3; print \"Contains 40? \" + n",
		"height(t) := 3; print \"Height:   \" + str(height(1))", "String(4)"] {
		assert_eq!(hints_of(code), Vec::<String>::new(), "{code}");
	}
}
