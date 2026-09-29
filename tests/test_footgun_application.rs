use warp::*;

#[test]
fn test_juxtaposition_application_inside_addition() {
	is!("f := it*10; 1 + f 3", 31);
}

#[test]
fn test_juxtaposition_application_binds_looser_than_arithmetic_argument() {
	is!("square:=it^2;1+square 2+3", 26);
}
