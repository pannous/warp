// Sum types (card sum-types, samples/types.warp): `type Option[T] = Some(T) | None` declares a class per variant with a
// payload, each extending the sum type; a variant without payload is its name (None stays ø); match takes them apart
use crate::is;

#[test]
fn variant_with_payload_constructs_and_matches() {
	is!("type Option[T] = Some(T) | None; o = Some(3); match o { Some(v) => v + 1; None => 0 }", 4);
	is!("type Option[T] = Some(T) | None; o = None; match o { Some(v) => v + 1; None => 0 }", 0);
}

#[test]
fn variants_are_the_sum_type() {
	is!("type Result[T, E] = Ok(T) | Err(E); r = Err(\"boom\"); r is Result", true);
	is!("type Result[T, E] = Ok(T) | Err(E); r = Ok(2); match r { Ok(v) => v * 10; Err(e) => -1 }", 20);
}

#[test]
fn variants_with_several_or_named_fields() {
	is!("type Color: red | green | blue | rgb(int, int, int); c = rgb(1, 2, 3); match c { red => 0; rgb(r, g, b) => r + g + b }", 6);
	is!("type Color: red | green | blue | rgb(int, int, int); c = green; match c { red => 1; green => 2; rgb(r, g, b) => 3 }", 2);
	is!("type Shape = Circle(radius: int) | Square(side: int); s = Square(4); s.side * s.side", 16);
}

#[test]
fn variants_on_their_own_lines() {
	is!("type WebData\n  = NoRequest\n  | Loading\n  | Success(int)\nd = Success(7)\nmatch d { Loading => 0; Success(n) => n }", 7);
}

#[test]
fn generic_type_parameters_in_square_brackets() {
	is!("type Box[T]: { item: T }; b = Box(\"xy\"); b.item", "xy");
	is!("type Predicate[T] = T -> bool; 3", 3);
}
