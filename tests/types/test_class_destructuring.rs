//! Destructuring instances (notes/classes.md): `{x, y} = p` by name, `(x, y) = p` in field order, match arms
//! `Point{x, y} => …` and `Point(x, y) => …`
use crate::is;

const POINT: &str = "class Point{x:int y:int}; p = Point(1, 2); ";

#[test]
fn braces_take_fields_by_name() {
	is!(&format!("{POINT}{{x, y}} = p; x * 10 + y"), 12);
	is!(&format!("{POINT}{{y}} = p; y"), 2);
	is!(&format!("{POINT}{{y: b, x: a}} = p; a * 10 + b"), 12);
	is!("m = {x:1 y:2}; {x, y} = m; x + y", 3);
}

#[test]
fn parentheses_take_fields_in_order() {
	is!(&format!("{POINT}(a, b) = p; a * 10 + b"), 12);
	is!(&format!("{POINT}a, b = p; a * 10 + b"), 12);
	is!("class Point{x:int y:int}; (a, b) = Point(3, 4); a + b", 7);
}

#[test]
fn match_arms_take_an_instance_apart() {
	let shapes = "class Circle{r:int}; class Rect{w:int h:int}; ";
	is!(&format!("{shapes}area(s) := match s {{ Circle{{r}} => 3 * r * r; Rect{{w, h}} => w * h }}; area(Rect(2, 3)) + area(Circle(1))"), 9);
	is!(&format!("{shapes}area(s) := match s {{ Circle(r) => 3 * r * r; Rect(w, h) => w * h }}; area(Rect(2, 5))"), 10);
}

#[test]
fn nested_patterns_and_constants() {
	let classes = "class Point{x:int y:int}; class Line{start:Point end:Point}; ";
	let kind = "kind(p) := match p { Point{x: 0, y} => y; Point(x, 0) => x * 10; Point{x, y} => x + y + 100 }; ";
	is!(&format!("{classes}{kind}kind(Point(0, 7))"), 7);
	is!(&format!("{classes}{kind}kind(Point(4, 0))"), 40);
	is!(&format!("{classes}{kind}kind(Point(1, 2))"), 103);
	let start = "first_y(l) := match l { Line{start: Point{x: 0, y}, end} => y + end.x; Line(Point(a, _), _) => a }; ";
	is!(&format!("{classes}{start}first_y(Line(Point(0, 2), Point(5, 6)))"), 7);
	is!(&format!("{classes}{start}first_y(Line(Point(3, 2), Point(5, 6)))"), 3);
	is!(&format!("{classes}l = Line(Point(1, 2), Point(3, 4)); {{start: Point{{x, y}}, end}} = l; x + y + end.y"), 7);
}
