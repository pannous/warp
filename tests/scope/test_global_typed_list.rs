// `global xs` in a function of a typed main-level list (`xs: [int]`, `stored users: [User]`): the function reads and
// changes the list itself, as it does an untyped one (found writing samples/server.warp)
use crate::is;

#[test]
fn a_function_changes_a_typed_global_list() {
	is!("xs: [int] = [1]; def f() { global xs; xs += [2]; count(xs) }; f()", 2);
	is!("xs: [int] = [1]; def f() { global xs; xs.add(2); count(xs) }; f()", 2);
}

#[test]
fn a_function_reads_a_typed_global_list_of_instances() {
	is!("class U{n:text}; xs: [U] = [U(\"a\")]; def f() { global xs; count(xs) }; f()", 1);
	is!("class U{n:text}; xs: [U] = [U(\"a\")]; def f() { global xs; xs.add(U(\"b\")); xs#2.n }; f()", "b");
}
