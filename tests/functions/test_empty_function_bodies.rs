// Card function-whose: a function whose body is ø compiles and leaves the rest of the program alone
use crate::is;
use warp::Node::Empty;

#[test]
fn a_function_with_an_empty_body_leaves_the_program_alone() {
	is!("f() := ø; 3", 3);
	is!("f() := ø\n3", 3);
	is!("compute() := emit ask; 3", 3);
}

#[test]
fn calling_a_function_with_an_empty_body_gives_empty() {
	is!("f() := ø; f(); 3", 3);
	is!("f() := ø; f()", Empty);
}
