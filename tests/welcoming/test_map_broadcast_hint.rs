// card hint-map (user 2026-10-15): a named function of one value maps over a list by itself (wiki/Broadcasting.md),
// so `map` of it hints the broadcast form in every spelling; a word taking the list whole or a lambda gets no hint
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

/// The (written, preferred) pairs of the broadcast hints of `code`
fn broadcast_hints(code: &str) -> Vec<(String, String)> {
	// a hint is said once per thread: another test compiling the same code first would silence it
	warp::normalize::clear_shown_hints();
	let (_, hints) = capture_hints(|| eval(code));
	hints
		.into_iter()
		.filter(|hint| hint.reason.contains("maps over a list by itself"))
		.map(|hint| (hint.original, hint.canonical))
		.collect()
}

fn hinted(written: &str, preferred: &str) -> Vec<(String, String)> {
	vec![(written.to_string(), preferred.to_string())]
}

#[test]
fn the_method_form_is_hinted() {
	assert_eq!(broadcast_hints("square := it*it; [1 2].map(square)"), hinted("[1 2].map(square)", "square [1 2]"));
	assert_eq!(broadcast_hints("[-1 2].map(abs)"), hinted("[-1 2].map(abs)", "abs [-1 2]"));
}

#[test]
fn the_call_form_is_hinted() {
	assert_eq!(broadcast_hints("map([\"ab\"], upper)"), hinted("map([\"ab\"], upper)", "upper [\"ab\"]"));
	assert_eq!(broadcast_hints("sq(x) := x*x; map(sq, [1 2])"), hinted("map(sq, [1 2])", "sq [1 2]"));
}

#[test]
fn the_phrase_forms_are_hinted() {
	assert_eq!(broadcast_hints("map abs [-1 2]"), hinted("map abs [-1 2]", "abs [-1 2]"));
	assert_eq!(broadcast_hints("map floor on [1.5]"), hinted("map floor on [1.5]", "floor [1.5]"));
	assert_eq!(
		broadcast_hints("square := it*it; map square on: [1 2]"),
		hinted("map square on: [1 2]", "square [1 2]")
	);
}

#[test]
fn a_function_taking_the_list_whole_is_not_hinted() {
	assert_eq!(broadcast_hints("[[1 2]].map(reverse)"), vec![]);
	assert_eq!(broadcast_hints("f(xs: list) := count xs; map([[1] [2 3]], f)"), vec![]);
	assert_eq!(broadcast_hints("g(xs) := xs#1; map([[1] [2 3]], g)"), vec![]);
}

/// `p xs` prints the list once, the map prints each item: only where `f xs` broadcasts does the hint exist
#[test]
fn a_function_that_does_not_broadcast_is_not_hinted() {
	assert_eq!(broadcast_hints("p(x) := print x; xs = [1 2]; xs.map(p)"), vec![]);
	assert_eq!(broadcast_hints("h(x) := count(x); map([[1] [2 3]], h)"), vec![]);
}

#[test]
fn a_lambda_or_a_program_s_own_map_is_not_hinted() {
	assert_eq!(broadcast_hints("map([1 2], x => x+1)"), vec![]);
	assert_eq!(broadcast_hints("def map(list, fn) { list }; square := it*it; map([1], square)"), vec![]);
}

#[test]
fn a_mapped_function_still_maps() {
	assert_eq!(eval("square := it*it; map square on: [1 2]").serialize(), "[1 4]");
}
