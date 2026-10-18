//! card block-function-argument (decision serve-plain Q1): a `{…}` block passed to a parameter typed function is a
//! lambda of it, of no parameters, or of `it` when the block mentions it; it can be kept and called later, as a route's
//! answer is (a sibling of data-parameter, which takes the block as its tree)
use crate::is;

#[test]
fn a_block_to_a_function_parameter_is_called() {
	is!("run(f: function) := f()\nrun { 1 + 2 }", 3);
	is!("run(f: function) := f()\nrun({ 1 + 2 })", 3);
	is!("twice(f: function) := f(f(1))\ntwice { it * 10 }", 100);
}

#[test]
fn a_block_names_what_is_around_it() {
	is!("run(f: function) := f()\nn = 4\nrun { n + 1 }", 5);
}

#[test]
fn a_kept_block_is_called_later() {
	is!(
		"answers = []\nhandle(path: text, answer: function) := { global answers; answers.add({path: path, answer: answer}) }\nhandle \"/a\" { it + 1 }\nhandle \"/b\" { it * 3 }\nfor a in answers { if a.path == \"/b\" { return a.answer(5) } }\n0",
		15
	);
}

#[test]
fn a_lambda_or_function_still_passes_as_is() {
	is!("run(f: function) := f(3)\nrun(x => x * 2)", 6);
	is!("triple(x) := x * 3\nrun(f: function) := f(3)\nrun(&triple)", 9);
}
