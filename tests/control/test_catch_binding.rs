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
