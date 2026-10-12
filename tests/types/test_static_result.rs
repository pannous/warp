//! A constant call whose value the declared result type cannot hold is refused when compiled, as `x: int = 3/2` is
//! (card static-int); one whose value fits compiles (`half(4)` is 2)
use crate::is;
use warp::law::type_model::warp_verdict;

fn refused(code: &str, needle: &str) {
	match warp_verdict(code) {
		Err(why) => assert!(why.contains(needle), "{code}: {why}"),
		Ok(()) => panic!("{code} compiles, expected a refusal containing {needle:?}"),
	}
}

#[test]
fn test_misfit_constant_call_is_refused() {
	refused("def half(x) -> int { x / 2 }; half(3)", "half(3)");
	refused("int square(x) = x*x; square(4) + square 3.1", "square");
	refused("def square(x) as int = x*x; square 3.1", "declared int");
	refused("def f(x) -> int { x }; f(\"a\")", "declared int");
}

#[test]
fn test_fitting_constant_call_compiles() {
	assert_eq!(warp_verdict("def half(x) -> int { x / 2 }; half(4)"), Ok(()));
	is!("def half(x) -> int { x / 2 }; half(4)", 2);
	is!("int square(x) = x*x; square 4", 16);
}
