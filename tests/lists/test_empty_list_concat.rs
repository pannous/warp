// card null-typed: `[]` is ø, the empty list: concatenating it to a list needs no `if ys {…}` check
use crate::common::fails_with;
use crate::is;

#[test]
fn an_empty_list_variable_concatenates() {
	is!("xs = [1, 2]; ys = []; c = xs + ys; count(c)", 2);
	is!("xs = [1, 2]; ys = []; c = ys + xs; count(c)", 2);
	is!("xs = [1, 2]; count(xs.concat(xs))", 4);
	is!("count(concat([1], [2, 3]))", 3);
	fails_with("n = 5; m = ø; n + m", "m may be ø");
}
