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

/// decisions.md data-names: a name the program binds is its value, spliced in at the call site; an unbound word stays
/// a word; a quoted text is the escape; an operator whose operands are all values folds
#[test]
fn bound_names_in_data_are_their_values() {
	is!("theme = 'blue'\nwritten(code: data) := str(code)\nwritten(color: theme)", "{color:\"blue\"}");
	is!("theme = 'blue'\nwritten(code: data) := str(code)\nwritten{ li{ color: theme } }", "{li:{color:\"blue\"}}");
	is!("blue = 1\nwritten(code: data) := str(code)\nwritten(color: \"blue\")", "{color:\"blue\"}");
	is!("n = 2\nwritten(code: data) := str(code)\nwritten(\"count \" + n)", "count 2");
	is!("written(code: data) := str(code)\nshown(x) := written(color: x)\nshown('blue')", "{color:\"blue\"}");
}

// a name assigned inside a data argument is not bound by the program: it stays as written
#[test]
fn names_assigned_in_data_stay_written() {
	is!("written(code: data) := str(code)\nwritten{ x = 1 }", "{x=1}");
}
