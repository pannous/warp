//! Ratios whose numerator or denominator exceed i64 stay exact as Number::BigQuotient (g-qU7E).
use num_bigint::BigInt;
use warp::extensions::numbers::Number;
use warp::extensions::reals::Rational;
use warp::wasm_emitter::eval;
use warp::{is, Node};

fn big_ratio(numerator: &str, denominator: &str) -> Node {
	let ratio = Rational::new(numerator.parse::<BigInt>().unwrap(), denominator.parse::<BigInt>().unwrap());
	Node::Number(Number::from_rational(ratio))
}

#[test]
fn test_big_ratio_readback_stays_exact() {
	is!("(0-2^70)/3", big_ratio("-1180591620717411303424", "3"));
	is!("2^70/3", big_ratio("1180591620717411303424", "3"));
	is!("y=(0-2^70)/3; y as string", "-1180591620717411303424/3");
	is!("y=2^70/3; y as string", "1180591620717411303424/3");
}

#[test]
fn test_big_ratio_terminating_decimal() {
	// 1/(2^70) terminates (denominator is a power of 2) and prints as an exact decimal
	let decimal = "0.0000000000000000000008470329472543003390683225006796419620513916015625";
	is!("1/(2^70)", big_ratio("1", "1180591620717411303424"));
	is!("y=1/(2^70); y as string", decimal);
	assert_eq!(eval("1/(2^70)").serialize(), decimal);
}

#[test]
fn test_big_ratio_unit_amounts_stay_exact() {
	// 2^62 km / 7 mm → (2^62 · 10^6)/7, beyond i64, stays BigQuotient
	assert_eq!(eval("4611686018427387904 km / 7 mm").serialize(), "4611686018427387904000000/7");
	assert_eq!(eval("4611686018427387904 m / 7").serialize(), "4611686018427387904/7 m");
}
