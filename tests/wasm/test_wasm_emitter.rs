use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_wasm_roundtrip_via_is() {
	// Parser treats html{test=1} as Key("html", {test=1}); every `=` in a tag is a value entry (user, decision
	// tag-assign), so it is html{test:1}
	is!("html{test=1}", eval("html{test:1}"));
}
