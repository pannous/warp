//! wiki/type.md (card namespace-decl): C# 10's file-scoped declarations. `class Foo;` takes in the definitions after it,
//! up to the first other statement; `namespace test;` names the file's words, so `test.bar()` is `bar()`
use crate::is;

#[test]
fn a_file_class_takes_in_the_definitions_after_it() {
	is!("class Foo; int bar(){ 42 }; Foo().bar()", 42);
	is!("class Foo; int bar(){ 42 }; int boom(){ -1 }; Foo().boom()", -1);
	is!("class Foo; x: int; double() := x * 2; Foo(4).double()", 8);
}

#[test]
fn a_namespace_qualifies_the_words_of_the_file() {
	is!("namespace test; int bar(){42}; test.bar()", 42);
	is!("namespace test; int bar(){42}; bar()", 42);
	is!("namespace test; class Foo; int bar (){ 42 }; int boom (){ -1 }; test.Foo().boom()", -1);
}

#[test]
fn a_braced_empty_class_takes_nothing_in() {
	is!("class Foo {}; int bar(){42}; bar()", 42);
}

#[test]
fn a_class_body_of_one_typed_method() {
	is!("class Foo { int bar(){ 42 } }; Foo().bar()", 42);
}
