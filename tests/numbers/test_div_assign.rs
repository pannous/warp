// `x /= y` is exactly `x = x / y`; floor division is `x//=y`; a variable declared `x:int` keeps a whole number
// (user decision 2026-10-08, card div-assign)
use warp::wasm_emitter::eval;
use crate::is;

#[test]
fn divide_assign_divides_exactly() {
	assert_eq!(eval("x = 3; x /= 2; x"), eval("3/2"));
	assert_eq!(eval("x = 3; x /= 2; x"), eval("x = 3; x = x / 2; x"));
	assert_eq!(eval("x = 3.0; x /= 2; x"), eval("1.5"));
	assert_eq!(eval("def f(){ x = 3; x /= 2; x }; f()"), eval("3/2"));
	is!("x = 10; x /= 2; x", 5);
}

#[test]
fn floor_divide_assign_floors() {
	is!("x = 7; x//=2; x", 3);
	is!("x = -7; x//=3; x", -3);
}

#[test]
fn declared_int_keeps_dividing_whole() {
	is!("x:int = 3; x /= 2; x", 1);
}
