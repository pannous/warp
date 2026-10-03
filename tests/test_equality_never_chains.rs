//! #39 (user, 2026-10-03): `==` and `!=` never chain with `<` `>`: `1<2==2` is `(1<2)==2`; `a<b<c` still chains
use warp::*;

#[test]
fn equality_compares_the_result_of_an_ordering() {
	is!("1<2==2", false);
	is!("1<2==1", true);
	is!("3>2!=1", false);
	is!("2==2<3", false);
}

#[test]
fn orderings_still_chain() {
	is!("1<2<3", true);
	is!("3>2>1", true);
	is!("1<3<2", false);
	is!("1<=1<2", true);
}
