// card match-guard: a guarded arm keeps its arm when braces follow the guard: a block body `n if n > 3 => { … }` or a
// map in the condition `if k in {a: 1} => …`; the trailing if took the braces as its own branch
use crate::is;

#[test]
fn a_guarded_arm_with_a_block_body() {
	is!("x = 5\nmatch x { n if n > 3 => { \"big\" }; _ => \"small\" }", "big");
	is!("x = 1\nmatch x { n if n > 3 => { \"big\" }; _ => \"small\" }", "small");
	is!("x = 5\nmatch x {\n\tn if n > 3 => {\n\t\ty = n * 2\n\t\ty\n\t}\n\t_ => 0\n}", 10);
}

#[test]
fn a_guard_holding_a_map() {
	is!("k = \"a\"\nmatch k { w if w in {a: 1} => \"known\"; _ => \"unknown\" }", "known");
	is!("k = \"b\"\nmatch k { w if w in {a: 1} => \"known\"; _ => \"unknown\" }", "unknown");
}

#[test]
fn a_statement_if_keeps_its_block() {
	is!("x = 0\nif 1 > 0 { x = 2 }\nx", 2);
	is!("x = 0\nx = 3 if 1 > 0\nx", 3);
}
