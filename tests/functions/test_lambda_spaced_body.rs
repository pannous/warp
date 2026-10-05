// `=>` binds looser than the space: the words after the arrow are the lambda's body (`x => print x`)
use warp::is;

#[test]
fn a_lambda_body_may_be_a_spaced_call() {
	is!("f = (x => print x); f(3)", 3);
	is!("xs = [1 2]; xs.each(x => print x); 1", 1);
	is!("[1 2].map(x => print x)", warp::ints(vec![1, 2]));
	is!("f = (x => x + 1); f(3)", 4);
}

#[test]
fn a_list_of_functions_holds_each_with_its_own_capture() {
	is!("fs = [(x => x + 1) (x => x + 2)]; fs#2(10)", 12);
	is!("adders = [1 2 3].map(n => (x => x + n)); f = adders#2; f(10)", 12);
	is!("adders = [1 2 3].map(n => (x => x + n)); adders#3(10)", 13);
	is!("fs = []; for k in 1..4 { fs.add(x => x * k) }; fs#2(10)", 20);
	is!("(x => x * 2)(5)", 10);
}
