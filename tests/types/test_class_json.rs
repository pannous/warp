//! Class instances and json (notes/classes.md): to_json gives the fields as an object, from_json and `as` build an
//! instance from one
use crate::is;

const POINT: &str = "use json; class Point{x:int y:int}; ";

#[test]
fn to_json_of_an_instance_is_its_fields() {
	is!(&format!("{POINT}to_json(Point(1, 2))"), "{\"x\":1,\"y\":2}");
	is!(&format!("{POINT}to_json([Point(1, 2), Point(3, 4)])"), "[{\"x\":1,\"y\":2},{\"x\":3,\"y\":4}]");
	is!(&format!("{POINT}class Line{{a:Point b:Point}}; to_json(Line(Point(1, 2), Point(3, 4)))"), "{\"a\":{\"x\":1,\"y\":2},\"b\":{\"x\":3,\"y\":4}}");
}

#[test]
fn an_instance_from_json() {
	is!(&format!("{POINT}p = parse_json(\"{{\\\"x\\\":1,\\\"y\\\":2}}\") as Point; p.x * 10 + p.y"), 12);
	is!(&format!("{POINT}p = Point.from_json(\"{{\\\"x\\\":3,\\\"y\\\":4}}\"); p.y"), 4);
	is!("class Point{x:int y:int}; p = {x:1 y:2} as Point; p.y", 2);
	is!(&format!("{POINT}class Line{{a:Point b:Point}}; l = Line.from_json(\"{{\\\"a\\\":{{\\\"x\\\":1,\\\"y\\\":2}},\\\"b\\\":{{\\\"x\\\":3,\\\"y\\\":4}}}}\"); l.b.x"), 3);
}

#[test]
fn a_roundtrip_through_json() {
	is!(&format!("{POINT}p = Point.from_json(to_json(Point(5, 6))); p.x + p.y"), 11);
}

#[test]
fn other_languages_serializers_are_to_json() {
	is!(&format!("{POINT}JSON.stringify(Point(1, 2))"), "{\"x\":1,\"y\":2}");
	is!("class Point{x:int y:int}; d = dataclasses.asdict(Point(1, 2)); d.y", 2);
	is!(&format!("{POINT}Json.encodeToString(Point(1, 2))"), "{\"x\":1,\"y\":2}");
	is!("use json; @Serializable data class Point(val x: Int, val y: Int)\nto_json(Point(1, 2))", "{\"x\":1,\"y\":2}");
}

#[test]
fn lists_of_instances_from_json() {
	is!(&format!("{POINT}ps = parse_json(\"[{{\\\"x\\\":1,\\\"y\\\":2}},{{\\\"x\\\":3,\\\"y\\\":4}}]\") as [Point]; ps#2.x"), 3);
	is!(&format!("{POINT}class Poly{{points:[Point]}}; p = Poly.from_json(to_json(Poly([Point(1, 2), Point(5, 6)]))); p.points#2.y"), 6);
}
