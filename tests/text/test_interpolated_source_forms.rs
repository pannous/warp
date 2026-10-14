//! Card interpolation-source: the holes of `"…\(x)…"` are code to every source pass, so a hole may hold any form a
//! statement may: reflection words, dir(), class instances.

use crate::is;

#[test]
fn reflection_words_in_holes() {
	is!("area(w: int, h: int) := w * h\n\"\\(area.params)\"", "[\"w\" \"h\"]");
	is!("area(w: int, h: int) := w * h\n\"sig \\(area.signature)\"", "sig (w:int, h:int) -> int");
	is!("class Point { x: int; y: int }\np = Point(3, 4)\n\"\\(dir(p)) \\(p.fields)\"", "[\"x\" \"y\"] [\"x\" \"y\"]");
}
