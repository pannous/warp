//! A declared list given the concatenation of two lists checks the other list's items (card concat-unchecked:
//! `xs: ints = [1]; ys = ["a"]; xs = xs + ys` gave [1 'a']): the items of `xs + ys` are of both lists' common type,
//! held as Nodes when the lists differ
use crate::common::fails_with;
use crate::is;

#[test]
fn concatenation_checks_declared_element_type() {
	fails_with("xs: ints = [1]; ys = [\"a\"]; xs = xs + ys; xs", "xs is declared ints, cannot hold an item that is no int");
	fails_with("xs: ints = [1]; ys = [\"a\"]; xs = [2] + ys; xs", "cannot hold an item that is no int");
	fails_with("xs: ints = [1]; ys = [\"a\"]; xs += ys; xs", "cannot hold an item that is no int");
	is!("xs = [1]; ys = [\"a\"]; zs = xs + ys; zs#2 is int", false);
	is!("xs: ints = [1]; ys = [2, 3]; xs = xs + ys; xs#3", 3);
}
