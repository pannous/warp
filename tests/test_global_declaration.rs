// `global x` without initializer declares a zero-initialized global that later statements assign
use warp::*;

#[test]
fn test_global_without_initializer() {
	is!("global x;x=7;x", 7);
	is!("global x;x=7;x+1", 8);
	is!("global k;k = 7", 7);
	is!("global x;x", 0);
}

#[test]
fn test_compound_assignment_on_a_global() {
	is!("global x=7;x+=1", 8);
	is!("global x=7;x+=1;x", 8);
	is!("global x=7;x*=2;x", 14);
	is!("global x=7;x-=2;x", 5);
	is!("global x=7;x++;x", 8);
	is!("global x=7.5;x+=1;x", 8.5);
}
