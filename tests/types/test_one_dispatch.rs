//! card one-dispatch (notes/dispatch.md "One lookup for every word"): `x.word` ≡ `x.word()`, the receiver's own
//! member before its type's library word, decided at compile time when the receiver is known, else at run time
use crate::is;

const PERSON: &str = "class P{first:int}";

#[test]
fn a_member_is_read_with_or_without_parentheses() {
	is!("p = {first: 1}; p.first", 1);
	is!("p = {first: 1}; p.first()", 1);
	is!(&format!("{PERSON}; n = P(3); n.first()"), 3);
	is!("f(m) := m.first(); f({first: 9})", 9);
}

#[test]
fn an_instance_of_unknown_class_reads_its_own_member_at_run_time() {
	is!(&format!("{PERSON}; f(p) := p.first; f(P(3))"), 3);
	is!(&format!("{PERSON}; f(p) := p.first(); f(P(3))"), 3);
	is!(&format!("{PERSON}; f(p) := p.first; f([4, 5])"), 4);
}
