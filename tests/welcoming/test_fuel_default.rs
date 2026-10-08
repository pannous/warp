// The default fuel lets ordinary 10^7-step loops finish; a runaway still stops with an error naming WARP_FUEL
// (card fuel-default). Filling a float list item by item costs ~1000 fuel a step, which put 10^7 of them right at the
// old default; a full 10^7 run takes half a minute in a debug build, so this runs 10^4 steps on their share of it
use warp::node::Node;
use warp::wasm_emitter::eval;
use warp::util::{with_fuel, DEFAULT_FUEL, FUEL_VARIABLE};

const ORDINARY_LOOP_STEPS: u64 = 10_000_000;
const MEASURED_STEPS: u64 = 10_000;
/// steps ten times this heavy still finish
const HEADROOM: u64 = 10;

#[test]
fn ten_million_float_fills_fit_the_default_fuel() {
	let share_of_default = DEFAULT_FUEL / (ORDINARY_LOOP_STEPS / MEASURED_STEPS) / HEADROOM;
	let fill = format!("xs = float[{MEASURED_STEPS}]; for i in 1 to {MEASURED_STEPS} {{ xs#i = i * 0.5 }}; xs#{MEASURED_STEPS}");
	assert_eq!(with_fuel(share_of_default, || eval(&fill)), eval(&format!("({MEASURED_STEPS} * 0.5) as float")));
}

#[test]
fn runaway_names_the_fuel_variable() {
	match with_fuel(1_000_000, || eval("i = 0; while 1 { i = i + 1 }")) {
		Node::Error(message) => assert!(format!("{message}").contains(FUEL_VARIABLE), "{message}"),
		other => panic!("expected out of fuel, got {other:?}"),
	}
}
