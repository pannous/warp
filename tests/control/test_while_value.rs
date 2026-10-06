use crate::is;
use warp::*;

#[test]
fn test_while_value_is_last_body_value() {
	is!("x=0;while x<3: x+=1", 3);
	is!("x=0;while x<3 { x+=2 }", 4);
}

#[test]
fn test_while_value_of_untouched_loop_is_empty() {
	is!("x=5;while x<3: x+=1", Empty);
}
