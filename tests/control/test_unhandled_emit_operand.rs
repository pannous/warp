// card emit-operand-unhandled: an emit nobody handles gives no value; as an operand it is a compile error naming the
// event and the handler that would give one, not a run-time "not a number"
use crate::common::fails_with;
use crate::is;

#[test]
fn an_unhandled_emit_as_an_operand_names_its_handler() {
	fails_with("price(net) := net * (1 + emit tax_rate); price(100)", "no handler for event tax_rate");
	fails_with("price(net) := net * (1 + emit tax_rate); price(100)", "on tax_rate");
	fails_with("x = 1 + emit missing; x", "no handler for event missing");
}

#[test]
fn a_handled_emit_as_an_operand_gives_its_value() {
	is!("on tax_rate { 0.5 }; price(net) := net * (1 + emit tax_rate); price(100)", 150);
}
