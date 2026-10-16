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

// a field takes no bare word: `hex` stays the conversion beside a field named hex
#[test]
fn a_conversion_word_beside_a_field_of_that_name() {
	is!("p = {hex: 1}; hex(255) == 255 as hex", true);
	is!("f(hex) := hex + 1; f(1)", 2);
}

// a field binds its name inside its object or class only: elsewhere a bare width is the canvas's
#[test]
fn a_field_named_like_a_canvas_side_binds_only_inside_its_class() {
	is!("class R{width:int; height:int; area() := width * height}; R(2,3).area()", 6);
	is!("p = {width: 5}; width == canvas.width", true);
	is!("p = {width: 5}; p.width", 5);
}

// shadowing is per arity: the program's first(x) leaves first(xs, n) to the library
#[test]
fn a_program_word_shadows_only_its_own_arities() {
	is!("first(x) := x * 2; first([5,6,7], 2) == [5, 6]", true);
	is!("first(x) := x * 2; first(3)", 6);
}

// decision class-members-free: library and built-in words are no keywords; a class member or a variable may take one
#[test]
fn a_class_member_or_variable_may_take_a_library_word() {
	is!("class Box{width:int, height:int}; b = Box(3, 4); b.width * b.height", 12);
	is!("width = 5; width + 1", 6);
	is!("class P{count:int}; P(2).count", 2);
	is!("class B{w:int; first() := w * 2}; B(4).first()", 8);
}

// a destructured parameter binds its names inside the function: `width` there is no canvas side
#[test]
fn a_destructured_parameter_named_like_a_canvas_side() {
	is!("area({width, height}) := width * height\nm = {width: 3, height: 4}\narea(m)", 12);
}
