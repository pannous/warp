// card enum-parameter-type: an enum names the type of its cases, `f(c: Coin)`; a case of `enum Coin { Penny, Dime }`
// stays its number (Coin.Dime is 1, test_records_classes.rs), so Coin is a type of ints
use crate::is;

const COIN: &str = "enum Coin { Penny, Nickel, Dime }";

fn with_coin(code: &str) -> String {
	format!("{COIN}; {code}")
}

#[test]
fn an_enum_is_a_parameter_type() {
	is!(&with_coin("f(c: Coin) := c; f(Coin.Dime)"), 2);
	is!(
		&with_coin("cents(c: Coin) := match c { Coin.Penny: 1; Coin.Nickel: 5; _: 10 }; cents(Coin.Nickel)"),
		5
	);
}

#[test]
fn an_enum_is_a_field_and_variable_type() {
	is!(
		&with_coin("class Purse { coin: Coin }; p = Purse(Coin.Dime); p.coin"),
		2
	);
	is!(&with_coin("c: Coin = Coin.Nickel; c"), 1);
}
