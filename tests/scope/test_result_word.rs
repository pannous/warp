// `result` is the value of the statement before it (wiki/result.md, evaluation.md): an expression, an assignment's
// value, the branch an `if` took; a program defining `result` itself keeps its own
use crate::is;

#[test]
fn result_is_the_value_of_the_statement_before() {
	is!("3+4; result * 2", 14);
	is!("x = 5; result + 1", 6);
	is!("if 1>0 : \"hi\" else : \"ho\"\nresult", "hi");
	is!("f := it * 3; f 4; result", 12);
}

#[test]
fn a_program_may_name_its_own_result() {
	is!("result = 9; 3; result", 9);
}

// card result-keyword: a print gives ø, so `result` after it is the value it printed (wiki/result.md)
#[test]
fn result_after_a_print_is_the_printed_value() {
	is!("print 7; result + 1", 8);
	is!("if 1>0 : print \"hi\" else : print \"ho\"\nresult", "hi");
	is!("if 1<0 : print \"hi\" else : print \"ho\"\nresult", "ho");
	is!("if 1>0: { x = 2; print x * 3 } else: 0\nresult", 6);
}
