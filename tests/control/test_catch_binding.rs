// P67 (user, 2026-10-05): in `catch e { … }` and `except E as e:` the name is the caught Error value (its message): a
// raised value, an Error a call gave back, or the named runtime error
use crate::common::fails_with;
use crate::is;

#[test]
fn catch_binds_the_error() {
	fails_with("try { raise \"boom\" } catch e { e }", "boom");
	fails_with("try { [1 2]#5 } catch e { e }", "index out of range");
	fails_with("f() := error(\"bad\"); try f() catch e { e }", "bad");
	is!("try { [1 2]#5 } catch e { is_error(e) }", 1);
	is!("r = try { raise \"boom\" } catch e { 1 }; r", 1);
}

#[test]
fn except_as_binds_the_error() {
	fails_with("try:\n  [1 2]#5\nexcept IndexError as e:\n  e", "index out of range");
	is!("try:\n  [1 2]#5\nexcept:\n  7", 7);
}

// card try-raise: a caught Error joined with text reads as its message
#[test]
fn a_caught_error_reads_as_its_message_in_text() {
	is!("try { raise \"boom\" } catch e { \"caught: \" + e }", "caught: boom");
	is!("try { [1 2]#5 } catch e { e + \"!\" }", "index out of range!");
}

// card try-raise: whatever was raised, the caught Error's message is its text
#[test]
fn a_raised_value_is_caught_as_its_text() {
	is!("try { raise 42 } catch e { \"caught \" + e }", "caught 42");
	fails_with("raise 42", "42");
}

// card try-raise: an interpolation hole reads the caught Error too
#[test]
fn an_interpolation_reads_the_caught_error() {
	is!("try { raise \"boom\" } catch e { \"caught: \\(e)\" }", "caught: boom");
}
