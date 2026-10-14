// Enums and sealed classes with matching on them, as other languages write them (notes/classes.md "Enums"): a warp
// enum is the object of its cases numbered from 0 (declarations::enum_object), matched with `switch`
use crate::is;

#[test]
fn a_swift_enum_matched_with_switch_case() {
	let direction = "enum Direction { case north, south }\nlet d = Direction.south\n";
	is!(&format!("{direction}switch d {{ case .north: 1; case .south: 2 }}"), 2);
	is!(&format!("{direction}switch d {{ case Direction.north: 1; default: 3 }}"), 3);
}

#[test]
fn a_kotlin_enum_class_matched_with_when() {
	let color = "enum class Color { RED, GREEN }\nval c = Color.GREEN\n";
	is!(&format!("{color}when (c) {{ Color.RED -> 1; Color.GREEN -> 2 }}"), 2);
	is!(&format!("{color}when (c) {{ Color.RED -> 1; else -> 3 }}"), 3);
	is!("val n = 5\nwhen (n) { 1, 2 -> 10; 5 -> 50; else -> 0 }", 50);
}

#[test]
fn a_kotlin_sealed_class_matched_by_type() {
	let shapes = "sealed class Shape\nclass Circle(val r: Int) : Shape()\nclass Square(val a: Int) : Shape()\n";
	is!(&format!("{shapes}fun area(s: Shape): Int = when (s) {{ is Circle -> 3 * s.r * s.r; is Square -> s.a * s.a }}\narea(Square(2))"), 4);
	is!(&format!("{shapes}fun area(s: Shape): Int = when (s) {{ is Circle -> 3 * s.r * s.r; is Square -> s.a * s.a }}\narea(Circle(1))"), 3);
}

#[test]
fn a_rust_enum_matched_by_path() {
	is!("enum Color { Red, Green }\nlet c = Color::Green;\nmatch c { Color::Red => 1, Color::Green => 2 }", 2);
}
