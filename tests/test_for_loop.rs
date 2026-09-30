use warp::*;

#[test]
fn test_for_over_list_literal() {
	is!("sum=0;for i in [1 2 3] {sum+=i};sum", 6);
}

#[test]
fn test_for_over_list_variable() {
	is!("xs=[4 5 6];sum=0;for x in xs {sum+=x};sum", 15);
}

#[test]
fn test_for_over_exclusive_range() {
	is!("sum=0;for i in 1..3 {sum+=i};sum", 3);
}

#[test]
fn test_for_over_inclusive_range() {
	is!("sum=0;for i in 1...3 {sum+=i};sum", 6);
	is!("sum=0;for i in 1 to 3 {sum+=i};sum", 6);
}

#[test]
fn test_for_with_colon_body() {
	is!("sum=0;for i in 1 to 3: sum+=i;sum", 6);
}

#[test]
fn test_for_classic_three_part_header() {
	is!("sum = 0; for(i=0;i<10;i++){sum+=i};sum", 45);
}

#[test]
fn test_nested_for() {
	is!("n=0;for a in [1 2] {for b in [1 2 3] {n+=1}};n", 6);
}
