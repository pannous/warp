// Card out-list: `out = out + x` of a list local nothing else holds grows it in place, as `out += x` does: a copy at
// each step was quadratic. A list another name, a caller or a list holds stays a copy (P200b: `+=` is seen by them all)
use crate::is;
use warp::util::with_fuel;
use warp::{ints, parse};

const GATHERED: &str = "def gathered(n) { out = []; for i in 1 to n { out = out + [i, i] }; out }\n";
/// 3000 steps of two items: a linear gathering takes well under 10^7 steps, the quadratic copies ~ 10^8
const LINEAR_BUDGET: u64 = 30_000_000;

#[test]
fn a_self_sum_of_an_unshared_list_takes_linear_steps() {
	with_fuel(LINEAR_BUDGET, || is!(&format!("{GATHERED}count(gathered(3000))"), 6000));
	is!(&format!("{GATHERED}gathered(2)"), ints(vec![1, 1, 2, 2]));
	is!("def words() { out = []; for w in [\"a\" \"b\"] { out = out + w }; out }\nwords()", parse("[\"a\" \"b\"]"));
}

#[test]
fn a_self_sum_of_a_shared_list_leaves_its_holders_alone() {
	is!("def kept() { a = [1]; b = a; a = a + [2]; b }\nkept()", ints(vec![1]));
	is!("def held() { a = [1]; m = [a]; a = a + [2]; m#1 }\nheld()", ints(vec![1]));
	is!("def given(xs) { xs = xs + [2]; xs }\nys = [1]; given(ys); ys", ints(vec![1]));
	is!("def walked() { a = [1, 2]; for v in a { a = a + [v] }; a }\nwalked()", ints(vec![1, 2, 1, 2]));
}
