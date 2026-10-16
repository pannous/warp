// Properties are defined in warp (decision property-accessors, card graphics-properties): a getter `o.p := …` and
// its setter, the one-parameter overload `o.p(v) := …` or a getter-shaped body reading `it`; lib/draw.warp and
// lib/prelude.warp define canvas, window and display this way, a program its own (tests/programs/test_window_size.rs
// covers the standard ones)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_program_defines_a_property_with_a_setter() {
	is!("box.size := 4\nbox.size(n) := print \"set ${n}\"\nbox.size = 7\nbox.size", 4);
	is!("global kept = 1\nbox.size := kept\nbox.size := kept = it\nbox.size = 7\nbox.size += 1\nbox.size", 8);
	is!("global kept = 1\nbox.size := kept\nbox.size(n) := kept = n\nbox.size(5)\nbox.size", 5);
}

#[test]
fn a_property_without_a_setter_is_read_only() {
	let refused = format!("{:?}", eval("// the box is sealed\nbox.size := 4\nbox.size = 7"));
	assert!(refused.contains("box.size is read-only") && refused.contains("the box is sealed"), "{refused}");
}

#[test]
fn the_setter_is_called_like_a_method() {
	is!("canvas.width(12)\ncanvas.width", 12);
	is!("display.dpi(200 dpi)\n(1 inch * display.dpi) == 200px", true);
}

#[test]
fn a_property_of_a_value_or_an_instance_setter_is_refused_loudly() {
	let of_value = format!("{:?}", eval("p = {x: 1}\np.x := 5\np.x"));
	assert!(of_value.contains("p is a value") && of_value.contains("write p.x = …"), "{of_value}");
	let of_class = format!("{:?}", eval("class Point{x;y}\nPoint.label(v) := print v\nPoint(1,2).x"));
	assert!(of_class.contains("defines no setter of Point's instances"), "{of_class}");
}

// card point-sum: a property of a class's instances, `it` the instance
#[test]
fn a_class_defines_a_property_of_its_instances() {
	is!("class Point{x;y}\nPoint.sum := it.x + it.y\nPoint(1,2).sum", 3);
	is!("class Point{x;y}\nPoint.sum := it.x + it.y\np = Point(3,4)\np.sum + sum([1, 2])", 10);
}
