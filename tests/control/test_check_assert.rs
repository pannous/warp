// card check-assert (user): `check C` is `assert C`: it throws an error naming the failed condition, as written.
// samples/polymorphism.warp combines numbers (user: float stays IEEE, so a float variant would give 3.3000000000000003)
use crate::common::fails_with;
use crate::is;

const COMBINE: &str = "combine number with number = number#1 + number#2; combine int with int = $0 * $1; ";

#[test]
fn check_throws_when_the_condition_fails() {
	fails_with("check 1 == 2", "assertion failed: 1 == 2");
	fails_with("x = 3; check x > 5", "assertion failed: x > 5");
	fails_with("assert 1 > 2", "assertion failed: 1 > 2");
	fails_with("check 1 > 2 else \"too small\"", "too small");
}

#[test]
fn check_passes_a_condition_that_holds() {
	is!("check 1 == 1; 7", 7);
	is!("check 1.1 + 2.2 ≈ 3.3; 7", 7); // decimals are floats (decision exact-default)
	is!(&format!("{COMBINE}r = combine 1.1 with 2.2; check r ≈ 3.3; 7"), 7);
	is!(&format!("{COMBINE}check combine(1.1, 2.2) == 3.3; check combine(2, 3) == 6; 7"), 7);
	is!("def check(n, i) = n + i; check(1, 2)", 3);
}

#[test]
fn a_failed_check_stops_the_program() {
	fails_with("check 1 == 2; 7", "assertion failed: 1 == 2");
	fails_with("assert 1 == 2; 7", "assertion failed: 1 == 2");
	fails_with("x = 1; check x == 2; print \"after\"; 7", "assertion failed: x == 2");
	is!("try { check 1 > 2; 7 } else 9", 9);
}
