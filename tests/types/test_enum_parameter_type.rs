// card enum-parameter-type: an enum names the type of its cases, `f(c: Coin)`; a case is its own value of that type
// (user decision enum-case B, test_plain_enum.rs)
use crate::is;
use warp::symbol;

const COIN: &str = "enum Coin { Penny, Nickel, Dime }";

fn with_coin(code: &str) -> String {
	format!("{COIN}; {code}")
}

#[test]
fn an_enum_is_a_parameter_type() {
	is!(&with_coin("f(c: Coin) := c; f(Coin.Dime)"), symbol("Dime"));
	is!(&with_coin("cents(c: Coin) := match c { Coin.Penny: 1; Coin.Nickel: 5; _: 10 }; cents(Coin.Nickel)"), 5);
}

#[test]
fn an_enum_is_a_field_and_variable_type() {
	is!(&with_coin("class Purse { coin: Coin }; p = Purse(Coin.Dime); p.coin"), symbol("Dime"));
	is!(&with_coin("c: Coin = Coin.Nickel; c"), symbol("Nickel"));
}
