// Free Int vars shared across if condition and then-arm (g-rT0c): without their kinds in return-kind inference the
// arm is typed Symbol/Text, the function returns a Node, and a numeric `+` of two calls traps. Fold can hide this by
// inlining a constant free var; these cases stay non-foldable (or then-only) so the typed path is what is tested.
use crate::is;
#[cfg(feature = "native")]
use crate::eq;
#[cfg(feature = "native")]
use warp::*;

#[cfg(feature = "native")]
fn compiled(code: &str) -> Node {
	let module = warp::pipeline::compile(code).expect("compiles");
	warp::wasm_reader::read_bytes(&module.bytes).expect("runs")
}

fn same_value(code: &str, value: i64) {
	is!(code, value);
	#[cfg(feature = "native")]
	eq!(compiled(code), value);
}

#[test]
fn non_foldable_capped_sum() {
	// `limit = n` is not a constant binding, so fold leaves the free var
	same_value("n=10; limit=n; def capped(x){ if x > limit then limit else x }; capped(3)+capped(30)", 13);
}

#[test]
fn then_arm_only_free_var_sum() {
	same_value("limit=10; def f(x){ if x > 0 then limit else x }; f(1)+f(2)", 20);
}

#[test]
fn global_capped_sum() {
	same_value("global limit=10; def capped(x){ if x > limit then limit else x }; capped(3)+capped(30)", 13);
}

#[test]
fn capped_single_and_semantics() {
	same_value("n=10; limit=n; def capped(x){ if x > limit then limit else x }; capped(13)", 10);
	same_value("n=10; limit=n; def capped(x){ if x > limit then limit else x }; capped(3)", 3);
}

#[test]
fn free_var_only_in_condition_stays() {
	same_value("n=10; limit=n; def f(x){ if x > limit then 99 else x }; f(3)+f(30)", 102);
}

#[test]
fn free_var_arithmetic_without_if() {
	same_value("n=10; limit=n; def f(x){x+limit}; f(3)+f(30)", 53);
}

#[test]
fn ternary_capped_sum() {
	same_value("n=10; limit=n; def c(x){ x > limit ? limit : x }; c(3)+c(30)", 13);
}
