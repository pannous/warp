use crate::is;

// natural scripting style: range(n), [0] * n and list params of fn-defined functions fail, see probes/algo/queens/
#[test]
fn test_queens() {
	is!("samples/functions/queens.warp", 1119);
}

#[test]
fn test_queens_idiomatic() {
	is!("samples/functions/queens_idiomatic.warp", 1119);
}
