//! Errors a program raised while running: the messages for traps and `raise`, and the hints added to them.

use crate::node::Node;





/// The run computed past its time limit: it probably does not terminate, or needs more time
pub fn out_of_time(limit: std::time::Duration) -> Node {
	crate::node::error(&format!(
		"out of time after {} s of computing in one turn: the program may not terminate (give it more with {}=<seconds>, or count steps with warp fuel <steps>)",
		limit.as_secs_f64(),
		crate::util::TIME_LIMIT_VARIABLE))
}

/// The run used up its fuel: it probably does not terminate, or needs a larger budget
pub fn out_of_fuel(steps: u64) -> Node {
	crate::node::error(&format!(
		"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or warp fuel <steps>)",
		crate::util::FUEL_VARIABLE))
}
