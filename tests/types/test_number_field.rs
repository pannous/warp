// Card number-field: `number` is an int or a float. A field amount:number holding a float reads as that float, and
// `"5" as number` used as a number is 5 (found in units-dynamic)
use crate::is;

const Q: &str = "class Q{amount:number}\n";

#[test]
fn a_number_field_holds_a_float() {
	is!(&format!("{Q}n = \"2.5\" as number\nQ(n).amount / 2"), 1.25);
	is!(&format!("{Q}n = \"2.5\" as number\nq = Q(n)\nq.amount * 2"), 5.0);
	is!(&format!("{Q}f(q:Q) := q.amount / 2\nf(Q(\"2.5\" as number))"), 1.25);
	is!(&format!("{Q}Q(3).amount * 2"), 6);
	is!(&format!("{Q}Q(2.5).amount * 2"), 5.0);
}

#[test]
fn a_whole_text_as_number_is_a_number() {
	is!("n = \"5\" as number\nn / 2", 2.5);
	is!("n = \"5\" as number\nn + 1", 6);
	is!(&format!("{Q}n = \"5\" as number\nQ(n).amount / 2"), 2.5);
}
