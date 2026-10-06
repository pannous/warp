// footgun-pi: `pi = 4; pi` gave 3.14159…: the parser read pi as π and the assignment was dropped. P130 (user: "loud
// error, if it was declared constant before which it should be"): a named number is a declared constant, assigning it
// is a compile error; a field named pi is the type's own field
use crate::is;
use crate::common::fails_with;

const REFUSAL: &str = "is a constant";

#[test]
fn an_assigned_constant_word_is_an_error() {
	fails_with("pi = 4; pi", "pi is a constant");
	fails_with("pi := 4", REFUSAL);
	fails_with("tau = 1; tau + 1", "tau is a constant");
}

#[test]
fn a_constant_word_still_reads_and_names_fields() {
	is!("class circle{pi = 3}; c = circle{}; c.pi", 3);
	is!("x = pi; x > 3.14 and x < 3.15", true);
	is!("pi == 3", false);
}

// P130 assumption: a field named pi is the class's own field, declared typed or with a value, and its methods read it
#[test]
fn a_field_named_like_a_constant_is_read_by_the_methods() {
	is!("class C{pi:int}; c = C(4); c.pi", 4);
	is!("class C{r:int; pi = 3; f() := pi * r}; c = C(2); c.f()", 6);
	is!("class C{r:int; f() := r > 0 ? pi : 0}; C(1).f() > 3", true);
	is!("class C{pi:int}; x = pi; x > 3", true);
}
