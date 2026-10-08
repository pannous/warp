// Card g_YiSA (user): the hint toward `xs#2` over `xs[1]` only for a number, where it is as short; `hidden_bias[j]`
// would become the longer `hidden_bias#(j+1)`, so a name gets no hint
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasp_parser::WaspParser;

fn index_hints(code: &str) -> Vec<String> {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| WaspParser::parse(code));
	hints.into_iter().filter(|hint| hint.reason.contains("indexing")).map(|hint| hint.original).collect()
}

#[test]
fn only_a_number_index_gets_the_hash_hint() {
	assert_eq!(index_hints("hidden_bias=[1 2 3]; hidden_bias[1]"), vec!["hidden_bias[1]"]);
	assert_eq!(index_hints("hidden_bias=[1 2 3]; j=0; hidden_bias[j]"), Vec::<String>::new());
}
