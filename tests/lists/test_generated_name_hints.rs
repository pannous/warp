// Card internal-name: a hint names only what the user wrote, never a temporary the compiler made (the list a
// comprehension or `where` builds was taught as `prefer var comprehension_list_0 over let comprehension_list_0`)
use crate::is;

fn hint_texts(code: &str) -> Vec<String> {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().map(|hint| format!("{} → {}: {}", hint.original, hint.canonical, hint.reason)).collect()
}

#[test]
fn comprehension_and_where_give_no_hint_about_their_list() {
	for code in ["[x * 2 for x in [1 2 3] if x > 1]", "xs = [1 2 3]; xs where it > 1"] {
		let hints = hint_texts(code);
		assert!(hints.iter().all(|hint| !hint.contains("comprehension")), "{code}: {hints:?}");
	}
	is!("[x * 2 for x in [1 2 3] if x > 1]", warp::warp_parser::parse("[4 6]"));
}
