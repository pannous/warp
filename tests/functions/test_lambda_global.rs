//! A lambda declaring `global n` changes main's n as a def does (card closure-global, found by warp-ed)
use crate::is;

#[test]
fn a_lambda_with_global_changes_mains_variable() {
	is!("n = 0; f = () => { global n; n += 1 }; f(); f(); n", 2);
	is!("global n = 0; f = () => { global n; n += 1 }; f(); f(); f(); n", 3);
	is!("n = 0; def f() { global n; n += 1 }; f(); f(); n", 2);
}
