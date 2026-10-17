//! card receiver-field-assign: in a method a bare field name reads and writes its receiver's field (P115), until a
//! `let` gives the method a local of that name; a plain `body = …` changes the field, in an outside method `x(f: F)`
//! as in the class body, silently in both (decisions.md receiver-field-assign)
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
fn a_bare_field_write_changes_the_field() {
	is!(&format!("{OUTSIDE}\tbody = 99\n\tbody\n}}\nf = F(\"main\", 42)\nfresh(f)\nf.body"), 99);
	is!(&format!("{INSIDE}\tbody = 99\n\tbody\n}} }}\nf = F(\"main\", 42)\nf.fresh()\nf.body"), 99);
}
