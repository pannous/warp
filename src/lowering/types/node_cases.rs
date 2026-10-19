//! The built-in node's cases (decision compiler-node-enum): lib/compiler/node.warp declares `enum Node` case for case as
//! src/node/mod.rs has it, and its Op, Bracket and Separator. No class is made of Node: `Node` is the type of any node,
//! `Node.Key` is the case Key, `Key(a, Op.Add, b)` builds the key `a+b`, and a match arm `Key(left, op, right):` tests the
//! kind and reads the node's parts (destructuring.rs). Op, Bracket and Separator are plain enums whose cases are spelled
//! as a node carries them: `Op.Add as text` is "+", `"+" as Op` is Add.

use super::nodes::{key, matched};
use crate::node::{call, Bracket, Node};
use crate::operators::{Op, OP_CODES};
use std::collections::HashMap;

pub const NODE_DECLARATION: &str = include_str!("../../../lib/compiler/node.warp");
pub const NODE_TYPE: &str = "Node";
/// The enums of node.warp whose cases are spelled, in its order
pub const SPELLED_ENUMS: [&str; 3] = ["Op", "Bracket", "Separator"];
const ANY_TYPE: &str = crate::words::ANY_TYPE;
/// The types a spelled case converts to its spelling: `op as text`
pub const TEXT_TYPES: [&str; 2] = [crate::words::TEXT_WORD, "string"];
const OTHER_ARM: &str = "_";
const SPELLED_VALUE: &str = "spelled";
/// `Op·text(c)`: the spelling of a case; `Op·case(t)`: the case spelled so
const TEXT_FUNCTION: &str = "text";
const CASE_FUNCTION: &str = "case";
const BRACKETS: [(&str, Bracket); 5] = [
	("Curly", Bracket::Curly),
	("Square", Bracket::Square),
	("Round", Bracket::Round),
	("Less", Bracket::Less),
	("None", Bracket::None),
];

/// A case of the built-in node with values: the type its kind tests as and the reader of each field, in order
pub struct NodeCase {
	pub name: &'static str,
	pub kind_test: KindTest,
	pub fields: &'static [Part],
}

pub enum KindTest {
	/// `n is key`
	TypeWord(&'static str),
	/// `bracket_of(n) != ø`: a group, whatever its bracket (`is list` holds for ø too)
	Group,
}

/// How a field reads its node: the node itself, a part reader, or a part reader whose spelling is a case of an enum
pub enum Part {
	Itself,
	Reader(&'static str),
	Spelled(&'static str, &'static str),
}

/// The cases with values a pattern takes apart; Error, Data, Meta and Type have no reader yet
pub const NODE_CASES: [NodeCase; 6] = [
	NodeCase { name: "Number", kind_test: KindTest::TypeWord("number"), fields: &[Part::Itself] },
	NodeCase { name: "Char", kind_test: KindTest::TypeWord("codepoint"), fields: &[Part::Itself] },
	NodeCase { name: "Text", kind_test: KindTest::TypeWord("text"), fields: &[Part::Itself] },
	NodeCase { name: "Symbol", kind_test: KindTest::TypeWord("symbol"), fields: &[Part::Reader("name_of")] },
	NodeCase {
		name: "Key",
		kind_test: KindTest::TypeWord("key"),
		fields: &[Part::Reader("key_of"), Part::Spelled("op_of", "Op"), Part::Reader("value_of")],
	},
	NodeCase {
		name: "List",
		kind_test: KindTest::Group,
		fields: &[Part::Itself, Part::Spelled("bracket_of", "Bracket"), Part::Spelled("separator_of", "Separator")],
	},
];

/// The case of that name a pattern takes apart, when the program has no type of that name
pub fn node_case(name: &str) -> Option<&'static NodeCase> {
	NODE_CASES.iter().find(|case| case.name == name)
}

/// The kind test of a case at `path`
pub fn kind_test(case: &NodeCase, path: &Node) -> Node {
	match case.kind_test {
		KindTest::TypeWord(word) => key(path.clone(), Op::Eq, Node::symbol(word)),
		KindTest::Group => key(call("bracket_of", vec![path.clone()]), Op::Ne, Node::Empty),
	}
}

/// The field of a case at `path`
pub fn field_at(part: &Part, path: &Node) -> Node {
	match part {
		Part::Itself => path.clone(),
		Part::Reader(reader) => call(reader, vec![path.clone()]),
		Part::Spelled(reader, enum_name) => {
			call(&spelled_function(enum_name, CASE_FUNCTION), vec![call(reader, vec![path.clone()])])
		}
	}
}

fn spelled_function(enum_name: &str, function: &str) -> String {
	format!("{enum_name}·{function}")
}

thread_local! {
	/// node.warp's statements as the parser reads them
	static DECLARATIONS: Vec<Node> =
		crate::normalize::without_hints(|| crate::warp_parser::WarpParser::parse(NODE_DECLARATION))
			.drop_meta()
			.sequence_statements()
			.to_vec();
}

/// node.warp's plain enum of that name, `enum Op {…}`, and its cases
pub fn spelled_enum(name: &str) -> Option<(Node, Vec<String>)> {
	DECLARATIONS.with(|declarations| {
		declarations.iter().find_map(|statement| match statement.drop_meta() {
			Node::List(items, Bracket::None, _) if items.len() == 3 && items[1].drop_meta().name() == name => {
				let Node::List(cases, Bracket::Curly, _) = items[2].drop_meta() else { return None };
				let cases = super::sum_variants::case_rows(cases).iter().map(|case| case.drop_meta().name()).collect();
				Some((statement.clone(), cases))
			}
			_ => None,
		})
	})
}

/// The text a node carries for a case of Op, Bracket or Separator: `+` for Add
pub fn spelling(enum_name: &str, case: &str) -> Option<String> {
	let named = |debug: String| debug == case;
	match enum_name {
		"Op" => OP_CODES.iter().find(|op| named(format!("{op:?}"))).map(|op| op.as_str().to_string()),
		"Bracket" => BRACKETS.iter().find(|(name, _)| *name == case).map(|(_, bracket)| match bracket {
			Bracket::None => String::new(),
			other => other.opening().to_string(),
		}),
		"Separator" => crate::wasm_emitter::SEPARATORS
			.iter()
			.find(|separator| named(format!("{separator:?}")))
			.map(|separator| separator.to_char().map(String::from).unwrap_or_default()),
		_ => None,
	}
}

/// `Op·text(c) := match c {Add: "+", …, _: c}` and `Op·case(t) := match t {"+": Add, …, _: t}`: a text passes
/// either, so `key(a, "+", b)` keeps working while the compiler moves to the cases
fn spelling_functions(enum_name: &str, cases: &[String]) -> Vec<Node> {
	let value = Node::symbol(SPELLED_VALUE);
	let spelled: Vec<(Node, Node)> =
		cases.iter().filter_map(|case| Some((Node::symbol(case), Node::Text(spelling(enum_name, case)?)))).collect();
	let function = |name: &str, arms: Vec<Node>| {
		let other = key(Node::symbol(OTHER_ARM), Op::Colon, value.clone());
		let body = matched(value.clone(), [arms, vec![other]].concat());
		key(call(&spelled_function(enum_name, name), vec![value.clone()]), Op::Define, body)
	};
	let to_text = spelled.iter().map(|(case, text)| key(case.clone(), Op::Colon, text.clone())).collect();
	let to_case = spelled.iter().map(|(case, text)| key(text.clone(), Op::Colon, case.clone())).collect();
	vec![function(TEXT_FUNCTION, to_text), function(CASE_FUNCTION, to_case)]
}

/// The type names the program declares itself: `enum Op {…}`, `class Node {…}`
fn own_types(program: &Node) -> Vec<String> {
	let mut names = vec![];
	program.visit(&mut |node| match node {
		Node::Type { name, .. } => names.push(name.drop_meta().name()),
		Node::List(items, _, _) if items.len() == 3 && items[0].is_symbol("enum") => {
			names.push(items[1].drop_meta().name())
		}
		_ => {}
	});
	names
}

fn mentions(program: &Node, word: &str) -> bool {
	let mut found = false;
	program.visit(&mut |node| found |= node.is_symbol(word));
	found
}

/// The sugar pass: `Node.Key` → Key, `n: Node` → `n: any`, and node.warp's spelled enums and their spelling functions
/// in front of a program naming them or building and taking apart keys and lists
pub fn lower(program: Node) -> Node {
	let own = own_types(&program);
	let builds_nodes = ["Key", "List"].iter().any(|case| mentions(&program, case));
	let program = match own.iter().any(|name| name == NODE_TYPE) {
		true => program,
		false => qualified_cases(program),
	};
	let enums: Vec<&str> = SPELLED_ENUMS
		.into_iter()
		.filter(|name| !own.iter().any(|own| own == name) && (builds_nodes || mentions(&program, name)))
		.collect();
	let declarations = enums
		.iter()
		.filter_map(|name| Some((name, spelled_enum(name)?)))
		.flat_map(|(name, (declaration, cases))| [vec![declaration], spelling_functions(name, &cases)].concat());
	super::nodes::with_statements_first(program, declarations.collect())
}

/// `Node.Key`, `Node::Key` → Key, `Node.True` → yes, `Node.Empty` → ø, and `Node` where a type is written → any
fn qualified_cases(node: Node) -> Node {
	match node.map_children(qualified_cases) {
		Node::Key(owner, Op::Dot | Op::Scope, case) if owner.is_symbol(NODE_TYPE) => {
			match case.drop_meta().name().as_str() {
				"True" => Node::True,
				"False" => Node::False,
				"Empty" => Node::Empty,
				_ => case.drop_meta().clone(),
			}
		}
		Node::Key(subject, Op::Colon, kind) if kind.is_symbol(NODE_TYPE) => {
			Node::Key(subject, Op::Colon, Box::new(Node::symbol(ANY_TYPE)))
		}
		other => other,
	}
}

/// After the patterns took theirs apart (destructuring.rs): `Key(a, op, b)` builds the key, `List(items, bracket,
/// separator)` the list, with the program's own function or class of that name winning; `op as text` of a pattern's
/// op field is its spelling
pub fn lower_constructions(program: Node) -> Node {
	let own: Vec<String> = crate::modules::statements_of(&program)
		.iter()
		.filter_map(crate::modules::name_declared_by)
		.chain(own_types(&program))
		.collect();
	let builds = |case: &str| !own.iter().any(|name| name == case);
	let spelled = spelled_bindings(&program);
	constructions(program, &builds, &spelled)
}

/// The names a pattern bound to a spelled case, `op` of `Key(left, op, right)`, by the enum of each
fn spelled_bindings(program: &Node) -> HashMap<String, String> {
	let mut spelled = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Assign, value) = node {
			if let (Some(name), Some(enum_name)) = (name.symbol_name(), spelled_case_call(value)) {
				spelled.insert(name.to_string(), enum_name);
			}
		}
	});
	spelled
}

/// `Op·case(…)`: Op
fn spelled_case_call(value: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = value.drop_meta() else { return None };
	let function = items.first()?.symbol_name()?;
	let enum_name = function.strip_suffix(&format!("·{CASE_FUNCTION}"))?;
	SPELLED_ENUMS.contains(&enum_name).then(|| enum_name.to_string())
}

fn constructions(node: Node, builds: &dyn Fn(&str) -> bool, spelled: &HashMap<String, String>) -> Node {
	let as_text = |enum_name: &str, value: Node| call(&spelled_function(enum_name, TEXT_FUNCTION), vec![value]);
	let spelled_enum_of = |value: &Node| value.symbol_name().and_then(|name| spelled.get(name));
	match node.map_children(|child| constructions(child, builds, spelled)) {
		Node::List(items, Bracket::Round, separator) => match items.as_slice() {
			[head, left, op, right] if head.is_symbol("Key") && builds("Key") => {
				call("key", vec![left.clone(), as_text("Op", op.clone()), right.clone()])
			}
			[head, items, bracket, separator] if head.is_symbol("List") && builds("List") => {
				let bracketed = call("bracketed", vec![items.clone(), as_text("Bracket", bracket.clone())]);
				call("separated", vec![bracketed, as_text("Separator", separator.clone())])
			}
			_ => Node::List(items, Bracket::Round, separator),
		},
		Node::Key(value, Op::As, target)
			if spelled_enum_of(&value).is_some() && TEXT_TYPES.contains(&target.drop_meta().name().as_str()) =>
		{
			let enum_name = spelled_enum_of(&value).expect("guarded").clone();
			as_text(&enum_name, *value)
		}
		other => other,
	}
}
