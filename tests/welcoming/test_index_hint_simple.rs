// Card g-2WPo (user): the hint toward `xs#1` over `xs[1]` only for a simple index (a literal or a name), not for a
// computed one like `xs[i+1]`
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasp_parser::WaspParser;

fn index_hints(code: &str) -> Vec<String> {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| WaspParser::parse(code));
	hints.into_iter().filter(|hint| hint.reason.contains("indexing")).map(|hint| hint.original).collect()
}

#[test]
fn only_a_simple_index_gets_the_hash_hint() {
	assert_eq!(index_hints("xs=[1 2 3]; xs[1]"), vec!["xs[1]"]);
	// card g_YiSA (user): a name gets no hint either, `xs#(i+1)` is longer (test_index_hint_numbers.rs)
	assert_eq!(index_hints("xs=[1 2 3]; i=0; xs[i]"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1 2 3]; i=0; xs[i+1]"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1 2 3]; n=3; xs[n/2]"), Vec::<String>::new());
}
