// `global x` in a function is the same in a program that also makes closures (card global-function): the lifted
// lambdas join the program's statements instead of wrapping the program as one statement
use crate::is;

#[test]
fn global_in_a_program_with_a_lambda() {
	is!("x = 7\nys = [(value) => { 0 }]\ndef bump() { global x; x = 5 }\nbump()\nx", 5);
	is!("x = cell_new(1)\nys = [(value) => { 0 }]\ndef bump() { global x; cell_set(x, 5) }\nbump()\ncell_get(x)", 5);
}
