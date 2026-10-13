//! `|` in the key of a match arm separates alternatives (user, matching roadmap): no hint toward bit_or or `or` there;
//! outside an arm `x | 4` of whole numbers is the bit or (card operator-shall), only `|` of truth values hints
use warp::normalize::*;
use warp::warp_parser::WarpParser;

fn hinted_originals(code: &str) -> Vec<String> {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| WarpParser::parse(code));
	hints.into_iter().map(|hint| hint.original).collect()
}

#[test]
fn arm_alternatives_get_no_hint() {
	for code in [
		"match 2 { 1 | 2 => \"small\"; _ => \"big\" }",
		"match 2 { 1 | 2 | 3 => \"small\"; _ => \"big\" }",
		"f(v) := match v { \"a\" | \"b\" => 1; _ => 0 }",
	] {
		assert_eq!(hinted_originals(code), Vec::<String>::new(), "hints for {code}");
	}
}

#[test]
fn outside_an_arm_only_a_bar_of_truth_values_hints() {
	assert_eq!(hinted_originals("x = 3; x | 4"), Vec::<String>::new());
	assert_eq!(hinted_originals("x = yes; x | no"), vec!["|".to_string()]);
}
