//! card print-closure: print shows a function value: a named function by its name, a lambda as written
#[test]
#[cfg(feature = "native")]
fn print_shows_a_closure_as_written() {
	let printed = crate::common::printed("make(n) := x => x + n; print make(1)");
	assert_eq!(printed.trim(), "x => x+n");
}

#[test]
#[cfg(feature = "native")]
fn print_shows_a_list_of_functions() {
	let printed = crate::common::printed("f(x) := x + 1; print [&f, x => x * 2, (a, b) => a + b]");
	assert_eq!(printed.trim(), "[f x => x*2 (a, b) => a+b]");
}

/// card print-function-ref: `&f` and an alias of it are the function f, printed on its own as its definition (user
/// decision on card module-methods; in a list by its name)
#[test]
#[cfg(feature = "native")]
fn print_shows_a_function_reference() {
	assert_eq!(crate::common::printed("f(x) := x + 1; print &f").trim(), "f(x) := x+1");
	assert_eq!(crate::common::printed("f(x) := x + 1; g = &f; print g").trim(), "f(x) := x+1");
	assert_eq!(crate::common::printed("f(x) := x + 1; g = &f; print g(2)").trim(), "3");
	assert_eq!(crate::common::printed("f(x) := x + 1; g = &f; print g 2").trim(), "3");
}
