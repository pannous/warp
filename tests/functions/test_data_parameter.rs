//! decisions.md data-parameter: a parameter typed `data` takes its argument as an unevaluated tree, in a call and as a
//! tag's block, so a library takes a DSL block (`style{ ul > li { color: red } }`)
use crate::is;

#[test]
fn a_data_argument_is_its_code() {
	is!("shape(code: data) := code.op\nshape(a + b)", "+");
	is!("written(code: data) := str(code)\nwritten(ul > li)", "ul>li");
}

#[test]
fn a_tag_block_is_the_data_argument() {
	is!("written(sheet: data) := str(sheet)\nwritten{ ul > li }", "{ul>li}");
}

#[test]
fn other_parameters_take_values() {
	is!("labelled(n: int, code: data) := str(n) + str(code)\nlabelled(1 + 1, x + 1)", "2x+1");
}
