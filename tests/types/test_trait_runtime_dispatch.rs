// A declared trait operation on a value whose type is known only at run time (an element of a mixed list, a loop
// variable) calls the witness of the value's type through a generated dispatcher
use crate::is;

const SHAPES: &str = "trait shape{area}; class square{side:int}; class rect{w:int h:int}; area(s:square) := s.side*s.side; area(r:rect) := r.w*r.h; ";

#[test]
fn a_loop_over_mixed_instances_dispatches_at_run_time() {
	is!(&format!("{SHAPES}t=0; for s in [square(3), rect(2, 5)] {{ t += area(s) }}; t"), 19);
}

#[test]
fn an_element_of_a_mixed_list_dispatches_at_run_time() {
	is!(&format!("{SHAPES}xs=[square(3), rect(2, 5)]; area(xs#2)"), 10);
	is!(&format!("{SHAPES}xs=[square(3), rect(2, 5)]; area(xs#1)"), 9);
}

#[test]
fn a_variable_reassigned_a_value_of_run_time_type_dispatches_at_run_time() {
	is!(&format!("{SHAPES}xs=[square(3), rect(2, 5)]; s = square(1); s = xs#2; area(s)"), 10);
}
