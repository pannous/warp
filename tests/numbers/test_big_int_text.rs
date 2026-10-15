//! Card out-fuel: the text of a big integer is written 18 digits per division, so 10^10000 prints at once
use crate::is;

#[test]
fn test_big_int_text() {
	is!("x = 10^10000; count(text(x))", 10001);
	is!("x = 10^10000; t = text(x); [t#1, t#2, t#10001]", warp::parse(r#"["1" "0" "0"]"#));
	is!("x = -(10^2000) - 7; t = text(x); [count(t), t#1, t#2, t#(count(t))]", warp::parse(r#"[2002 "-" "1" "7"]"#));
	is!("text(123456789012345678901234567890)", "123456789012345678901234567890");
	is!("x = 10^18; text(x)", "1000000000000000000");
	is!("x = 10^36 + 5; text(x)", "1000000000000000000000000000000000005");
	is!("x = -(10^18) + 1; text(x)", "-999999999999999999");
	is!("text(0)", "0");
}
