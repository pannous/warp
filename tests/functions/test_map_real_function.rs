// card map-sin: a real function maps over a list of floats, as a call argument and as a method
use warp::wasm_emitter::eval;

/// the browser's Math.sin may differ from libm in the last bit: compared within a tolerance
const TOLERANCE: f64 = 1e-12;

fn the_one_item(code: &str) -> f64 {
	let text = eval(code).serialize();
	let item = text.strip_prefix('[').and_then(|rest| rest.strip_suffix(']'));
	item.and_then(|number| number.parse().ok())
		.unwrap_or_else(|| panic!("{code} gave {text}, not a list of one number"))
}

#[test]
fn sin_maps_over_floats() {
	for code in ["map([4.0], sin)", "[4.0].map(sin)"] {
		assert!((the_one_item(code) - 4.0f64.sin()).abs() < TOLERANCE, "{code}");
	}
}
