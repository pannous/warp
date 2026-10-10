//! A const list does not change through its methods either (card const-list-add): `add`, `push`, `pop`, `remove`,
//! `insert` are refused like an assignment
use crate::common::fails_with;
use crate::is;

const REFUSED: &str = "names is const, cannot change it";

#[test]
fn a_const_list_refuses_its_changing_methods() {
	fails_with("const names = [\"hi\"]; names.add(420); names", REFUSED);
	fails_with("const names = [\"hi\"]; names.push(\"yo\"); names", REFUSED);
	fails_with("const names = [\"b\" \"a\"]; names.pop(); names", REFUSED);
	fails_with("const names = [\"b\" \"a\"]; names.remove(\"a\"); names", REFUSED);
	fails_with("const names = [\"b\" \"a\"]; names.insert(0, \"c\"); names", REFUSED);
}

#[test]
fn a_const_list_reads_and_a_let_list_changes() {
	is!("const names = [\"b\" \"a\"]; #names", 2);
	fails_with("let names = [\"hi\"]; names.add(\"yo\"); #names", "names is let (immutable), cannot change it"); // user, card let-reassign
	is!("names = [\"hi\"]; names.add(\"yo\"); #names", 2);
}
