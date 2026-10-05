// P60 (user, 2026-10-05): the classical `try {…} catch {…}` (also `catch e`) and Python's `try: … except: …` are
// synonyms of `try X else Y`; function-level handlers come later
use crate::common::fails_with;
use warp::is;

#[test]
fn try_catch_is_try_else() {
	is!("try { [1 2]#5 } catch { 7 }", 7);
	is!("try { 5 } catch { 7 }", 5);
	is!("try { raise \"boom\" } catch e { 7 }", 7);
	fails_with("try { raise \"boom\" } catch e { e }", "boom"); // P67: e is the caught Error
}

#[test]
fn try_except_is_try_else() {
	is!("try: 1/0 except: 7", 7);
	is!("try:\n  1/0\nexcept:\n  7", 7);
	is!("try:\n  1/0\nexcept ZeroDivisionError:\n  7", 7);
	is!("x = 3\ntry:\n  x = 1/0\nexcept:\n  x = 4\nx", 4);
}
