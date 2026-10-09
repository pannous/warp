use warp::warp_parser::parse;
use warp::Node::Symbol;

#[test]
fn dollar_names_keep_their_sigil() {
	assert_eq!(parse("$main"), Symbol("$main".to_string()));
	assert_eq!(parse("$ii_i"), Symbol("$ii_i".to_string()));
	assert_eq!(parse("(func $add (param i32))").serialize(), "(func $add (param i32))");
}

#[test]
fn dollar_digit_stays_a_parameter_reference() {
	assert_eq!(parse("$0"), Symbol("$0".to_string()));
}

#[test]
fn dollar_names_survive_eval() {
	assert_eq!(warp::wasm_emitter::eval("$main"), Symbol("$main".to_string()));
}
