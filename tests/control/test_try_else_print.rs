//! card try-print: the fallback of `try X else Y` is a braceless call like the guarded part: `else print "msg"` prints
//! msg when X fails, and nothing when X succeeds
use crate::is;

#[test]
fn the_fallback_prints() {
	is!("try print 1/0 else print \"msg\"", warp::Node::Empty);
	is!("try print 1 + 2 else print \"msg\"", warp::Node::Empty);
	is!("y = try 1/0 else 2 + 3; y", 5);
}

#[cfg(all(unix, feature = "native"))]
#[test]
fn the_fallback_prints_its_text_unquoted() {
	let lines = |code: &str| crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(str::to_string).collect::<Vec<_>>();
	assert_eq!(lines("a = 1; b = 0\ntry print a / b else print \"msg\""), ["msg"]);
	assert_eq!(lines("try print 1 + 2 else print \"msg\""), ["3"]);
	assert_eq!(lines("try 1/0 else print \"a\" finally print \"z\""), ["a", "z"]);
}
