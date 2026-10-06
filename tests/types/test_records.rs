//! Records of declared types as values (samples/binary_tree.wasp): optional fields, setting fields of instances, and
//! the recursive functions over them
use crate::is;
use warp::*;

const TREE: &str = "type T { value: int, left?, right? }
def ins(tree, v) { if tree == ø { return T { value: v } }; if v < tree.value { tree.left = ins(tree.left, v) } else { tree.right = ins(tree.right, v) }; return tree }
";

#[test] // `right? }` with a space, and one field per line, lost the optional mark
fn test_optional_field_marks() {
	is!("type T { value: int, right? }; t = T { value: 5 }; t.value", 5);
	is!("type T {\n value: int\n left?\n right?\n}\nT { value: 5 }.value", 5);
	is!("x = 1; x ? 2 : 3", 2);
}

#[test] // a left out optional field holds ø; setting a field of an instance keeps it an instance of its type
fn test_optional_field_is_empty_and_settable() {
	is!("type T { value: int, left? }; t = T { value: 5 }; t.left == ø", true);
	is!("type T { value: int, left? }; def f(t) { t.left = 3; return t }; x = f(T { value: 5 }); x.left", 3);
	is!(&format!("{TREE}t = ø; for v in [5, 3, 8, 1] {{ t = ins(t, v) }}; t.left.value + t.right.value + t.left.left.value"), 12);
}

#[test] // `return T { … }` in a block, and a bare `return`
fn test_returns() {
	is!("type T { value: int }; mk(v) := { return T { value: v } }; mk(5).value", 5);
	is!("def f(x) { if x == 0 { return }; return 5 }; f(1) + 2", 7);
	is!("def f(x) { if x == 0 { return }; return 5 }; f(0)", Node::Empty);
}

#[test] // recursion is assumed to return an Int first: `f(t - 1) + [t]` became an error kind, or an Int
fn test_recursive_list_function() {
	is!("def io(t) { if t == 0 { return [] }; return io(t - 1) + [t] }; count(io(3))", 3);
	is!(&format!("{TREE}def io(tree) {{ if tree == ø {{ return [] }}; return io(tree.left) + [tree.value] + io(tree.right) }}; t = ø; for v in [5, 3, 8] {{ t = ins(t, v) }}; io(t)#3"), 8);
}

#[test] // a parameter that only a recursive call passes a text is a text
fn test_parameter_passed_a_text_by_recursion() {
	is!("def p(n, prefix) { if n == 0 { return count(prefix) }; return p(n - 1, prefix + \"ab\") }; p(2, \"\")", 4);
}
