// card check-assert: a `check` whose condition holds lets the program go on; `check` stays free as a function name.
// samples/polymorphism.warp combines numbers by typed variants (user: float stays IEEE)
use crate::is;

const COMBINE: &str = "combine(a:number, b:number) := a + b; combine(a:int, b:int) := a * b; ";

#[test]
fn a_check_that_holds_goes_on() {
	is!("check 1 == 1; 7", 7);
	is!("check 1.1 + 2.2 == 3.3; 7", 7);
	is!(&format!("{COMBINE}check combine(1.1, 2.2) == 3.3; check combine(2, 3) == 6; 7"), 7);
	is!("def check(n, i) = n + i; check(1, 2)", 3);
}
