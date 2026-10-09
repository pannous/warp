use crate::is;

#[test]
fn test_for_over_list_literal() {
	is!("sum=0;for i in [1 2 3] {sum+=i};sum", 6);
}

#[test]
fn test_for_over_list_variable() {
	is!("xs=[4 5 6];sum=0;for x in xs {sum+=x};sum", 15);
}

#[test]
fn test_for_over_exclusive_range() {
	is!("sum=0;for i in 1..3 {sum+=i};sum", 3);
}

#[test]
fn test_for_over_inclusive_range() {
	is!("sum=0;for i in 1...3 {sum+=i};sum", 6);
	is!("sum=0;for i in 1 to 3 {sum+=i};sum", 6);
}

#[test]
fn test_for_with_colon_body() {
	is!("sum=0;for i in 1 to 3: sum+=i;sum", 6);
}

#[test]
fn test_for_classic_three_part_header() {
	is!("sum = 0; for(i=0;i<10;i++){sum+=i};sum", 45);
}

#[test]
fn test_nested_for() {
	is!("n=0;for a in [1 2] {for b in [1 2 3] {n+=1}};n", 6);
}

#[test] // samples/snake.warp: a list-valued statement in a loop body was read as a number ("not an int")
fn test_loop_body_with_a_list_statement() {
	is!("x=[1]; for t in 1 to 2 { x }; 0", 0);
	is!("def f() { return [1] }; for t in [1,2] { f() }; 0", 0);
	is!("def f() { return [1] }; t=0; while t < 2 { f(); t++ }; t", 2);
}

/// card todo-app robustness: the iterable of a loop may be a phrase, `for todo in todos sorted by priority {…}`
#[test]
fn a_loop_walks_a_filter_or_a_sorted_list() {
	is!("xs = [3, 1, 2]; s = 0; for x in xs where it > 1 { s += x }; s", 5);
	is!("xs = [{p: 3}, {p: 1}]; s = \"\"; for x in xs sorted by p { s += x.p }; s", "13");
	is!("xs = [3, 1, 2]; s = 0; for x in xs where it > 1 and it < 3 { s += x }; s", 2);
}
