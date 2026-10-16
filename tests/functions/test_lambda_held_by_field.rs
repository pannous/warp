//! card lambda-held: a function value held by a field, of an untyped class field or a map entry, is called in every
//! call form: `(h.d)(5)`, `h.d(5)`, `5 |> h.d`, and through a variable `g = h.d; g(5)`
use crate::is;

const HOLDER: &str = "class Holder {d}\nh = Holder(x => x + 1)\n";

#[test]
fn a_lambda_in_an_untyped_class_field_is_called() {
	for call in ["(h.d)(5)", "h.d(5)", "5 |> h.d", "g = h.d\ng(5)"] {
		is!(&format!("{HOLDER}{call}"), 6);
	}
}

#[test]
fn a_lambda_in_a_map_entry_is_called() {
	is!("h = {d: x => x * 3}\n(h.d)(5)", 15);
	is!("h = {d: x => x * 3}\nh.d(5)", 15);
}
