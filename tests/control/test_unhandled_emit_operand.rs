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

// review of fix-emit-operand: any unhandled emit used as a value, not only as an operator's operand
#[test]
fn an_unhandled_emit_used_as_a_value_names_its_handler() {
	fails_with("x = emit missing; x + 1", "no handler for event missing");
	fails_with("f(x) := x; f(emit missing)", "no handler for event missing");
	fails_with("xs = [emit missing]; xs", "no handler for event missing");
	fails_with("g() := { return emit missing }; g()", "no handler for event missing");
}

#[test]
fn an_unhandled_emit_as_a_statement_still_only_notes() {
	is!("emit missing; 1", 1);
	is!("level(x) := { if x > 0 { emit missing }; 2 }; level(1)", 2);
}
