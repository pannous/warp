// A list variable of any elements that is indexed, counted or iterated is held as an array ($NodeList): O(1) index and
// count, amortised O(1) append, so loops over lists are linear (a 3000 item loop was seconds, quadratic, before)
use warp::{ints, is};

#[test]
fn loops_over_a_list_are_linear() {
	is!("xs=(0..3000).map(x=>x); s=0; for x in xs { s+=x }; s", 4498500);
	is!("xs=(0..3000).map(x=>x); s=0; for i in 0..3000 { s+=xs[i] }; s", 4498500);
	is!("xs=(0..3000).map(x=>x); for i in 0..3000 { xs[i]=1 }; sum xs", 3000);
	is!("xs=(0..3000).map(x=>x); count(xs.filter(x=>x>10))", 2989);
}

#[test]
fn a_node_list_keeps_its_elements_and_values() {
	is!("xs=[\"ab\" \"cd\"]; xs#2", "cd");
	is!("xs=[[1 2] [3]]; xs#1", ints(vec![1, 2]));
	is!("xs=(0..5).map(x=>x); ys=xs; ys#1=9; xs#1*10 + ys#1", 9);
	is!("def f(xs){ s=0; for x in xs { s+=x }; s }; f([1 2 3])", 6);
	is!("a=(1 2);b=(3 4);c=a+b;#c", 4);
	is!("a=[1,2,3,4]; b=a[:1] + a[2:]; count(b)*10 + b#2", 33);
}

#[test]
fn a_list_parameter_indexed_in_a_loop_is_linear() {
	is!("def g(xs){ s=0; for i in 0..#xs { s+=xs#(i+1) }; s }; g((0..3000).map(x=>x))", 4498500);
	is!("def swap(arr, i, j) { temp = arr[i]; arr[i] = arr[j]; arr[j] = temp; return arr }; swap([1 2 3], 0, 2)", ints(vec![3, 2, 1]));
}
