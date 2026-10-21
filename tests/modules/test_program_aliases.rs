//! A program's own alias lines (wiki/alias.md), as a used module's: `alias pw = power` names power pw too; an alias
//! never shadows a word the program declares itself (card module-alias)
use crate::is;

#[test]
fn a_program_alias_names_its_word() {
	is!("power(b, e) := b ^ e; alias pw = power; pw(2, 3)", 8);
	is!("alias pw = power; power(b, e) := b ^ e; pw(2, 3)", 8);
}

#[test]
fn a_program_alias_leaves_a_declared_word_alone() {
	is!("twice(x) := 2 * x; alias thrice = twice; thrice(x) := 3 * x; thrice(2)", 6);
}
