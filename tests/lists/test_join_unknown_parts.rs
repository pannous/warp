// `out += part` of parts of unknown kind (a parameter `any`, lib/list.warp's flatten) grows `out` in place, as a known
// list does: copying the whole list at each step was quadratic (emitter.warp's concatenated: a third of parser.warp's
// pure-only compile, card warp-profile)
use crate::is;
use warp::util::with_fuel;

const JOINED: &str = "def joined(parts: any): list { out = []; for part in parts { out += part }; out }\n";
/// 3000 parts of two: a linear join takes well under 10^7 steps, the quadratic copies ~ 10^8
const LINEAR_BUDGET: u64 = 30_000_000;

#[test]
fn joining_parts_of_unknown_kind_takes_linear_steps() {
	let parts = "parts = (1 to 3000).map(i => [i, i])\n";
	with_fuel(LINEAR_BUDGET, || is!(&format!("{JOINED}{parts}count(joined(parts))"), 6000));
	with_fuel(LINEAR_BUDGET, || is!(&format!("{parts}count(flatten(parts))"), 6000));
}

#[test]
fn joining_parts_of_unknown_kind_keeps_their_items() {
	is!(&format!("{JOINED}joined([[1, 2], [], [3]])"), warp::ints(vec![1, 2, 3]));
	is!(&format!("{JOINED}joined([[\"a\"], [\"b\", \"c\"]]).join(\"\")"), "abc");
}

#[test]
fn adding_to_a_value_of_unknown_kind_is_its_sum() {
	const ADDED: &str = "def added(a: any, b: any) { a += b; a }\n";
	is!(&format!("{ADDED}added(1, 2)"), 3);
	is!(&format!("{ADDED}added(\"a\", \"b\")"), "ab");
	is!(&format!("{ADDED}added([1], [2])"), warp::ints(vec![1, 2]));
	is!(&format!("{ADDED}xs = [1]; added(xs, [2]); count(xs)"), 2); // a list grows in place, as `xs += [2]` does (P200b)
}
