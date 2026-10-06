// Swift closures: `{ x in x*2 }`, `{ a, b in a+b }` and the shorthand arguments `{ $0 * 2 }`, `{ $0 + $1 }`
// (`$0` in a block was an internal error: WASM validation failed)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn swift_closure_parameters() {
	is!("f = { x in x*2 }; f(3)", 6);
	is!("f = { a, b in a+b }; f(2, 3)", 5);
	assert_eq!(eval("map([1,2,3], { x in x*x })").serialize(), "[1 4 9]");
}

#[test]
fn swift_shorthand_arguments() {
	assert_eq!(eval("map([1,2,3], { $0 * 2 })").serialize(), "[2 4 6]");
	is!("f = { $0 + $1 }; f(1, 2)", 3);
	is!("fold([1,2,3], 0, { $0 + $1 })", 6);
	assert_eq!(eval("xs=[3,1,2]; sorted(xs, by: { $0 > $1 })").serialize(), "[3 2 1]");
}

#[test]
fn membership_in_a_block_stays_membership() {
	is!("xs=[1,2]; x=2; { x in xs }", 2); // the position, as `x in xs` gives it
}
