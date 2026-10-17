// card lists-maps: `a ≈ b` of lists, maps, pairs and instances is lib/prelude.warp's alike, entry by entry with ≈ again;
// two plain numbers compare inline, their module has none of alike's runtime functions
use crate::is;

fn compiled_mentions(code: &str, name: &str) -> bool {
	let module = warp::pipeline::compile(code).expect("compiles").bytes;
	module.windows(name.len()).any(|window| window == name.as_bytes())
}

#[test]
fn pairs_and_nested_entries_are_alike_part_by_part() {
	is!("(a:1) ≈ (a:1.0000000001)", true);
	is!("(a:1) ≈ (b:1)", false);
	is!("[{x:[1 \"Hí\"]}] ≈ [{x:[1.0000000001 \"hi\"]}]", true);
	is!("tolerance = 0.1; m = {a: [1 2]}; m ≈ {a: [1.05 2]}", true);
	is!("[1 2] ≈ (1 2)", false);
}

#[test]
fn a_program_word_named_alike_leaves_the_prelude_word_be() {
	is!("alike(x) := x * 2; [alike(2)] ≈ [4.0000000001]", true);
}

#[test]
fn numbers_compare_inline_without_alike() {
	assert!(!compiled_mentions("x = 3; x ≈ 3", "values_similar"));
	assert!(!compiled_mentions("x = 3; x ≈ 3", "text_fold"));
	assert!(compiled_mentions("xs = [3]; xs ≈ [3]", "prelude·alike"));
}
