//! Rust's paths `A::b` read as warp (card rust-paths, decision rust-forms-welcome): `std::cmp::max(a, b)` is `max(a, b)`,
//! `f64::sqrt(x)` is `sqrt(x)`, `i32::MAX` its number, `String::from("hi")` the text, `Vec::new()` `[]`, and
//! `xs.iter()` / `.collect()` the list itself. An enum's `C::Red` and a path of the program's own names stay.

use crate::modules::USE_KEYWORD;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

/// The roots of Rust's standard paths, dropped with the module names after them: `std::collections::HashMap`
const STD_ROOTS: [&str; 3] = ["std", "core", "alloc"];
/// `f64::consts::PI`: the module of a number type's constants
const CONSTS_MODULE: &str = "consts";
const INTEGER_LIMITS: [(&str, i64, i64); 10] = [
	("i8", i8::MIN as i64, i8::MAX as i64),
	("i16", i16::MIN as i64, i16::MAX as i64),
	("i32", i32::MIN as i64, i32::MAX as i64),
	("i64", i64::MIN, i64::MAX),
	("isize", i64::MIN, i64::MAX),
	("u8", 0, u8::MAX as i64),
	("u16", 0, u16::MAX as i64),
	("u32", 0, u32::MAX as i64),
	("u64", 0, i64::MAX),
	("usize", 0, i64::MAX),
];
const FLOAT_TYPES: [&str; 2] = ["f32", "f64"];
const FLOAT_CONSTANTS: [(&str, f64); 6] = [
	("MAX", f64::MAX),
	("MIN", f64::MIN),
	("EPSILON", f64::EPSILON),
	("INFINITY", f64::INFINITY),
	("NEG_INFINITY", f64::NEG_INFINITY),
	("NAN", f64::NAN),
];
/// The constructors of the standard collections and texts, `Owner::constructor(…)`, and the warp literal each is;
/// None: the argument itself (`String::from("hi")` is "hi")
const CONSTRUCTORS: [(&str, &str, Option<&str>); 12] = [
	("String", "new", Some("\"\"")),
	("String", "from", None),
	("Vec", "new", Some("[]")),
	("Vec", "with_capacity", Some("[]")),
	("Vec", "from", None),
	("VecDeque", "new", Some("[]")),
	("HashMap", "new", Some("{}")),
	("BTreeMap", "new", Some("{}")),
	("HashMap", "with_capacity", Some("{}")),
	("HashSet", "new", Some("[]")),
	("BTreeSet", "new", Some("[]")),
	("Box", "new", None),
];
/// `xs.iter()`, `xs.into_iter().map(f).collect()`: a list iterates itself, these methods are the list
const ITERATOR_METHODS: [&str; 4] = ["iter", "into_iter", "iter_mut", "collect"];

pub fn lower(program: Node) -> Node {
	if !mentions_rust_path(&program) {
		return program;
	}
	let own = crate::program_names::Bindings::of(&program).given();
	rewritten(program, &own)
}

fn mentions_rust_path(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |node| {
		found |= matches!(node, Node::Key(_, Op::Scope, _))
			|| matches!(node, Node::Key(_, Op::Dot, method) if iterator_method(method))
	});
	found
}

fn rewritten(node: Node, own: &HashSet<String>) -> Node {
	// `use std::collections::HashMap` names what it brings, the module loader reads the path
	if matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.is_symbol(USE_KEYWORD)))
	{
		return node;
	}
	// a path as a whole: `std::cmp` alone is no value, `std::cmp::max(a, b)` is
	if let Node::Key(_, Op::Scope, _) = node.drop_meta() {
		let segments: Vec<Node> = segments(&node).into_iter().map(|segment| rewritten(segment, own)).collect();
		return path_value(segments.clone(), own).unwrap_or_else(|| joined(segments));
	}
	let node = node.map_children(|child| rewritten(child, own));
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot, method) if iterator_method(method) && !own.contains(&called_name(method)) => {
			receiver.as_ref().clone()
		}
		_ => node,
	}
}

fn iterator_method(method: &Node) -> bool {
	let called_without_arguments = match method.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.len() == 1,
		Node::Symbol(_) => true,
		_ => false,
	};
	called_without_arguments && ITERATOR_METHODS.contains(&called_name(method).as_str())
}

/// The name a segment calls or is: `new` of `new()`, `PI` of `PI`
fn called_name(segment: &Node) -> String {
	match segment.drop_meta() {
		Node::List(items, Bracket::Round, _) if !items.is_empty() => items[0].name(),
		other => other.name(),
	}
}

/// `a::b::c` as its segments a, b, c
fn segments(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::Key(left, Op::Scope, right) => [segments(left), vec![right.as_ref().clone()]].concat(),
		_ => vec![node.clone()],
	}
}

/// The segments as the path `a::b::c` again
fn joined(segments: Vec<Node>) -> Node {
	let mut segments = segments.into_iter();
	let first = segments.next().expect("a path has segments");
	segments.fold(first, |path, segment| Node::Key(Box::new(path), Op::Scope, Box::new(segment)))
}

/// The warp value of a standard path, None for any other (an enum's case, a path of the program's own names)
fn path_value(mut segments: Vec<Node>, own: &HashSet<String>) -> Option<Node> {
	if own.contains(&segments[0].name()) {
		return None;
	}
	if STD_ROOTS.contains(&segments[0].name().as_str()) {
		segments.remove(0);
		while segments.len() > 1 && is_module_name(&segments[0]) {
			segments.remove(0);
		}
	}
	match segments.as_slice() {
		[single] => Some(single.clone()),
		[owner, member] => number_type_member(owner, member).or_else(|| constructed(owner, member)),
		[owner, consts, member] if consts.is_symbol(CONSTS_MODULE) => number_type_member(owner, member),
		_ => None,
	}
}

/// A lowercase module of the standard library: `cmp`, `collections`, `mem`; a number type is no module
fn is_module_name(segment: &Node) -> bool {
	let name = segment.name();
	matches!(segment.drop_meta(), Node::Symbol(_))
		&& name.starts_with(|first: char| first.is_lowercase())
		&& !is_number_type(&name)
}

fn is_number_type(name: &str) -> bool {
	FLOAT_TYPES.contains(&name) || INTEGER_LIMITS.iter().any(|(type_name, _, _)| *type_name == name)
}

/// `i32::MAX`, `f64::EPSILON`: the number; `f64::sqrt(x)`, `f64::consts::PI`: the word or constant itself
fn number_type_member(owner: &Node, member: &Node) -> Option<Node> {
	let type_name = owner.name();
	if !is_number_type(&type_name) {
		return None;
	}
	let name = member.name();
	if let Some((_, min, max)) = INTEGER_LIMITS.iter().find(|(written, _, _)| *written == type_name) {
		match name.as_str() {
			"MAX" => return Some(Node::int(*max)),
			"MIN" => return Some(Node::int(*min)),
			_ => {}
		}
	}
	if FLOAT_TYPES.contains(&type_name.as_str()) {
		if let Some((_, value)) = FLOAT_CONSTANTS.iter().find(|(written, _)| *written == name) {
			return Some(Node::float(*value));
		}
	}
	Some(member.clone())
}

/// `String::from("hi")`, `Vec::new()`: the value warp writes for the standard constructor
fn constructed(owner: &Node, constructor: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Round, _) = constructor.drop_meta() else {
		return None;
	};
	let owner = owner.name();
	let (_, _, literal) =
		CONSTRUCTORS.iter().find(|(written_owner, name, _)| *written_owner == owner && *name == items[0].name())?;
	match literal {
		Some(literal) => Some(crate::warp_parser::parse(literal)),
		None => items.get(1).cloned(),
	}
}
