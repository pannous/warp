// A definition's body as written, whatever the parser stops at (cards method-suffix, sum-function, def-end)
use crate::is;

/// card method-suffix: `fn f() := 3 squared` in a class is the method, its suffix word the body's
#[test]
fn a_method_body_keeps_its_suffix_word() {
	is!("class C{ n: int; fn f() := 3 squared }; C(2).f()", 9);
	is!("class C{ n: int; f() := n squared }; C(4).f()", 16);
}

/// card sum-function: an assigned or defined value reads `sum of xs` as alone
#[test]
fn of_phrase_as_a_value() {
	is!("f() := sum of [1, 2]; f()", 3);
	is!("x = sum of [1, 2]; x", 3);
	is!("f() := first of [5, 2]; f()", 5);
}

/// card def-end: a top-level Ruby def with its lines indented by tabs, as by spaces
#[test]
fn ruby_def_indented_by_tabs() {
	is!("def f(x)\n\tx * 2\nend\nf(3)", 6);
	is!("def f(x)\n\ty = x * 2\n\ty + 1\nend\nf(3)", 7);
	is!("class Point\n\tdef initialize(x)\n\t\t@x = x\n\tend\n\tdef double\n\t\t@x * 2\n\tend\nend\nPoint.new(4).double", 8);
}
