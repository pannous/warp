// `global x` without initializer declares a zero-initialized global that later statements assign
use warp::*;

#[test]
fn test_global_without_initializer() {
	is!("global x;x=7;x", 7);
	is!("global x;x=7;x+1", 8);
	is!("global k;k = 7", 7);
	is!("global x;x", 0);
}
