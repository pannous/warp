// A text joined with a call that gives a float takes the float's text form, not the call's source text
use warp::is;

#[test]
fn a_float_call_joins_a_text_by_its_value() {
	is!("f(x) := (x+1) as float; \"a\" + f(1)", "a2");
	is!("float addi(int x,int y){x+y}; \"hello\" + addi(2,3)", "hello5");
	is!("f(x) := sqrt(x); y = \"a\" + f(4); y", "a2");
}
