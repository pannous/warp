// card big-int: & | xor of Ints beyond i64 work on the whole two's complement number, not mod 2^64
use crate::is;
use warp::parse;

#[test]
fn bits_of_ints_beyond_i64() {
	is!("2^70 | 1", parse("1180591620717411303425"));
	is!("2^70 & (2^70 + 5)", parse("1180591620717411303424"));
	is!("2^70 xor (2^64 + 3)", parse("1199038364791120855043"));
}

#[test]
fn bits_of_negative_ints_beyond_i64() {
	is!("-(2^70) | 1", parse("-1180591620717411303423"));
	is!("-(2^70) & (2^71 - 1)", parse("1180591620717411303424"));
	is!("-(2^70) xor -1", parse("1180591620717411303423"));
}

#[test]
fn bits_of_small_ints_stay() {
	is!("6 & 3", 2);
	is!("6 | 3", 7);
	is!("5 xor 3", 6);
	is!("-6 & 7", 2);
	is!("2^62 | 2^61", parse("6917529027641081856"));
}
