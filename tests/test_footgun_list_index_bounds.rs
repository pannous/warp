use warp::wasm_emitter::eval;
use warp::Node;

fn assert_index_out_of_range(code: &str) {
	match eval(code) {
		Node::Error(message) => assert!(format!("{:?}", message).contains("index out of range"), "{code}: {message:?}"),
		other => panic!("{code} should be an error value, got {other:?}"),
	}
}

#[test]
fn test_list_index_past_end_is_an_error() {
	assert_index_out_of_range("x=[1 2 3]; x[3]");
}

#[test]
fn test_selector_zero_is_an_error() {
	assert_index_out_of_range("x=[1 2 3]; x#0");
}

#[test]
fn test_negative_bracket_index_is_an_error() {
	assert_index_out_of_range("x=[1 2 3]; x[-1]");
}
