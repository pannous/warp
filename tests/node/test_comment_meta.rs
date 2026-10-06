//! Card g-1tHQ (playground: `Error("no field meta@https")`): comments attach to the next element as metadata
//! (samples/comments.wasp, wiki/comments.md, wiki/meta.md): `x.meta` is the map of x's meta information, its
//! comment as `comment`; `x.@comment` reads the comment like any meta key. Only under `use comments` (P114)
use crate::is;

const GREETING: &str = "use comments\n// Comments attach to the next element as metadata\n// This documents the greeting variable\ngreeting: \"Hello\"\n";

#[test]
fn the_comment_of_a_binding_is_its_meta() {
	is!(&format!("{GREETING}greeting.meta.comment"), "Comments attach to the next element as metadata\nThis documents the greeting variable");
	is!(&format!("{GREETING}greeting.@comment"), "Comments attach to the next element as metadata\nThis documents the greeting variable");
	is!("use comments\n/* the answer */\nx = 42\nx.@comment", "the answer");
	is!(&format!("{GREETING}greeting"), "Hello");
}

#[test]
fn a_field_named_meta_stays_the_field() {
	is!("use comments\n// a point\np = {meta: 7}\np.meta", 7);
}

/// Firefox and Safari write a stack frame as `name@url`: the missing field is `meta`, not `meta@https`
#[test]
fn a_missing_field_is_named_from_any_browsers_trace() {
	let named = |trace: &str| warp::wasm_emitter::trap_error(trace, "unreachable".to_string()).serialize();
	assert!(named("no_field_meta@https://warp.pannous.com/worker.js:1:2\nmain@https://warp.pannous.com/").contains("no field meta\""), "{}", named("no_field_meta@https://x"));
	assert!(named("    at no_field_@source (wasm://wasm/1234:1:2)").contains("no field @source"));
	assert!(named("no_field_@source@https://warp.pannous.com/worker.js:1:2").contains("no field @source\""));
}

/// P114 (user): comments become meta information only when activated; without `use comments` a read names the pragma
#[test]
fn comments_are_meta_information_only_under_use_comments() {
	crate::common::fails_with("// doc\ngreeting: \"Hello\"\ngreeting.meta", "under `use comments`");
	crate::common::fails_with("// doc\nx = 1\nx.@comment", "under `use comments`");
	is!("// doc\nx = 1\nx + 1", 2);
	is!("use comments\nuse strict\n// doc\nx = 1\nx.@comment", "doc");
}
