//! `do <block>` runs the block on the spot, like the evaluating `!`

use warp::is;

#[test]
fn do_runs_a_block_literal() {
	is!("do {1+2}", 3);
	is!("x=0; do {x=5}; x", 5);
}

#[test]
fn do_runs_a_named_block() {
	is!("f:={1+2}; do f", 3);
	is!("f:={1+2}; f!", 3);
	is!("x=4; f:={x*2}; do f", 8);
}

#[test]
fn do_still_joins_a_while_condition_with_its_body() {
	is!("i=0; while i<3 do {i++}; i", 3);
}

#[test]
fn a_user_function_named_do_wins() {
	is!("do(a):=a*2; do(4)", 8);
}
