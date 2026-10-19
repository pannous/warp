//! Card node-enum (decision compiler-node-enum): lib/compiler/node.warp declares the built-in node's cases, so `Node` is
//! a type, `Key(a, Op.Add, b)` builds a key and `match n { Key(left, op, right): … }` takes the node itself apart; Op,
//! Bracket and Separator are enums spelled as a node carries them. The twin tests hold node.warp to src/node/mod.rs.
use crate::is;
use warp::lowering::node_cases::{spelled_enum, spelling, NODE_DECLARATION, SPELLED_ENUMS};
use warp::node::{Bracket, Node, Separator};
use warp::operators::{Op, OP_CODES};
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

/// The Rust node's cases with their number of values; a new case fails this match until node.warp has it too
fn rust_case(node: &Node) -> (&'static str, usize) {
	match node {
		Node::True => ("True", 0),
		Node::False => ("False", 0),
		Node::Empty => ("Empty", 0),
		Node::Number(_) => ("Number", 1),
		Node::Char(_) => ("Char", 1),
		Node::Text(_) => ("Text", 1),
		Node::Symbol(_) => ("Symbol", 1),
		Node::Error(_) => ("Error", 1),
		Node::Key(..) => ("Key", 3),
		Node::List(..) => ("List", 3),
		Node::Data(_) => ("Data", 1),
		Node::Meta { .. } => ("Meta", 2),
		Node::Type { .. } => ("Type", 2),
	}
}

/// node.warp's Node cases as written, with their number of values
fn declared_node_cases() -> Vec<(String, usize)> {
	let body = NODE_DECLARATION.split("enum Node {").nth(1).expect("enum Node in node.warp");
	let body = body.split("\n}").next().expect("the end of enum Node");
	body.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.map(|case| match case.split_once('(') {
			Some((name, values)) => (name.to_string(), values.matches(':').count()),
			None => (case.to_string(), 0),
		})
		.collect()
}

#[test]
fn node_warp_declares_the_rust_node() {
	let samples = [
		Node::True,
		Node::False,
		Node::Empty,
		Node::int(1),
		Node::Char('a'),
		Node::Text("t".into()),
		symbol("s"),
		Node::Error(Box::new(Node::Empty)),
		Node::Key(Box::new(Node::Empty), Op::Add, Box::new(Node::Empty)),
		Node::List(vec![], Bracket::Round, Separator::Comma),
		Node::data(1),
		Node::Meta { node: Box::new(Node::Empty), data: Box::new(Node::Empty) },
		Node::Type { name: Box::new(Node::Empty), body: Box::new(Node::Empty) },
	];
	let rust: Vec<(String, usize)> = samples.iter().map(rust_case).map(|(name, count)| (name.into(), count)).collect();
	assert_eq!(declared_node_cases(), rust);
}

#[test]
fn node_warp_spells_each_operator_bracket_and_separator() {
	for enum_name in SPELLED_ENUMS {
		let (_, cases) = spelled_enum(enum_name).expect("the enum in node.warp");
		for case in &cases {
			assert!(spelling(enum_name, case).is_some(), "{enum_name}.{case} has no spelling");
		}
	}
	let (_, ops) = spelled_enum("Op").unwrap();
	for op in OP_CODES {
		assert!(ops.contains(&format!("{op:?}")), "node.warp's Op lacks {op:?}");
	}
	assert_eq!(spelled_enum("Separator").unwrap().1.len(), warp::wasm_emitter::SEPARATORS.len());
}
