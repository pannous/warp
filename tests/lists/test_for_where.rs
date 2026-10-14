// card for-where: `for x in xs where x > 1 { … }` walks the elements the condition keeps, the loop variable or `it`
// naming each (found editing samples/orm.warp: it was "undefined variable: where")
use crate::is;

#[test]
fn a_loop_over_the_elements_a_condition_keeps() {
	is!("xs = [1, 2, 3]; total = 0; for x in xs where x > 1 { total += x }; total", 5);
	is!("xs = [1, 2, 3]; total = 0; for x in xs where it > 2 { total += x }; total", 3);
	is!("class P{n: text; a: int}; ps: [P] = [P(\"a\", 1), P(\"b\", 5)]; names = []; for p in ps where a > 2 { names.add(p.n) }; names", warp::texts(vec!["b"]));
}
