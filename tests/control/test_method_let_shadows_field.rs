//! card receiver-field-assign: in a method a bare field name reads and writes its receiver's field (P115), until a
//! `let` gives the method a local of that name; a plain `body = …` in an outside method `def x(f: F)`, whose fields
//! the text never shows, changes the field with a warning naming both readings
use crate::common::warns_with;
use crate::is;

const OUTSIDE: &str = "class F { name: text; body: any }\nfresh(f: F) := {\n";
const INSIDE: &str = "class F { name: text; body: any\n fresh() := {\n";

#[test]
fn a_let_local_hides_the_field() {
	is!(&format!("{OUTSIDE}\tlet body = 99\n\tbody\n}}\nf = F(\"main\", 42)\nfresh(f) + 1000 * f.body"), 42099);
	is!(&format!("{INSIDE}\tlet body = 99\n\tbody\n}} }}\nf = F(\"main\", 42)\nf.fresh() + 1000 * f.body"), 42099);
}

#[test]
fn a_let_local_starts_after_its_value() {
	is!(&format!("{OUTSIDE}\tlet body = body + 1\n\tbody\n}}\nfresh(F(\"main\", 42))"), 43);
}

#[test]
fn an_outside_method_writing_a_bare_field_warns() {
	let code = format!("{OUTSIDE}\tbody = 99\n\tbody\n}}\nf = F(\"main\", 42)\nfresh(f)\nf.body");
	warns_with(&code, 99, "make a new local");
}
