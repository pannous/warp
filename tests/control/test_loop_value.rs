// The value of a loop is its last body value: the step a `for` loop counts with is not part of it
use crate::is;

#[test]
fn a_counting_loop_gives_its_last_body_value() {
	is!("for i in 1 to 5 : {i*10}", 50);
	is!("for i in 1 to 5 : {puti(i)}", 5);
	is!("for i in 1 to 3 {puti i}; i", 4); // the counter after the loop
}

#[test]
fn a_walking_loop_gives_its_last_body_value() {
	is!("for i in [1,2,3] : i*10", 30);
}

#[test]
fn a_while_loop_of_one_expression_gives_its_value() {
	is!("i=1; while (i+=1)<=5 {i*10}", 50);
}
