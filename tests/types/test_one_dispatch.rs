//! card one-dispatch (notes/dispatch.md "One lookup for every word"): `x.word` ≡ `x.word()`, the receiver's own
//! member before its type's library word, decided at compile time when the receiver is known, else at run time
use crate::is;

const PERSON: &str = "class P{first:int}";

#[test]
fn a_member_is_read_with_or_without_parentheses() {
	is!("p = {first: 1}; p.first", 1);
	is!("p = {first: 1}; p.first()", 1);
	is!(&format!("{PERSON}; n = P(3); n.first()"), 3);
	is!("f(m) := m.first(); f({first: 9})", 9);
}

#[test]
fn an_instance_of_unknown_class_reads_its_own_member_at_run_time() {
	is!(&format!("{PERSON}; f(p) := p.first; f(P(3))"), 3);
	is!(&format!("{PERSON}; f(p) := p.first(); f(P(3))"), 3);
	is!(&format!("{PERSON}; f(p) := p.first; f([4, 5])"), 4);
}

// decision setter-calls: a property's getter and setter by uniform calls too, `width(canvas)` and `width(canvas, 12)`
#[test]
fn a_property_is_called_in_function_form() {
	is!("canvas.width = 30; width(canvas)", 30);
	is!("width(canvas, 12); canvas.width", 12);
	is!("global kept = 1\nbox.size := kept\nbox.size(n) := kept = n\nsize(box, 5)\nsize box", 5);
}

// inside its own setter, writing the property stores the field (it recursed until the heap ran out)
#[test]
fn a_setter_writing_its_own_field_stores_it() {
	is!("class Box{w:int; set w(v) { w = v * 2 }}; b = Box(1); b.w = 5; b.w", 10);
}

// shadowing is scoped: a function binding a host word keeps it to itself, the rest of the program has the host's
#[test]
fn a_host_word_bound_inside_a_function_stays_the_hosts_outside() {
	is!("f(x) := { random = 4; random + x }; r = random; r < 1", true);
	is!("f(x) := { random = 4; random + x }; f(1)", 5);
}

#[test]
fn parse_bound_inside_a_function_stays_the_code_outside() {
	is!("f(x) := { parse = 4; parse + x }; parse(\"1 + 2\") == data (1 + 2)", true);
	is!("f(x) := { parse = 4; parse + x }; f(1)", 5);
	is!("f(eval) := eval + 1; f(1)", 2);
	is!("f(eval) := eval + 1; eval(\"2*3\")", 6);
}
