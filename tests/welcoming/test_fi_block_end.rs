use crate::common::printed;
use crate::is;

#[test]
fn fi_closes_a_multiline_if_block() {
	for closer in ["end", "fi"] {
		let source = format!("if 3 > 2\n\tprint \"yes\"\n\tprint \"sure\"\n{closer}");
		assert_eq!(printed(&source), "yes\nsure\n");
		let source = format!("if 2 > 3\n\tprint \"no\"\n{closer}");
		assert_eq!(printed(&source), "");
	}
	is!("fi = 3; fi", 3);
}
