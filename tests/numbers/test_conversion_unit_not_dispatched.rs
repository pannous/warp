//! card print-km: in a program using the run-time Quantity class, the unit after `as` (`m/s`) is no division its
//! operator dispatch rewrites (`if m is Quantity then m.divide(s) …`), so a static conversion and a run-time one mix
#[cfg(feature = "native")]
use crate::common::printed;
use warp::wasm_emitter::eval;

#[test]
fn a_static_conversion_beside_a_run_time_quantity() {
	let code = "x = 1 km/h as m/s\nq = quantity(2, \"km\").to(\"m\")\n[x == 1 km/h, q.in(\"m\")]";
	assert_eq!(eval(code).serialize().trim(), "[yes 2000]");
}

#[cfg(feature = "native")]
#[test]
fn both_print() {
	assert_eq!(printed("print 1 km / 1 h as m/s\nprint quantity(2, \"km\").in(\"m\")"), "0.28m/s\n2000\n");
}
