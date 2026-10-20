//! Card node-enum (decision compiler-node-enum): lib/compiler/node.warp declares the built-in node's cases, so `Node` is
//! a type, `Key(a, Op.Add, b)` builds a key and `match n { Key(left, op, right): … }` takes the node itself apart; Op,
//! Bracket and Separator are enums spelled as a node carries them.
use crate::is;
use warp::{parse, symbol};

/// Counts the additions in a tree: the patterns of every node case the compiler walks
const ADDS: &str = "adds(n: Node): int := match n {
	Key(left, Op.Add, right): 1 + adds(left) + adds(right)
	Key(left, op, right): adds(left) + adds(right)
	List(items, bracket, separator): sum(items.map(adds))
	_: 0
}";

#[test]
fn a_pattern_takes_the_node_apart() {
	is!("n = parse(\"a+1\"); match n { Key(l, op, r): op, _: 0 }", symbol("Add"));
	is!("n = parse(\"a+1\"); match n { Key(l, op, r) => l, _ => 0 }", symbol("a"));
	is!("n = parse(\"x\"); match n { Symbol(name): name, _: 0 }", "x");
	is!("n = parse(\"[1,2]\"); match n { List(items, bracket, s): bracket, _: 0 }", symbol("Square"));
	is!("match parse(\"f(x)\") { Key(Symbol(n), op, v): n, List(items, Bracket.Round, s): 1, _: 0 }", 1);
	is!(&format!("{ADDS}\nadds(parse(\"a + b * (c + d)\"))"), 2);
}

/// `match f(x) {…}`, a call as the subject, takes the node apart as a name does
#[test]
fn a_called_subject_is_taken_apart() {
	is!(
		"id(n: Node): Node := n; right(m: Node): Node := match id(m) { Key(a, op, b): b, _: m }; right(parse(\"x+2\"))",
		2
	);
}

#[test]
fn a_case_builds_the_node() {
	is!("Key(symbol(\"a\"), Op.Add, 1)", parse("a+1"));
	is!("Node.Key(symbol(\"a\"), Op.Mul, 2)", parse("a*2"));
	is!("List([1, 2], Bracket.Round, Separator.Comma)", parse("(1, 2)"));
	is!(
		"swap(n: Node): Node := match n { Key(l, Op.Add, r): Key(r, Op.Add, l), _: n }; swap(parse(\"1+2\"))",
		parse("2+1")
	);
}

#[test]
fn node_is_the_type_of_any_node() {
	is!("f(n: Node): Node := n; f(3)", 3);
	is!("Node.True", true);
	is!("Node.Empty == ø", true);
}

#[test]
fn an_operator_case_is_spelled_as_the_node_carries_it() {
	is!("Op.Add", symbol("Add"));
	is!("Op.Add as text", "+");
	is!("\"+\" as Op", symbol("Add"));
	is!("Bracket.Square as text", "[");
	is!("Separator.Comma as text", ",");
}

#[test]
fn a_programs_own_names_win() {
	is!("class Key { a: int }; k = Key(3); k.a", 3);
	is!("Key(a, b) := a + b; Key(1, 2)", 3);
	is!("enum Op { Plus, Minus }; Op.Minus as int", 1);
}

#[test]
fn an_enum_case_in_a_pattern_compares() {
	let program = "enum Color { Red, Green }\nclass P { c: Color; n: int }\np = P(Color.Green, 2)\n";
	is!(&format!("{program}match p {{ P(Color.Red, n): 1, P(Green, n): n, _: 0 }}"), 2);
}

#[test]
fn a_patterns_op_is_spelled_as_text() {
	is!("n = parse(\"a+1\"); match n { Key(l, op, r): op as text, _: 0 }", "+");
	is!("n = parse(\"(a)\"); match n { List(items, bracket, s): bracket as text, _: 0 }", "(");
}
