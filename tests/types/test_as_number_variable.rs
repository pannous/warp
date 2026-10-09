//! card number-variable: `"4" as number` held in a variable, or of a text known only at run time, is the number,
//! as the literal cast is: an Int when whole, else a Float
use crate::is;

#[test]
fn a_variable_holds_the_cast_number() {
	is!("class V{x:int}\nn = \"4\" as number\nV(n).x", 4);
	is!("class V{x:int}\nV(\"4\" as number).x", 4);
	is!("n = \"4\" as number\nn + 1", 5);
	is!("n = \"4.5\" as number; n * 2", 9);
	is!("class V{x:float}\nn = \"4.5\" as number\nV(n).x", 4.5);
}

#[test]
fn a_text_known_at_run_time_casts_to_its_number() {
	is!("class V{x:int}\nf(t:text) := V(t as number).x\nf(\"4\")", 4);
	is!("t = \"12\"; (t as number) + 1", 13);
	is!("t = \"4.5\"; t as number", 4.5);
	is!("f(t:text) := t as number; f(\"4.5\") * 2", 9);
}
