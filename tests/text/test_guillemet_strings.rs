// «text» is a text that closes with », so it can hold " and ' unescaped (card guillemet-strings)
use crate::is;

#[test]
fn guillemets_close_with_the_right_one() {
	is!("«hello»", "hello");
	is!("x = «say \"hi\" and 'bye'»; x", "say \"hi\" and 'bye'");
	is!("«a» + «b»", "ab");
}
