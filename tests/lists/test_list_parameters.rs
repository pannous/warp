use crate::is;

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

#[test]
fn a_list_literal_assigned_to_a_parameter() {
	is!("def qp(arr, lo) { arr = [2]; return arr }; qp([1], 0)", warp::ints(vec![2]));
	is!("def qp(arr) { b = [2]; arr = b; arr }; qp([1])", warp::ints(vec![2]));
}
