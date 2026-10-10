// card print-oldest (user, 2026-10-10: "text + Formatable should work in general"): an instance of a type with its own
// text, `text(p:P) := …` or a text() method (Printable, P31), joins a text as that text, as str() and "\(x)" do
use crate::is;

const PRINTABLE: &str = "class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")";

#[test]
fn a_printable_instance_joins_a_text_as_its_own_text() {
	is!(&format!("{PRINTABLE}\n\"hi \" + bo"), "hi person Bo");
	is!(&format!("{PRINTABLE}\n\"hi \" + bo + \"!\""), "hi person Bo!");
	is!(&format!("{PRINTABLE}\nbo + \" there\""), "person Bo there");
	is!(&format!("{PRINTABLE}\ns = \"yo \"\ns + bo"), "yo person Bo");
	is!("class Q{name: text\ntext() := \"Q \" + name}\n\"hi \" + Q(\"Cy\")", "hi Q Cy");
}
