use warp::is;

#[test]
fn for_without_variable_binds_it() {
	is!("x=0; for 1..4 {x+=it}; x", 6);
	is!("x=0; for 1...4 {x+=it}; x", 10);
	is!("x=0; for [5 6] {x+=it}; x", 11);
}

#[test]
fn upto_excludes_the_end_unlike_to() {
	is!("x=0; for i in 1 upto 4 {x+=i}; x", 6);
	is!("x=0; for i in 1 to 4 {x+=i}; x", 10);
}

#[test]
fn a_range_is_a_list_in_a_variable() {
	is!("r=1…3; r", warp::ints(vec![1, 2, 3]));
	is!("r=1 to 3; r", warp::ints(vec![1, 2, 3]));
	is!("r=1..3; r", warp::ints(vec![1, 2]));
	is!("x=0; r=1…4; for i in r {x+=i}; x", 10);
	is!("r=1…3; count r", 3);
}

#[test]
fn ranges_in_for_headers_stay_ranges() {
	is!("x=0; for i in 1..4 {x+=i}; x", 6);
	is!("x=0; for i in 1...4 {x+=i}; x", 10);
}
