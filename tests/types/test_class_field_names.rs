//! Field names that are also words of the language or of the program (card classes-field): an operator word before a
//! key colon names the key (`{from:… to:…}`), and `l.start` reads the field even beside a function `start`
use crate::is;

#[test]
fn operator_words_name_fields() {
	is!("class Point{x:int y:int}; class Line{from:Point to:Point}; l = Line(Point(1,2), Point(3,4)); l.from.x + l.to.y", 5);
	is!("x = {from:1 to:2}; x.to", 2);
	is!("m = {is:1 or:2 and:3}; m.is + m.or + m.and", 6);
}

#[test]
fn field_beside_function_of_its_name() {
	is!("class Line{start:int end:int}; start(l) := l.start * 10; start(Line(3,4))", 30);
	is!("class Line{start:int end:int}; start(l:Line) := l.start * 10; x = Line(3,4); start(x) + x.start", 33);
}
