//! card instance-result: a method's result that is an instance prints with its class's text(), also straight from the
//! call in an interpolation hole, as it does held in a variable
use crate::is;

const QUANTITY: &str = "class Q{v:float; u:text; text() := \"\\(v) \\(u)\"; to(w:text) := Q(v*2, w)}\nq0 = Q(1.5, \"m\")\n";

#[test]
fn an_interpolated_method_result_uses_its_text() {
	is!(&format!("{QUANTITY}\"\\(q0.to(\"km\"))\""), "3 km");
	is!(&format!("{QUANTITY}\"\\(q0.to(\"km\").to(\"mi\"))\""), "6 mi");
	is!(&format!("{QUANTITY}q0.to(\"km\") as text"), "3 km");
	is!(&format!("{QUANTITY}q = q0.to(\"km\"); \"\\(q)\""), "3 km");
}
