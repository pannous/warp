// card type-value: type(x) of a value held as a Node (an optional, an `any` parameter, an item of a mixed list) is the
// value's type read at run time, not "empty"
use crate::is;

#[test]
fn the_type_of_a_node_held_value_is_read_at_run_time() {
	is!("x:int?=3; type(x)", "int");
	is!("f(x:any) := type(x); f(3)", "int");
	is!("f(x:any) := type(x); f(\"ab\")", "text");
	is!("f(x:any) := type(x); f(2.5)", "float"); // as type(2.5): decimals are floats (decision exact-default)
	is!("f(x:any) := type(x); f(true)", "bool");
	is!("xs = [1, \"ab\"]; type(xs#2)", "text");
	is!("x: int|text = 3; type(x)", "int");
}
