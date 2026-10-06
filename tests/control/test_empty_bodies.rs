// An empty body `{}` is a function or handler that does nothing and returns ø (card empty-timer: `on every 1000 ms {}`
// failed with "cannot extract a numeric value from {}")
use crate::is;

#[test]
fn an_empty_function_body_returns_nothing() {
	is!("f() := {}; f(); 3", 3);
	is!("f() := { {} }; f(); 3", 3);
}

#[test]
fn an_empty_timer_body_does_nothing() {
	is!("n = 1; on every 1000 ms {}; n", 1);
}
