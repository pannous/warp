// card fractional-durations: a decimal amount of a unit is exact, `0.3s` is 3/10 s (units.rs reads its decimal text):
// sleep takes it, a transition's duration too, and quantities compute with it
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_decimal_amount_of_a_unit_is_exact() {
	assert_eq!(eval("0.3 s in ms").serialize(), "300 ms");
	assert_eq!(eval("1.5 km + 20 m").serialize(), "1520 m");
	assert_eq!(eval("0.1 s + 0.2 s in ms").serialize(), "300 ms");
	is!("0.25km == 250 m", true);
}

#[test]
fn sleep_takes_a_decimal_duration() {
	is!("started = clock(); sleep 0.3s; clock() - started >= 300", true);
	is!("started = clock(); sleep(0.05 s); clock() - started >= 50", true);
}
