//! Card else-comment-swallows: a comment after `try … else v` (or `assert … else m`) ends the guard's line; the next
//! line is its own statement, not more of the fallback

use crate::is;

#[test]
fn a_comment_after_a_guard_ends_its_line() {
	is!("x = try { 1/0 } else 7 // fallback\nx", 7);
	is!("x = try { 1/0 } else 7 # fallback\nx + 1", 8);
	is!("y = 5\nx = try { 1/0 } else y // fallback\ny = 6\nx", 5);
	is!("x = try { 7 /! 2 } else 0\nx", 3);
}
