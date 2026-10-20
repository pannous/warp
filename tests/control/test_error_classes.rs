// Error classes (decision error-classes, notes/error_classes.md): `class ParseError: Error` declares one, `raise
// ParseError("…")` raises it, `except C` catches C and the classes extending it and lets every other error pass on;
// runtime errors are of Python's classes (IndexError, ZeroDivisionError, …)
use crate::common::fails_with;
use crate::is;

#[test]
fn except_catches_its_class_and_lets_others_pass() {
	is!("try 1/0 except ZeroDivisionError: 7", 7);
	is!("try (try 1/0 except IndexError: 1) except ZeroDivisionError: 2", 2);
	is!("try (try [1 2]#5 except ZeroDivisionError: 1) except Error: 3", 3);
	is!("try [1]#3 except LookupError: 4", 4);
	fails_with("try 1/0 except IndexError: 1", "divide by zero");
}

#[test]
fn a_declared_error_class_raises_and_catches() {
	let classes = "class ParseError: Error\nclass TokenError extends ParseError {}\n";
	is!(&format!("{classes}try raise ParseError(\"bad token\") except ParseError as e: e.message"), "bad token");
	is!(&format!("{classes}try raise TokenError(\"bad char\") except ParseError as e: e.message"), "bad char");
	is!(&format!("{classes}try (try raise ParseError(\"x\") except TokenError: 1) except Error: 2"), 2);
	fails_with(&format!("{classes}try raise ParseError(\"x\") except IndexError: 1"), "x");
}

#[test]
fn an_undeclared_error_class_is_named() {
	fails_with("try raise \"boom\" except FooError: 1", "declare it with `class FooError: Error`");
	fails_with("raise FooError(\"x\")", "declare it with `class FooError: Error`");
}
