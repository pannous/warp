//! A main-level variable a function reads, declared with a type or read by a call before the definition, is read as it
//! is at the call, like an untyped one first bound after the definition (card typed-global)
use crate::is;

#[test]
fn a_typed_variable_bound_after_the_definition_is_read_at_the_call() {
	is!("def f(){ count(users) }; users: [int] = [1,2]; f()", 2);
	is!("def f(){ k + 1 }; k: int = 3; f()", 4);
}

#[test]
fn a_call_before_the_definition_reads_the_variable() {
	is!("users: [int] = [1,2]; x = f(); def f(){ count(users) }; x", 2);
	is!("users = [1,2]; x = f(); def f(){ count(users) }; x", 2);
	is!("k = 3; x = f(); def f(){ k + 1 }; x", 4);
}
