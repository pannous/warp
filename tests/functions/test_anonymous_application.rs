//! Wiki row 23 (lambda.md, symbolism.md): an anonymous function applied where it stands, to a juxtaposed argument or a
//! list, like a named one: `{it*2} 3` → 6, `{it^2}[1 2 3]` → [1 4 9] (functions broadcast, row 24)
use warp::is;
use warp::wasp_parser::parse;

#[test]
fn test_a_block_applied_to_a_juxtaposed_argument() {
	is!("{it*2} 3", 6);
	is!("(x => x*2) 4", 8);
}

#[test]
fn test_an_anonymous_function_broadcasts_over_a_list() {
	is!("({it^2}[1 2 3])#3", 9);
	is!("{it*it}[1 2 3]", parse("[1 4 9]"));
	is!("(x => x*2)[1 2 3]", parse("[2 4 6]"));
	is!("{it*2}([1 2 3])", parse("[2 4 6]"));
}

#[test]
fn test_a_data_block_is_not_applied() {
	is!("x = {1 2}; count x", 2);
}
