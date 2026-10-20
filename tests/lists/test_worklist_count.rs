// card list-count-grows: a worklist grown in place while its count is read (`while at <= #waiting { … waiting += [x] }`,
// prelude_words.warp's symbol_words) keeps its count: list_extend forgot it, so each step counted the whole list again
// (half of passes.warp's pure-only run). Values built at run time (random) so nothing folds.
use crate::is;
use warp::util::with_fuel;

/// ~1 M steps with the count kept, hundreds of millions counting the list at every step
const LINEAR_BUDGET: u64 = 30_000_000;

#[test]
fn a_worklist_keeps_its_count_while_it_grows() {
	let worklist = "waiting = [0 + round(random() * 0)]\nat = 1\nwhile at <= #waiting {\n\tx = waiting#at\n\tat += 1\n\tif x < 3000 { waiting += [x + 1] }\n}\n#waiting";
	with_fuel(LINEAR_BUDGET, || is!(worklist, 3001));
	// the count after other changes: an append of several items, of none, onto ø
	is!("xs = [1]; n = #xs; xs += [2, 3]; xs += []; [n, #xs]", warp::ints(vec![1, 3]));
	is!("xs = ø; n = #xs; xs += [7]; [n, #xs]", warp::ints(vec![0, 1]));
}
