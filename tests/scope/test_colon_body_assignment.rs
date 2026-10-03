use warp::*;

#[test]
fn test_while_colon_body_with_compound_assignment() {
	is!("i=0;while i<3: i+=2;i", 4);
	is!("i=9;while i>3: i-=2;i", 3);
}

#[test]
fn test_while_colon_body_with_assignment() {
	is!("i=0;while i<3: i=i+2;i", 4);
}

#[test]
fn test_if_colon_body_with_compound_assignment() {
	is!("x=1;if x>0: x+=1;x", 2);
	is!("x=1;if x>0: x-=1;x", 0);
}

#[test]
fn test_if_colon_body_with_assignment() {
	is!("x=1;if x>0: x=5;x", 5);
}

#[test]
fn test_for_colon_body_with_assignments() {
	is!("s=0;for i in [1 2 3]: s+=i;s", 6);
	is!("s=9;for i in [1 2 3]: s-=i;s", 3);
	is!("s=9;for i in [1 2 3]: s=i;s", 3);
}

#[test]
fn test_if_colon_body_assignment_stops_at_else() {
	is!("x=1;if x>0: x=5 else x=7;x", 5);
	is!("x=0;if x>0: x=5 else x=7;x", 7);
}
