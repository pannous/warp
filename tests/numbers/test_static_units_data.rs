//! Static units, stage 6: quantities in object fields, in `any` and annotated variables, serialized, and in lists grown at
//! run time. The signatures stay static; a final object names the units of its fields in `wasp.units`
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_object_fields_hold_quantities() {
	assert_eq!(shown("p = {dist: 0 m}; for i in 1..3 { p.dist += 250 m }; p.dist"), "500m");
	assert_eq!(shown("p = {dist: 0 km, name: \"run\"}; for i in 1..3 { p.dist += 250 m }; p"), "{dist:500m name:\"run\"}");
	assert_eq!(shown("p = {dist: 1 km}; for i in 1..2 { p.dist = p.dist * 2 }; p.dist / 4 h"), "0.5km/h");
	fails_with("p = {dist: 1 m}; for i in 1..2 { p.dist = 2 s }; p", "DimensionError");
	// `dist: 5 m` is data, no block to run later
	assert_eq!(shown("p = {dist: 5 m, t: 2 s}; p"), "{dist:5m t:2s}");
	assert_eq!(shown("p = {dist: 5 m}; p.dist + 1 km"), "1005m");
}

#[test]
fn test_annotated_variables_keep_and_check_units() {
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; x:any = d; x"), "4m");
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 500 m }; x:km = d; x"), "1000m");
	fails_with("d = 0 m; for i in 1..3 { d += 2 m }; x:s = d; x", "DimensionError");
}

#[test]
fn test_a_serialized_quantity_names_its_unit() {
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; d.serialize()"), "\"4m\"");
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; serialize(d)"), "\"4m\"");
}

#[test]
fn test_lists_grow_with_quantities_of_their_unit() {
	assert_eq!(shown("xs = [1 m]; xs.add(2 m); sum(xs)"), "3m");
	fails_with("xs = [1 m]; xs.add(2 s); sum(xs)", "DimensionError");
}

#[test]
fn test_a_whole_object_shows_its_units() {
	// stage 7: the quantity fields come first in the text, the plain ones follow
	assert_eq!(shown("p = {name: \"a b\", dist: 2 m}; for i in 1..2 { p.dist += 1 m }; \"${p}\""), "\"{dist:3m name:\"a b\"}\"");
	assert_eq!(shown("p = {dist: 0 km, t: 1 s}; for i in 1..3 { p.dist += 250 m }; text_form(p)"), "\"{dist:500m t:1s}\"");
	#[cfg(feature = "native")]
	assert!(crate::common::printed("p = {dist: 0 km, name: \"run\"}; for i in 1..3 { p.dist += 250 m }; print p").starts_with("{dist:500m name:\"run\"}\n"));
}

#[test]
fn test_aliases_keep_the_units() {
	assert_eq!(shown("p = {dist: 1 km, n: 3}; q = p; q.dist += 500 m; q.dist"), "1500m");
	assert_eq!(shown("p = {dist: 1 km}; q = p; q"), "dist:1km");
	assert_eq!(shown("xs = [1 m]; ys = xs; ys.add(2 m); sum(ys)"), "3m");
	fails_with("p = {dist: 1 km}; q = p; q.dist = 2 s; q", "DimensionError");
}
