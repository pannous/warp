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

#[test]
fn a_stored_error_fails_when_an_operation_runs_on_it() {
	// card error-value-kind (notes/type_theory.md Stored errors): its type is any, not text, so nothing is refused while
	// compiling and each operation raises the stored error itself when it runs
	for operation in ["r - 1", "r * 2", "r#1", "for x in r { print(x) }"] {
		crate::common::fails_with(&format!("r = error(\"bad value\"); {operation}"), "bad value");
	}
	is!("r = error(\"bad value\"); if 0 then r - 1 else 5", 5);
	is!("r = error(\"bad value\"); type(r) == error", true);
}
