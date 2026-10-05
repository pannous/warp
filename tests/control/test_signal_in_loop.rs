// `once x == 5 {…}` with x written as the only statement of a loop body: the check after the write made the body a
// juxtaposition (`x = i check`), "not an int"; it is a sequence now
use warp::*;

#[test]
fn a_listener_checks_a_write_in_a_one_statement_loop() {
	is!("x = 1; hits = 0; once x == 5 { hits += 1 }; for i in 1..10 { x = i }; x + hits * 100", 109);
	is!("x = 1; once x == 5 { print \"halfway\" }; for i in 1..10 { x = i }; x", 9);
}
