// `(def square (typed x int) (mul it it))` (card wisp-defn): the last part of a def is its body, the parameter list
// before it the body's metadata `(params …)`; it took the parameter list as the body and dropped `(mul it it)`
use warp::{parse_wisp, Node, Op};

#[test]
fn a_wisp_def_tells_the_parameters_from_the_body() {
	let Node::Key(name, Op::Define, body) = parse_wisp("(def square (typed x int) (mul it it))") else { panic!("expected a def") };
	assert_eq!(name.serialize(), "square");
	assert_eq!(body.drop_meta().serialize(), "mul(it it)");
	let Node::Meta { data, .. } = body.as_ref() else { panic!("expected the parameters as metadata: {body:?}") };
	assert!(data.serialize().contains("params"), "{}", data.serialize());
	assert_eq!(parse_wisp("(def square (mul it it))").serialize(), "square:=mul(it it)");
}
