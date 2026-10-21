//! card map-field: a class field typed with a collection word (`map`, `dict`, `list of int`) holds that collection
use crate::is;

#[test]
fn a_map_field_holds_its_map() {
	is!("class Bindings { meanings: map }\nb = Bindings{meanings: {a: 1}}\nb.meanings[\"a\"]", 1);
	is!("class Bindings { meanings: map }\nb = Bindings{meanings: {a: 1, b: 2}}\nb.meanings is map", true);
	is!("class Bindings { meanings: dict }\nb = Bindings{meanings: {a: 1}}\nb.meanings[\"a\"]", 1);
	is!("class Scores { points: map of int }\ns = Scores{points: {a: 3}}\ns.points[\"a\"]", 3);
}
