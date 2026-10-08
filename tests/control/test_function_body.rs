// Card g_X_3s: `f.body` is the function's body as written, as data (like `data x+1`): not run, readable and printable
use warp::wasm_emitter::eval;

fn body_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_functions_body_is_its_code_as_data() {
	assert_eq!(body_of("f(x) := x+1\nf.body"), "x+1");
	assert_eq!(body_of("def g(x){x*2}\ng.body"), body_of("data {x*2}"));
	assert_eq!(body_of("square := it*it\nsquare.body"), "it*it");
}

#[test]
fn a_field_named_body_wins() {
	assert_eq!(body_of("page = {body: 3}\npage.body"), "3");
}
