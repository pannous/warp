// The text of an exact real keeps its symbolic form (user decision 2026-10-03: "√2 if we preserve that information
// symbolically"): `"f" + sqrt(2)` is "f√2", not its source or an f64
use warp::is;

#[test]
fn an_exact_real_joins_a_text_symbolically() {
	is!("\"f\" + sqrt(2)", "f√2");
	is!("√2 + \"x\"", "√2x");
	is!("\"a\" + π", "aπ");
	is!("x=√2*3; \"v=\" + x", "v=3√2");
}

#[test]
fn an_exact_real_as_text_is_symbolic() {
	is!("str(√2)", "√2");
	is!("sqrt(2) as string", "√2");
	is!("π/2 as string", "π/2");
}
