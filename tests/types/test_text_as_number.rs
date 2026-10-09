// card units-dynamic: `t as number` of a text known only at run time is the number it spells, as `"5" as number` is:
// an Int when whole, else a Float; never the text itself
use crate::is;

const READ: &str = "var t = \"\"\nfor c in \"5\".chars() { t = t + c }\n";

#[test]
fn a_run_time_text_as_number_is_its_number() {
	is!(&format!("{READ}(t as number) * 2"), 10);
	is!(&format!("{READ}n = t as number; n + 1"), 6);
	is!(&format!("{READ}class Q{{amount}}\nq = Q(t as number)\nq.amount / 2"), 2.5);
	is!(&format!("{READ}xs = [t as number]\nxs#1 * 3"), 15);
	is!("var t = \"\"\nfor c in \"-2.5\".chars() { t = t + c }\n(t as number) * 2", -5.0);
}

#[test]
fn a_literal_text_as_number_is_its_number_in_a_variable_too() {
	is!("n = \"5\" as number; n * 2", 10);
	is!("f(a, b) := a * 2\nf(\"5\" as number, 1)", 10);
}
