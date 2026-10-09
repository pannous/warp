//! cards field-named-size, orm-members-word: a declared field named like a builtin word (size, count, length, members)
//! wins over the word when the value's class is known, also through a declared list `xs: [C]`; assigning the field or
//! adding to it changes the field, not the reflection list
use crate::is;

#[test]
fn a_size_field_of_an_element_of_a_declared_list() {
	is!("class C{size: float}\nxs: [C] = [C(3.5)]\nxs#1.size", 3.5);
	is!("class C{size: float}\nxs: [C] = [C(3.5)]\nc = xs#1\nc.size", 3.5);
	is!("class C{count: int}\nxs: [C] = [C(7)]\nxs#1.count", 7);
	is!("class C{length: int}\nxs: [C] = [C(7)]\nfor c in xs { c.length }", 7);
}

#[test]
fn a_members_field_is_a_place() {
	is!("class G{name: text; members: [text]}\ngs: [G] = [G(\"a\", [\"x\"])]\ngs#1.members = [\"y\", \"z\"]\ncount(gs#1.members)", 2);
	is!("class G{name: text; members: [text]}\nf(o) := { o.members.add(\"z\"); count(o.members) }\nf(G(\"a\", [\"x\"]))", 2);
}
