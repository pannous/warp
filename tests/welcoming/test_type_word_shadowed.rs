//! Card local-named-like-2: a variable named like a type word (`key = 5`) still makes `n is key` compare with its
//! value, but never silently: a warning names the variable and the explicit forms `n == key` and `n is a key`. With
//! an article the type word is always the type
use crate::common::warns_with;
use crate::is;

#[test]
fn a_type_test_of_a_shadowed_type_word_warns() {
	let key_node = "Key(1, \":\", 2)";
	warns_with(&format!("f(n: any) := {{ key = 5; type(n) is key }}; f({key_node})"), false, "`key` is a variable");
	warns_with("int = 5; 5 is int", true, "`int` is a variable");
}

#[test]
fn an_article_keeps_the_type_word_a_type() {
	is!("f(n: any) := { key = 5; n is a key }; f(Key(1, \":\", 2))", true);
	is!("int = 5; 3 is an int", true);
}
