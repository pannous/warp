use warp::is;

#[test]
fn implicit_it_indexed_as_list() {
	is!("foo:=it#1;foo [1 2 3]", 1);
	is!("foo:=it#3;foo([4 5 6])", 6);
	is!("foo:=it[1];foo [4 5 6]", 5);
}

#[test]
fn named_parameter_indexed_as_list() {
	is!("foo(x):=x#2;foo [7 8 9]", 8);
}

#[test]
fn indexed_parameter_can_be_called_repeatedly_and_combined() {
	is!("foo:=it#1+it#2;foo [10 20 30]", 30);
	is!("second(xs):=xs#2;second([1 2 3]) + second([4 5 6])", 7);
}
