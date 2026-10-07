// card exact-global-widen: a main-level variable started exact (`b = 0.5`) that a function gives a float holds an f64
// throughout, as a main-level `b = b + random()` already widens it
use crate::is;

#[test]
fn a_function_assigning_a_float_widens_an_exact_global() {
	is!("b = 0.5; def t() { global b; b = b + random() }; t(); b > 0", 1);
	is!("b = 0.5; def t() { global b; b = b + random(); b }; t() > 0", 1);
	is!("b = 0.5; def t() { global b; b += random() }; t(); b > 0", 1);
	is!("global b = 0.5; def t() { b = b + random() }; t(); b > 0", 1);
}

#[test]
fn exact_updates_keep_a_global_exact() {
	is!("b = 0.5; def t() { global b; b = b + 0.25 }; t(); b", 0.75);
	is!("b = 1; def t() { global b; b = b * 2 }; t(); b", 2);
}
