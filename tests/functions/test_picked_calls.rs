// A function value called where it is picked from a list: `fs[1](3)` calls the second function (it was the
// two-item list `closure_lambda_2 3`)
use crate::is;

#[test]
fn a_function_picked_from_a_list_is_called() {
	is!("fs = [x=>x+1, x=>x*2]; fs[1](3)", 6);
	is!("fs = [x=>x+1, x=>x*2]; fs#1(3)", 4);
	is!("fs = [x=>x+1, x=>x*2]; s = 0; for i in 0..4 { s += fs[i%2](i) }; s", 1 + 2 + 3 + 6);
}
