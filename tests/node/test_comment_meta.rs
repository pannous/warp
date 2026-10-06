//! Card g-1tHQ (playground: `Error("no field meta@https")`): comments attach to the next element as metadata
//! (samples/comments.wasp, wiki/comments.md, wiki/meta.md): `x.meta` is the map of x's meta information, its
//! comment as `comment`; `x.@comment` reads the comment like any meta key
use crate::is;

const GREETING: &str = "// Comments attach to the next element as metadata\n// This documents the greeting variable\ngreeting: \"Hello\"\n";

#[test]
fn the_comment_of_a_binding_is_its_meta() {
	is!(&format!("{GREETING}greeting.meta.comment"), "Comments attach to the next element as metadata\nThis documents the greeting variable");
	is!(&format!("{GREETING}greeting.@comment"), "Comments attach to the next element as metadata\nThis documents the greeting variable");
	is!("/* the answer */\nx = 42\nx.@comment", "the answer");
	is!(&format!("{GREETING}greeting"), "Hello");
}

#[test]
fn a_field_named_meta_stays_the_field() {
	is!("// a point\np = {meta: 7}\np.meta", 7);
}

/// Firefox and Safari write a stack frame as `name@url`: the missing field is `meta`, not `meta@https`
#[test]
fn a_missing_field_is_named_from_any_browsers_trace() {
	let named = |trace: &str| warp::wasm_emitter::trap_error(trace, "unreachable".to_string()).serialize();
	assert!(named("no_field_meta@https://warp.pannous.com/worker.js:1:2\nmain@https://warp.pannous.com/").contains("no field meta\""), "{}", named("no_field_meta@https://x"));
	assert!(named("    at no_field_@source (wasm://wasm/1234:1:2)").contains("no field @source"));
	assert!(named("no_field_@source@https://warp.pannous.com/worker.js:1:2").contains("no field @source\""));
}
