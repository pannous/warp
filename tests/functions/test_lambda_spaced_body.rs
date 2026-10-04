// `=>` binds looser than the space: the words after the arrow are the lambda's body (`x => print x`)
use warp::is;

#[test]
fn a_lambda_body_may_be_a_spaced_call() {
	is!("f = (x => print x); f(3)", 3);
	is!("xs = [1 2]; xs.each(x => print x); 1", 1);
	is!("[1 2].map(x => print x)", warp::ints(vec![1, 2]));
	is!("f = (x => x + 1); f(3)", 4);
}
