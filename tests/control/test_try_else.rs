use warp::is;
use warp::wasm_emitter::eval;

fn error_text(code: &str) -> String {
	match eval(code) {
		warp::Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

#[test]
fn try_keeps_a_value() {
	is!("try 3 else 0", 3);
	is!("try [1 2]#2 else 0", 2);
	is!("try 7%2 else 0", 1);
	is!("try 7/7 else 0", 1);
	is!("x=4; try x else 9", 4);
}

#[test]
fn try_falls_back_when_the_index_is_out_of_range() {
	is!("try [1 2]#5 else 0", 0);
	is!("try [1 2]#0 else 0", 0);
	is!("l=[1 2 3]; i=9; try l#i else 8", 8);
}

#[test]
fn try_falls_back_on_division_and_modulo_by_zero() {
	is!("try 1%0 else 7", 7);
	is!("try 1/0 else 7", 7);
	is!("a=5; b=0; try a%b else 7", 7);
}

#[test]
fn try_of_an_assignment_assigns_the_fallback() {
	is!("try item = [1 2]#5 else 0; item", 0);
	is!("try item = [1 2]#2 else 0; item", 2);
}

#[test]
fn try_falls_back_on_an_error_value() {
	is!("try error(\"no\") else \"yes\"", "yes");
	is!("try \"fine\" else \"yes\"", "fine");
}

#[test]
fn assert_gives_the_condition_or_an_error() {
	is!("assert 2>1 else \"x\"", 1);
	assert!(error_text("assert 1>2 else \"out of stock\"").contains("out of stock"));
	assert!(error_text("assert 1>2").contains("assertion failed"));
	is!("assert 1", 1);
}
