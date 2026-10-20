// card room-conversions: a loop variable over lists, read once a round (`room[i] for room in rooms`), stays the Node
// list its cursor reads at once; made an array every round it walked each list whole and ran out of fuel
use crate::is;
use warp::util::with_fuel;

const ROOMS_OF_RANGES: &str = "n = 3000\nrooms = [0..n as list, 0..n as list, 0..n as list, 0..n as list]\nsums = [sum([room[i] for room in rooms]) for i in 0..n]\nsums[n - 1] + sums[7]";
/// linear reads fit well within this, an array made of each list every round does not
const LINEAR_BUDGET: u64 = 100_000_000;

#[test]
#[ignore = "card room-conversions: still out of fuel; a list made by `0..n as list` read by turns in a comprehension (a for loop is linear)"]
fn a_loop_variable_read_once_a_round_stays_a_node_list() {
	with_fuel(LINEAR_BUDGET, || is!(ROOMS_OF_RANGES, 4 * 2999 + 4 * 7));
}

#[test]
fn a_loop_variable_read_in_an_inner_loop_keeps_its_values() {
	is!(
		"rows = [[1, 2], [3, 4, 5]]; total = 0; for row in rows { for j in 1..(#row + 1) { total += row#j } }; total",
		15
	);
}
