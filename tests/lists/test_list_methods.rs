//! card list-methods: the words of a standard module named after a type (lib/list.warp, text.warp, map.warp) are
//! that type's methods: `xs.unique()` works without `use list`, `unique(xs)` and the qualified `list.zip(a, b)` still
//! need it
use crate::is;
use crate::common::fails_with;

#[test]
fn a_list_module_word_is_a_method_without_use() {
	is!("xs=[1 2 2]; str(xs.unique())", "[1 2]");
	is!("xs=[1 2 3]; str(xs.take(2))", "[1 2]");
	is!("xs=[1 2 3]; xs.product()", 6);
	is!("use list; xs=[1 2 2 3]; unique(xs) == xs.unique()", true);
	is!("unique(xs) := 42; xs=[1 1]; xs.unique()", 42);
	fails_with("xs=[1 2 2]; unique(xs)", "use list");
}

#[test]
fn text_and_map_module_words_are_methods_too() {
	is!("\"a b\".words().count()", 2);
	is!("m={a:1}; m.merge({b:2}).b", 2);
}
