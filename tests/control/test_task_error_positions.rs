//! card runtime-errors: the runtime error of a task names the line and column of its statement, not the `await`
use crate::common::fails_with;

#[test]
fn a_failing_task_names_its_statement() {
	fails_with("f(i) := [1,2,3]#i\njob = go f(5)\nawait job", "task f: index out of range: 5 not in 1…3 at 1:9");
	fails_with(
		"f(i) := {\n  y = 1\n  [1,2,3]#i\n}\njob = go f(5)\nawait job",
		"task f: index out of range: 5 not in 1…3 at 3:3",
	);
}
