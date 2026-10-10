// Card emit-operand: an emit is a value like a call: it stands as an operand of arithmetic, with or without
// parentheses, and the handler's value is a number, not a one-item block
use crate::is;

#[test]
fn an_emitted_value_is_the_handlers_number() {
	is!("on ask { 42 }; x = emit ask; x + 1", 43);
	is!("on ask { 7 }; compute() := emit ask; 1 + compute()", 8);
}

#[test]
fn an_emit_is_an_operand() {
	is!("on ask { 42 }; x = 1 + (emit ask); x", 43);
	is!("on ask { 42 }; x = 1 + emit ask; x", 43);
	is!("on ask { 2 }; 3 * emit ask + 1", 7);
	is!("on ask { 1 } in { 10 + emit ask }", 11);
}

// Card price-net: `ask` asks a handler for a value, as emit; the other emit words work in an operand too
#[test]
fn ask_is_an_emit_operand() {
	is!("price(net) := net * (1 + ask tax_rate); on tax_rate { 0.2 } in { price(100) }", 120);
	is!("on rate { 2 } in { 3 * fire rate }", 6);
	is!("ask(x) := x * 2; 1 + ask 3", 7);
}
