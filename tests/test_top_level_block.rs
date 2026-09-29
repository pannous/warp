use warp::*;

#[test]
fn test_top_level_semicolons_yield_the_last_item() {
	is!("1;2;3", 3);
	is!("'hello';(1 2 3 4);10", 10);
}

#[test]
fn test_top_level_newlines_yield_the_last_item() {
	is!("1\n2\n3", 3);
}
