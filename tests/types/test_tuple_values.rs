// A comma tuple `(y, 4)` is a value of its elements: assigned whole (not the call y(4)) and compared element by element
use warp::is;

#[test]
fn a_tuple_with_a_variable_is_assigned_whole() {
	is!("y=1; z = (y, 4); z#2", 4);
	is!("y = 2.5; z = (y, 4); z#1", 2.5);
}

#[test]
fn tuples_compare_element_by_element() {
	is!("(1,2) == (3,2)", 0);
	is!("(1,2) != (1,3)", 1);
	is!("(2 as float, 4.3 as int) == (2.0, 4)", 1);
	is!("y = 2 as float; (y, 4) == (2.0, 4)", 1);
}
