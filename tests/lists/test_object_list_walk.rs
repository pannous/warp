// A loop or comprehension over a list of class instances (a cons-cell list, no typed array) reads `items#(i+1)` per
// step; each index walked from the head made a walk over 3000 objects take ~10^9 steps (a table's identity map, orm)
use crate::is;
use warp::util::with_fuel;

const WALKS: &str = "class P{age: int}\nps: [P] = []\nfor i in 1 to 3000 { ps += [P(i)] }\nkept = [p for p in ps if p.age > 2990]\ntotal = 0\nfor p in ps { total += p.age }\ncount(kept) + total";
/// the build takes under 10^7 steps; linear walks fit well within this, quadratic ones do not
const LINEAR_BUDGET: u64 = 100_000_000;

#[test]
fn walking_a_list_of_objects_takes_linear_steps() {
	with_fuel(LINEAR_BUDGET, || is!(WALKS, 10 + 3000 * 3001 / 2));
}

// the remembered position follows a list changed in place: an insert or removal before it shifts the items
#[test]
fn indexing_after_an_in_place_change_reads_the_changed_list() {
	is!("class P{age: int}\nps: [P] = [P(1), P(2), P(3)]\na = ps#3\nps.insert(P(9), 1)\nps#3.age", 2);
	is!("class P{age: int}\nps: [P] = [P(1), P(2), P(3)]\na = ps#3\nps.remove(ps#1)\nps#2.age", 3);
	is!("xs = [\"a\", \"b\", \"c\"]\na = xs#3\nxs.pop()\nb = xs#2\ncount(xs)", 2);
}
