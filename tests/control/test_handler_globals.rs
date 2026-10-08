// Cards handler-global and handler-annotated (found by the type model, warp-16): a handler writing a main-level
// variable, plain or annotated, writes the program's variable; an emit that may find no handler is ø only when it
// actually does
use crate::common::fails_with;
use crate::is;

#[test]
fn a_handler_assigns_an_emitted_value_to_a_global() {
	is!("y=0; on ask { 1 } in { on ask { y = emit ask; y + 10 } in { emit ask } }", 11);
	is!("y=0; f() := { global y; y = (if 1==1 then 5 else ø); y + 10 }; f()", 15);
}

#[test]
fn an_int_given_empty_fails_when_it_happens() {
	fails_with("y=0; f() := { global y; y = (if 1==2 then 5 else ø); y + 10 }; f()", "not a number");
}

#[test]
fn a_handler_writes_an_annotated_global() {
	is!("level: int = 0; on alarm {level=7}; emit alarm; level", 7);
	is!("level: int = 0; on alarm {level += 7}; emit alarm; emit alarm; level", 14);
}
