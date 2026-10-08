// Card catch-message: a caught Error is a value with a message: `e.message` is its text, `print e` prints it, and
// `x is error` tests any value at run time
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn an_error_has_a_message() {
	is!("try { raise \"boom\" } catch e { e.message }", "boom");
	is!("try { [1 2]#5 } catch e { e.message }", "index out of range");
	is!("e = error(\"bad\"); e.message", "bad");
}

#[test]
fn printing_an_error_prints_its_message() {
	is!("try { raise \"boom\" } catch e { print e; 1 }", 1);
	is!("e = error(\"bad\"); print e; 2", 2);
}

#[test]
fn is_error_tests_any_value() {
	is!("r = error(\"bad\"); r is error", true);
	is!("x = 3; x is error", false);
	is!("try { raise \"boom\" } catch e { e is error }", true);
	is!("\"bad\" is error", false);
	assert_eq!(eval("try { raise \"boom\" } catch e { e is error }").serialize(), "yes");
}
