// Class declarations as other languages write them, each the same class (notes/classes.md "Ported forms")
use crate::is;

#[test]
fn a_kotlin_primary_constructor_declares_the_fields() {
	is!("class Point(val x: Int, val y: Int) { fun sum() = x + y }; Point(1, 2).sum()", 3);
	is!("data class Point(val x: Int, val y: Int)\nval p = Point(1, 2)\np.x + p.y", 3);
	is!("data class P(val x: Int); P(1) == P(1)", 1);
	is!("class Box(var w: Int = 2); Box().w", 2);
}

#[test]
fn a_javascript_class_with_a_constructor() {
	let point = "class Point {\n  constructor(x, y) { this.x = x; this.y = y }\n  sum() { return this.x + this.y }\n}\n";
	is!(&format!("{point}new Point(1, 2).sum()"), 3);
	is!(&format!("{point}p = new Point(3, 4); p.y"), 4);
}

#[test]
fn a_python_class_with_init() {
	let point = "class Point:\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\n    def length2(self):\n        return self.x * self.x + self.y * self.y\n";
	is!(&format!("{point}Point(3, 4).length2()"), 25);
	is!(&format!("{point}p = Point(3, 4)\np.x"), 3);
}

#[test]
fn a_swift_struct_with_a_mutating_method() {
	let counter = "struct Counter {\n    var count = 0\n    mutating func increment() { count += 1 }\n}\n";
	is!(&format!("{counter}var c = Counter()\nc.increment()\nc.increment()\nc.count"), 2);
}

#[test]
fn a_csharp_record_declares_its_fields() {
	is!("record Point(int X, int Y);\nvar p = new Point(1, 2);\np.X + p.Y", 3);
}

#[test]
fn a_rust_struct_takes_the_methods_of_its_impl() {
	is!("struct Point { x: i32, y: i32 }\nimpl Point { fn sum(&self) -> i32 { self.x + self.y } }\nlet p = Point { x: 1, y: 2 };\np.sum()", 3);
}

#[test]
fn a_copy_with_changed_fields() {
	is!("data class Point(val x: Int, val y: Int)\nval p = Point(1, 2)\nval q = p.copy(y = 5)\nq.y * 10 + p.y", 52);
}
