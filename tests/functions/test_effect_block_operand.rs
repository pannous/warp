//! Card operand-effect: `on ev {…} in {…}` is an operand like any value: of an operator, an argument of print.

use crate::common::printed;
use crate::is;

#[test]
fn effect_block_as_operand() {
	is!("price(net) := net * (1 + emit tax)\n\"total \" + on tax { 0.5 } in { price(100) }", "total 150");
	is!("y = 1 + 2 * on t { 3 } in { emit t }\ny", 7);
	assert!(printed("print on t { 3 } in { emit t }").lines().any(|line| line == "3"));
}
