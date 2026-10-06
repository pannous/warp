// User decision #34 (2026-10-03, "Do it now"): `try X else Y` also catches a runtime error deep inside X
use crate::is;
use warp::wasm_emitter::eval;

fn error_text(code: &str) -> String {
	match eval(code) {
		warp::Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

#[test]
fn try_catches_an_error_inside_an_expression() {
	is!("try 1 + [1 2]#5 else 0", 0);
	is!("xs=[1 2]; try xs#1 + xs#5 else 9", 9);
	is!("try 1 + [1 2]#2 else 0", 3);
}

#[test]
fn try_catches_an_error_inside_a_called_function() {
	is!("f(i)=[1 2]#i; try f(5) + 1 else 7", 7);
	is!("f(i)=[1 2]#i; try f(2) + 1 else 7", 3);
}

#[test]
fn try_catches_a_failed_cast() {
	is!("x=\"a\"; try (x as int) + 1 else 0", 0);
}

#[test]
fn caught_errors_leave_later_errors_loud() {
	is!("a = try 1 + [1 2]#5 else 4; a + 1", 5);
	assert!(error_text("a = try 1 + [1 2]#5 else 4; [1 2]#a").contains("index out of range"));
	assert!(error_text("1 + [1 2]#5").contains("index out of range"));
}

#[test]
fn nested_try_restores_the_depth_on_both_paths() {
	is!("try (try 1 + [1 2]#5 else 2) + [1 2]#7 else 3", 3);
	is!("try (try 1 + [1 2]#5 else 2) + 1 else 3", 3);
	is!("try (try 1 + [1 2]#1 else 2) + 1 else 3", 3);
	assert!(error_text("(try 1 + [1 2]#5 else 2) + [1 2]#7").contains("index out of range"));
	assert!(error_text("(try 1 + [1 2]#1 else 2) + [1 2]#7").contains("index out of range"));
}

#[test]
fn try_inside_a_called_function_restores_the_depth() {
	is!("g(i)=try 1 + [1 2]#i else 0; g(5) + g(1)", 2);
	assert!(error_text("g(i)=try 1 + [1 2]#i else 0; g(5) + [1 2]#9").contains("index out of range"));
	assert!(error_text("g(i)=try 1 + [1 2]#i else 0; g(1) + [1 2]#9").contains("index out of range"));
}

#[test]
fn try_in_a_loop_catches_each_time() {
	is!("s=0; for i in [1 2 3 4] { s = s + (try [10 20]#i else 1) }; s", 32);
}
