// Several long lists read by turns (`[sum([room[i] for room in rooms]) for i in 0..n]`, lib/sound.warp's reverb over
// four combs): each keeps its own read cursor, so every read walks on from the last one instead of from the head of
// all but one list (card quadratic-out: a 2.6 s reverb chord ran out of 10^11 steps of fuel)
use crate::is;
use warp::util::with_fuel;

const BY_TURNS: &str = "n = 3000\nrooms = [[i for i in 0..n], [2 * i for i in 0..n], [3 * i for i in 0..n], [4 * i for i in 0..n]]\nsums = [sum([room[i] for room in rooms]) for i in 0..n]\nsums[n - 1] + sums[7]";
/// linear reads fit well within this, walks from the head (n²/2 links per list) do not
const LINEAR_BUDGET: u64 = 100_000_000;

#[test]
fn lists_read_by_turns_take_linear_steps() {
	with_fuel(LINEAR_BUDGET, || is!(BY_TURNS, 10 * 2999 + 10 * 7));
}

// a list changed in place forgets every cursor: the reads after see the changed lists
#[test]
fn lists_read_by_turns_after_an_in_place_change() {
	is!("a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]\nb = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120]\nx = a#10 + b#10\na.insert(0, at: 1)\nb.insert(0, at: 1)\na#10 + b#10", 99);
}
