// card xs-insert (user: "xs.insert(4) and xs.insert 4 should do the same thing from first principles"): a phrase
// whose head is a member, `xs.insert 4`, is the call of that member, `xs.insert(4)`, as `f x` is `f(x)`
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_member_with_arguments_is_its_call() {
	is!("xs = [1, 2]; xs.insert 4; xs", parse("[1 2 4]"));
	is!("xs = [1, 2]; xs.contains 2", true);
	is!("t = \"a-b\"; t.replace \"-\" \"+\"", "a+b");
}

#[test]
fn the_parenthesized_call_stays_the_same() {
	is!("xs = [1, 2]; xs.insert(4); xs", parse("[1 2 4]"));
}
