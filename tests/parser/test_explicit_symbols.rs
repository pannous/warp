//! card explicit-symbol (wiki/symbol.md): `:name` is the symbol name, never evaluated; whitespace tells it from a key,
//! `a:b` is a key, `a :b` the two items a and :b
use crate::is;

#[test]
fn a_colon_name_is_a_symbol() {
	is!("x = :src; x == symbol(\"src\")", true);
	is!("type(:src)", "symbol");
	is!("src = 3; x = :src; x == symbol(\"src\")", true);
}

#[test]
fn colon_names_uncharge_operators() {
	is!("count([:a :plus :b])", 3);
	is!("[:a :plus :b]#2 == symbol(\"plus\")", true);
}

#[test]
fn a_colon_without_space_before_stays_a_key() {
	is!("{a:1}.a", 1);
	is!("{a: 1}.a", 1);
	is!("x = {name : 2}; x.name", 2);
}

/// reviewer (explicit-symbol veto): after any other operand ` :name` keeps its meaning
#[test]
fn a_blank_colon_after_an_operand_keeps_its_meaning() {
	is!("y = 5\n1 < 0 ? 2 :y", 5);
	is!("x = 1 > 0 ? 2 :y; x", 2);
	is!("class P{ n :int }\nP(3).n", 3);
	is!("p = {name :ann}; p.name", "ann");
}
