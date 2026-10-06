use crate::is;

#[test]
fn test_global_with_type_word() {
	is!("global long k=7", 7);
	is!("global int k=7", 7);
}

#[test]
fn test_global_with_modifier_words() {
	is!("global const int k=7", 7);
	is!("global mutable int k=7", 7);
	is!("global mut int k=7", 7);
}

#[test]
fn test_global_with_typed_define() {
	is!("global int k:=7", 7);
}

#[test]
fn test_global_with_export() {
	is!("global export k=7", 7);
}

#[test]
fn test_global_without_initializer_keeps_the_type_word_out_of_the_name() {
	is!("global int k;k=3;k", 3);
}

#[test]
fn test_typed_global_without_initializer_is_zero() {
	is!("global int k", 0);
	is!("global const mut int k", 0);
}

#[test]
fn test_type_word_alone_is_still_a_name() {
	is!("global int;int", 0);
}
