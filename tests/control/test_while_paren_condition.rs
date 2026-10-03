use warp::*;

#[test]
fn test_while_parenthesized_condition_takes_a_bare_body() {
	is!("i=1;while(i<9)i++;i+1", 10);
}

#[test]
fn test_while_parenthesized_condition_takes_a_bare_body_with_space() {
	is!("i=1;while (i<9) i++;i+1", 10);
}
