// type string = str; NO! ugly for a reason!
use crate::extensions::lists::{map, VecExtensions2};
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::{DataValue, DataType, LineInfo};
#[cfg(feature = "native")]
use crate::wasm_reader::GcObject;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt;
use std::ops::{Add, Div, Index, IndexMut, Mul, Not, Sub};
use crate::operators::{is_function_keyword, Op};
use crate::node::Node::*;
use crate::type_kinds::Kind;

mod serialization;
pub use serialization::{NO, YES};
mod json_xml;
pub use json_xml::json_of_json5;
mod indexing;
mod comparison;
#[cfg(feature = "native")]
pub use comparison::GcComparable;
mod conversions;
mod arithmetic;

/// Prefix of a `@name(value)` annotation key
pub const ATTRIBUTE_MARK: char = '@';

/// The name and value of a meta entry `@name:value`
pub fn meta_entry(entry: &Node) -> Option<(&str, &Node)> {
	match entry.drop_meta() {
		Key(key, Op::Colon, value) => match key.drop_meta() {
			Symbol(key) => key.strip_prefix(ATTRIBUTE_MARK).map(|name| (name, value.as_ref())),
			_ => None,
		},
		_ => None,
	}
}

/// Meta key holding the source text of a literal (see `with_source_literal`)
const SOURCE_LITERAL: &str = "literal";
/// `global x` parses as the pair `global: x`: a block may declare several, no duplicate key
pub const GLOBAL_DECLARATION: &str = "global";



#[derive(Clone, Serialize, Deserialize)]
pub enum Node {
	// closed cannot be extended so anticipate all cases here
	True,
	False, // alternative would be pub const FALSE: Node = Node::Number(Number::Int(0));
	Empty, // Null, Nill, None, Ø, ø null nill none nil
	// Id(i64), // unique INTERNAL(?) node id for graph structures (put in metadata?)
	// Kind(i64), enum NodeKind in serialization
	Number(Number),
	// Number(Float|Int),
	Char(char), // Single Unicode codepoint/character like 'a', '🍏' necessary?? as Number?
	Text(String),
	Symbol(String),
	Error(Box<Node>),
	// Keyword(String), Call, Declaration … AST or here? AST!  via Meta(node, AstKind)

	// emit with dot .name for special semantics .meta:{} .data={type=T, b64=[] id} .type=T …
	// key can be any node (Symbol, Number, Text, etc.)
	Key(Box<Node>, Op, Box<Node>),
	List(Vec<Node>, Bracket, Separator),
	// Map via map:{[k,v],…} or "map"={k:v, …} or just [k:v, …] for us
	Data(DataValue), // most generic container for any kind of data not captured by other node types
	Meta { node: Box<Node>, data: Box<Node> },
	Type { name: Box<Node>, body: Box<Node> }, // type definition: name + fields
}

impl Node {
	pub fn meta(node: Node, meta: Node) -> Node {
		Meta {
			node: Box::new(node),
			data: Box::new(meta),
		}
	}
	pub fn is_nil(&self) -> bool {
		*self == Empty
	}
	/// ø or a block without statements: `{}` evaluates to nothing
	pub fn is_nothing(&self) -> bool {
		match self.drop_meta() {
			Empty => true,
			List(items, Bracket::Curly, _) => items.is_empty(),
			_ => false,
		}
	}
	pub fn data_value(&self) -> DataValue {
		// 💡use via
		// let val = data.data_value().downcast_ref::<MyType>().unwrap().clone();
		match self {
			Data(dada) => dada.clone(),
			Meta { node, .. } => node.data_value(),
			_ => DataValue {
				data: Box::new(()),
				type_name: "ø".to_string(),
				data_type: DataType::None,
			},
		}
	}
	/// Remove the children `from..=to` in place: `[a b c d].remove(1,2) == [a d]`.
	/// A negative `to` means up to the end, `to` is clamped to the last child.
	pub fn remove(&mut self, from: i32, to: i32) {
		match self {
			List(items, _, _) => {
				let last = items.len() as i32 - 1;
				let to = if to < 0 { last } else { to.max(from).min(last) };
				if from >= 0 && from <= to {
					items.drain(from as usize..=to as usize);
				}
			}
			Meta { node, .. } => node.remove(from, to),
			_ => panic!("can't remove without children: {self:?}"),
		}
	}
	pub fn strings(p0: Vec<&str>) -> Node {
		List(
			map(p0, |s| Text(s.to_string())),
			Bracket::Square,
			Separator::None,
		)
	}
	pub fn first(&self) -> Node {
		match self {
			List(xs, _, _) => {
				if let Some(first) = xs.first() {
					first.clone()
				} else {
					Empty
				}
			}
			Key(k, _, _v) => k.drop_meta().clone(), // first part of key-value pair is the key
			Meta { node, .. } => node.first(),
			_ => Empty,
		}
	}
	pub fn last_item(&self) -> Node {
		// last() belongs to iterator trade!!
		match self {
			// Text(t) => {Char(t.chars().last().unwrap_or('\0'))} // switch of semantics!?
			List(xs, _, _) => {
				if let Some(last) = xs.last() {
					last.clone()
				} else {
					Empty
				}
			}
			Key(_k, _, v) => v.as_ref().clone(), // last part of key-value pair is the value
			Meta { node, .. } => node.last_item(),
			_ => Empty,
		}
	}
	pub fn print(&self) {
		println!("{:?}", self);
	}
	/// The items of a list, none of ø, else the node as the one item
	pub fn as_items(&self) -> Vec<Node> {
		match self.drop_meta() {
			List(items, _, _) => items.clone(),
			Empty => vec![],
			single => vec![single.clone()],
		}
	}
	/// The direct parts, borrowed: a list's items, a key's two sides; none of anything else (a Meta too)
	pub fn parts(&self) -> Vec<&Node> {
		match self {
			List(items, _, _) => items.iter().collect(),
			Key(left, _, right) => vec![left, right],
			_ => vec![],
		}
	}
	pub fn children(&self) -> Vec<Node> {
		match self {
			List(xs, _, _) => xs.clone(),
			Meta { node, .. } => node.children(),
			_ => vec![],
		}
	}

	pub fn add(&self, other: Node) -> Node {
		// ⚠️different semantics for different types! todo OR JUST (cons a b) for all!?
		use Node::*;
		match (self, other) {
			(Number(n), Number(m)) => Number(n.add(m)),
			(Text(s), Text(m)) => Text(format!("{}{}", s, m)),
			(List(xs, br, sep), List(ys, _, _)) => List(
				xs.iter().cloned().chain(ys).collect(),
				br.clone(),
				sep.clone(),
			),
			(List(xs, br, sep), b) => List(
				xs.iter().cloned().chain([b]).collect(),
				br.clone(),
				sep.clone(),
			),
			(a, List(ys, br, sep)) => List(
				[a.clone()].into_iter().chain(ys).collect(),
				br.clone(),
				sep.clone(),
			),
			(Meta { node, .. }, n) => node.add(n),
			(n, Meta { node, .. }) => n.add(*node.clone()),
			// a data operation evaluates nothing: the unevaluated sum a + b (user, P86: lazily allowed; evaluated, both
			// operands must be addable, which the compiled program checks)
			(a, b) => Key(Box::new(a.clone()), Op::Add, Box::new(b)),
		}
	}

	pub fn values(&self) -> &Node {
		match self {
			Key(_, _, v) => v.as_ref(),
			Meta { node, .. } => node.values(),
			List(_, _, _) => self,
			_ => &Empty,
		}
	}

	/// Returns the Kind for this node (Meta unwraps to inner node's tag)
	pub fn kind(&self) -> Kind {
		match self {
			Empty => Kind::Empty,
			Text(_) => Kind::Text,
			Char(_) => Kind::Codepoint,
			Symbol(_) => Kind::Symbol,
			Key(_, _, _) => Kind::Key,
			List(_, Bracket::Curly, _) => Kind::Block,
			List(_, _, _) => Kind::List,
			Data(_) => Kind::Data,
			Meta { node, .. } => node.kind(),
			Type { .. } => Kind::TypeDef,
			Error(_) => Kind::Error,
			False | True => Kind::Int,
			Node::Number(num) => match num {
				Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_) => Kind::Int, // exact numbers
				_ => Kind::Float, // Float, Complex, Nan, Inf
			},
		}
	}
	pub fn length(&self) -> i32 {
		match self {
			List(items, _, _) => items.len() as i32,
			Key(_, _, v) => v.length(),
			Meta { node, .. } => node.length(),
			_ => 0,
		}
	}


	pub fn value(&self) -> &Node {
		match self {
			Node::Number(_) | Text(_) | Char(_) | Data(_) => {
				self //.clone()
			}
			List(items, _, _) => {
				if items.len() == 1 {
					items.first().unwrap()
				} else {
					&Empty // or self
				}
			}
			Meta { node, data } => {
				if node.is_nil() {
					data.value()
				} else {
					node.value()
				}
			}
			Key(_, _, v) => v.value(),
			_ => &Empty,
		}
	}

	/// The one item itself, or a list of all of them
	pub fn single_or_list(mut items: Vec<Node>, bracket: Bracket, separator: Separator) -> Node {
		if items.len() == 1 { items.remove(0) } else { List(items, bracket, separator) }
	}

	/// The name of a symbol, through its metadata
	pub fn symbol_name(&self) -> Option<&str> {
		match self.drop_meta() {
			Symbol(name) => Some(name),
			_ => None,
		}
	}

	/// Whether this is the symbol `word`, through its metadata
	pub fn is_symbol(&self, word: &str) -> bool {
		self.symbol_name() == Some(word)
	}

	pub fn name(&self) -> String {
		match self {
			Symbol(name) | Text(name) => name.clone(),
			Key(k, _, _) => match k.drop_meta() {
				Symbol(s) | Text(s) => s.clone(),
				Number(n) => n.to_string(),
				_ => String::new(),
			},
			Meta { node, .. } => node.name(),
			List(items, _, _) => {
				if let Some(first) = items.first() {
					let first_name = first.name();
					// For function declarations, the name is in the second element
					if is_function_keyword(&first_name) {
						if let Some(second) = items.get(1) {
							return second.name();
						}
					}
					first_name
				} else {
					String::new()
				}
			}
			_ => String::new(),
		}
	}

	/// Convert compact 3-field WASM GC object to Node
	#[cfg(feature = "native")]
	pub fn from_gc_object(obj: &GcObject) -> Node {
		obj.to_node()
	}

	/// Placeholder for a missing feature: an error value, never a fake result
	pub fn todo(missing_feature: String) -> Node {
		error(&format!("not implemented yet: {missing_feature}"))
	}

	/// Convert Node to bool following truthiness rules:
	/// - Empty, False, 0, "", [] -> false
	/// - Everything else -> true
	pub fn to_bool(&self) -> bool {
		match self {
			False => false,
			True => true,
			Empty => false,
			Node::Number(ref n) if n.zero() => false,
			Node::Number(_) => true,
			Text(ref s) if s.is_empty() => false,
			Text(_) => true,
			Symbol(ref s) if s.is_empty() => false,
			Symbol(_) => true,
			Char(c) if c == &'\0' => false,
			Char(_) => true,
			List(ref items, _, _) if items.is_empty() => false,
			List(_, _, _) => true,
			Meta { node, .. } => node.to_bool(),
			_ => true, // Other types (Data, Key, Pair, Tag) are truthy
		}
	}
}

impl Node {

	// associated 'static' functions
	pub fn key(s: &str, v: Node) -> Self {
		Key(Box::new(Symbol(s.to_string())), Op::Colon, Box::new(v))
	}
	pub fn keys(s: &str, v: &str) -> Self {
		Key(
			Box::new(Symbol(s.to_string())),
			Op::Colon,
			Box::new(Text(v.to_string())),
		)
	}
	pub fn text(s: &str) -> Self {
		Text(s.to_string())
	}
	pub fn codepoint(c: char) -> Self {
		Char(c)
	}
	pub fn symbol(s: &str) -> Self {
		Symbol(s.to_string())
	}
	pub fn data<T: 'static + Clone + PartialEq>(value: T) -> Self {
		Data(DataValue::new(value))
	}
	pub fn number(n: Number) -> Self {
		Node::Number(n)
	}
	pub fn int(n: i64) -> Self {
		Node::Number(Number::Int(n))
	}
	pub fn float(n: f64) -> Self {
		Node::Number(Number::Float(n))
	}
	pub fn list(xs: Vec<Node>) -> Self {
		List(xs, Bracket::Square, Separator::None)
	}
	pub fn ints(xs: Vec<i32>) -> Self {
		List(
			map(xs, |x| Node::Number(Number::Int(x as i64))),
			Bracket::Square,
			Separator::None,
		)
	}

	pub fn with_meta_data<T: 'static + Clone + PartialEq>(self, data: T) -> Self {
		// Store arbitrary MetaData as a Data node
		let data_node = Node::data(data);
		Meta {
			node: Box::new(self),
			data: Box::new(data_node),
		}
	}

	/// Keep the source text of a literal whose value would print differently (`01234`, `1.10`)
	pub fn with_source_literal(self, literal: &str) -> Self {
		Meta {
			node: Box::new(self),
			data: Box::new(Node::key(SOURCE_LITERAL, Node::text(literal))),
		}
	}

	pub fn source_literal(&self) -> Option<&str> {
		match self {
			Meta { node, data } => match &data[SOURCE_LITERAL] {
				Text(literal) => Some(literal),
				_ => node.source_literal(),
			},
			_ => None,
		}
	}

	pub fn with_comment(self, comment: String) -> Self {
		let comment = Node::key("comment", Node::text(&comment));
		Meta {
			node: Box::new(self),
			data: Box::new(comment),
		}
	}

	/// Annotate with `@name(value)`; the attribute key keeps its `@` so it never mixes with comments or line info
	pub fn with_attribute(self, name: &str, value: Node) -> Self {
		Meta {
			node: Box::new(self),
			data: Box::new(Node::key(&format!("{ATTRIBUTE_MARK}{name}"), value)),
		}
	}

	/// The annotation this very Meta layer carries, if it is one
	fn own_attribute(&self) -> Option<(&str, &Node)> {
		let Meta { data, .. } = self else { return None };
		let Key(key, _, value) = data.as_ref() else { return None };
		let Symbol(key_name) = key.as_ref() else { return None };
		key_name.strip_prefix(ATTRIBUTE_MARK).map(|name| (name, value.as_ref()))
	}

	/// All `@name(value)` annotations of this node, outermost first
	pub fn attributes(&self) -> Vec<(&str, &Node)> {
		match self {
			Meta { node, .. } => self.own_attribute().into_iter().chain(node.attributes()).collect(),
			_ => Vec::new(),
		}
	}

	/// The value of the attribute `@name`: an annotation `@name(value) x`, else the meta entry `@name:value` of the literal
	pub fn attribute(&self, name: &str) -> Option<&Node> {
		let annotation = self.attributes().into_iter().find(|(attribute_name, _)| *attribute_name == name).map(|(_, value)| value);
		annotation.or_else(|| self.meta_entries().find(|(entry_name, _)| *entry_name == name).map(|(_, value)| value))
	}

	/// The meta entries `@name:value` written inside this literal (`point{x:1 @source:"gps"}`), never fields
	pub fn meta_entries(&self) -> impl Iterator<Item = (&str, &Node)> {
		let items: &[Node] = match self.drop_meta() {
			List(items, _, _) => items,
			Key(_, Op::Colon | Op::None, value) => match value.drop_meta() {
				List(items, _, _) => items,
				single => std::slice::from_ref(single),
			},
			_ => &[],
		};
		items.iter().filter_map(meta_entry)
	}

	/// The value slot of attribute `@name`; a missing attribute is created around the innermost node,
	/// so attributes keep their insertion order from the outside in.
	fn attribute_slot(&mut self, name: &str) -> &mut Node {
		if self.attribute(name).is_none() {
			self.innermost_mut().wrap_in_attribute(name);
		}
		self.existing_attribute_slot(name).expect("attribute was just created")
	}

	fn innermost_mut(&mut self) -> &mut Node {
		match self {
			Meta { node, .. } => node.innermost_mut(),
			other => other,
		}
	}

	fn wrap_in_attribute(&mut self, name: &str) {
		let core = std::mem::replace(self, Empty);
		*self = core.with_attribute(name, Empty);
	}

	fn existing_attribute_slot(&mut self, name: &str) -> Option<&mut Node> {
		let Meta { node, data } = self else { return None };
		let is_named = matches!(data.as_ref(), Key(key, _, _) if matches!(key.as_ref(), Symbol(key_name) if key_name.strip_prefix(ATTRIBUTE_MARK) == Some(name)));
		match (is_named, data.as_mut()) {
			(true, Key(_, _, value)) => Some(value.as_mut()),
			_ => node.existing_attribute_slot(name),
		}
	}

	/// The node with `rewrite` applied to its direct children, in order: the items of a list, the left then the right of
	/// a key, the node under metadata; any other node as it is. A lowering pass's recursion over the rest of the tree
	pub fn map_children(self, mut rewrite: impl FnMut(Node) -> Node) -> Node {
		match self {
			List(items, bracket, separator) => List(items.into_iter().map(&mut rewrite).collect(), bracket, separator),
			Key(left, op, right) => {
				let left = rewrite(*left);
				Key(Box::new(left), op, Box::new(rewrite(*right)))
			}
			Meta { node, data } => Meta { node: Box::new(rewrite(*node)), data },
			// a class body: its methods are lowered as functions are, its field declarations stay (card method-bodies)
			Type { name, body } => {
				let mut rewrite_code = |item: Node| if is_class_code(&item) { rewrite(item) } else { item };
				let body = match *body {
					List(items, bracket, separator) => List(items.into_iter().map(&mut rewrite_code).collect(), bracket, separator),
					single => rewrite_code(single),
				};
				Type { name, body: Box::new(body) }
			}
			other => other,
		}
	}

	/// This node under the Meta layers (comments, positions) of `original`, which it replaces
	pub fn with_meta_of(self, original: &Node) -> Node {
		match original {
			Meta { node, data } => Meta { node: Box::new(self.with_meta_of(node)), data: data.clone() },
			_ => self,
		}
	}

	pub fn drop_meta(&self) -> &Node {
		match self {
			Meta { node, .. } => node.drop_meta(),
			_ => self,
		}
	}

	/// Whether one of `words` occurs as a symbol anywhere in the tree: a pass about these words has nothing to do
	/// without one, and can skip working out the program's functions
	pub fn mentions_any(&self, words: &[&str]) -> bool {
		let mut found = false;
		self.visit(&mut |node| found |= matches!(node, Symbol(name) if words.contains(&name.as_str())));
		found
	}

	/// Call `action` on this node and every descendant (pre-order, Meta dropped)
	pub fn visit<'a>(&'a self, action: &mut dyn FnMut(&'a Node)) {
		let node = self.drop_meta();
		action(node);
		match node {
			Key(left, _, right) => {
				left.visit(action);
				right.visit(action);
			}
			List(items, _, _) => items.iter().for_each(|item| item.visit(action)),
			Type { body, .. } => match body.drop_meta() {
				List(items, _, _) => items.iter().filter(|item| is_class_code(item)).for_each(|item| item.visit(action)),
				single => if is_class_code(single) { single.visit(action) },
			},
			_ => {}
		}
	}

	// get_meta data directly or Empty
	pub fn get_meta(&self) -> &Node {
		match self {
			Meta { data, .. } => data.as_ref(),
			_ => &Empty,
		}
	}

	pub fn get_lineinfo(&self) -> Option<LineInfo> {
		match self {
			Meta { data, .. } => {
				if let Data(dada) = data.as_ref() {
					dada.downcast_ref::<LineInfo>().cloned()
				} else {
					None
				}
			}
			_ => None,
		}
	}

	// member functions taking self
	/// The number of elements (P40: size counts elements, byte_size gives bytes): `len`
	pub fn size(&self) -> usize {
		self.len()
	}

	pub fn get(&self, i: usize) -> &Node {
		match self {
			List(elements, _, _) => elements.get(i).unwrap(),
			Meta { node, .. } => node.get(i),
			_ => &Empty,
		}
	}

	pub fn get_key(&self) -> &str {
		match self {
			Key(k, _, _) => match k.drop_meta() {
				Symbol(s) | Text(s) => s.as_str(),
				_ => "",
			},
			Meta { node, .. } => node.get_key(),
			_ => "",
		}
	}

	pub fn get_op(&self) -> Op {
		match self {
			Key(_, op, _) => *op,
			Meta { node, .. } => node.get_op(),
			_ => Op::None,
		}
	}

	pub fn get_value(&self) -> Node {
		match self {
			Key(_, _, v) => v.as_ref().clone(),
			Meta { node, .. } => node.get_value(),
			_ => Empty,
		}
	}

	pub fn iter(&self) -> NodeIter {
		match self {
			List(items, _, _) => NodeIter::new(items.clone()),
			Meta { node, .. } => node.iter(),
			_ => NodeIter::new(vec![]),
		}
	}

	/// The number of elements of a list (0 for anything else). Comments are never elements: the parser attaches them
	/// as metadata to the neighbouring element (`a // note` is one element), so no variant needs to count them.
	pub fn len(&self) -> usize {
		match self {
			List(items, _, _) => items.len(),
			Meta { node, .. } => node.len(),
			_ => 0,
		}
	}
	pub fn is_empty(&self) -> bool {
		self.len() == 0 || self == &Empty
	}

	/// The first Error node in the tree, e.g. a parse diagnostic
	pub fn first_error(&self) -> Option<&Node> {
		match self {
			Error(_) => Some(self),
			Key(left, _, right) => left.first_error().or_else(|| right.first_error()),
			List(items, _, _) => items.iter().find_map(Node::first_error),
			Meta { node, .. } => node.first_error(),
			Type { name, body } => name.first_error().or_else(|| body.first_error()),
			_ => None,
		}
	}

	/// Name of a key given twice in this object `{key: value …}`; code blocks may redefine
	pub fn duplicate_key(&self) -> Option<String> {
		let List(items, Bracket::Curly, _) = self.drop_meta() else { return None };
		let mut seen = std::collections::HashSet::new();
		items.iter().filter_map(|item| match item.drop_meta() {
			// repeated tags `div{…} div{…}` are children, as repeated elements in XML/HTML, not duplicate keys
			Key(_, Op::Colon, value) if matches!(value.drop_meta(), List(_, Bracket::Curly, _)) => None,
			Key(key, Op::Colon, _) => match key.drop_meta() {
				Symbol(name) if name == GLOBAL_DECLARATION => None,
				Symbol(name) | Text(name) => Some(name.clone()),
				_ => None,
			},
			_ => None,
		}).find(|name| !seen.insert(name.clone()))
	}

	pub fn is_falsy(&self) -> bool {
		match self {
			Empty => true,
			False => true,
			Node::Number(n) => n.zero(),
			Text(s) => s.is_empty(),
			Char('\0') => true,
			List(items, _, _) => items.is_empty(),
			Key(a, _, b) => a.is_falsy() && b.is_falsy(),
			Meta { node, .. } => node.is_falsy(), // metadata doesn't affect truthiness
			Error(_) => true, // a failed result: `if x {…}` checks it (wiki/null.md, DESIGN.md "Effects")
			_ => false,
		}
	}


}

impl fmt::Debug for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.serialize())
	}
}

pub struct NodeIter {
	items: Vec<Node>,
	index: usize,
}

impl NodeIter {
	fn new(items: Vec<Node>) -> Self {
		NodeIter { items, index: 0 }
	}
}

impl Iterator for NodeIter {
	type Item = Node;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index < self.items.len() {
			let item = self.items[self.index].clone();
			self.index += 1;
			Some(item)
		} else {
			None
		}
	}
}

impl IntoIterator for Node {
	type Item = Node;
	type IntoIter = NodeIter;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			List(items, _, _) => NodeIter::new(items),
			Meta { node, .. } => (*node).clone().into_iter(),
			_ => NodeIter::new(vec![]),
		}
	}
}

impl IntoIterator for &Node {
	type Item = Node;
	type IntoIter = NodeIter;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Bracket {
	Curly,  // '{'
	Square, // '['
	Round,  // '('
	Less,   // '<' rename to ?
	None,   // list via separator 1,2,3
	// brace or parenthesis
	Other(char, char),
}

impl Bracket {
	pub fn opening(&self) -> char {
		match self {
			Bracket::None => ' ',
			Bracket::Curly => '{',
			Bracket::Square => '[',
			Bracket::Round => '(',
			Bracket::Less => '<',
			Bracket::Other(open, _) => *open,
		}
	}
	pub fn closing(&self) -> char {
		match self {
			Bracket::None => ' ',
			Bracket::Curly => '}',
			Bracket::Square => ']',
			Bracket::Round => ')',
			Bracket::Less => '>',
			Bracket::Other(_, close) => *close,
		}
	}
}

impl fmt::Display for Bracket {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.opening())
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Separator {
	Space,     // ' ' - tightest binding
	Colon,     // ',' Comma
	Semicolon, // ';'
	Newline,   // '\n'
	Tab,       // '\t'
	None,      // no separator (default)
}

impl Separator {
	/// `;` and newline separate statements that run in order, never the arguments of a call
	pub fn separates_statements(&self) -> bool {
		matches!(self, Separator::Semicolon | Separator::Newline)
	}

	pub fn to_char(&self) -> Option<char> {
		match self {
			Separator::Space => Some(' '),
			Separator::Colon => Some(','),
			Separator::Semicolon => Some(';'),
			Separator::Newline => Some('\n'),
			Separator::Tab => Some('\t'),
			Separator::None => None,
		}
	}

	// Returns precedence: lower number = tighter binding
	pub fn precedence(&self) -> u8 {
		match self {
			Separator::Space => 0,
			// Separator::Tab => 1, //  "a,b,c d,e,f"  == "a b (c d) e f " in csv!
			// todo Tab depends on context!, also indent vs dedent !!
			Separator::Colon => 2,
			Separator::Semicolon => 3,
			Separator::Tab => 4, //  "a;b;c d;e;f"  == "((a b c) (d e f))" in tsv!
			Separator::Newline => 5,
			// Separator::Block => 5, //
			Separator::None => 255, // or -1?
		}
	}
}

impl fmt::Display for Separator {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Separator::Space => write!(f, " "),
			Separator::Colon => write!(f, ","),
			Separator::Semicolon => write!(f, ";"),
			Separator::Newline => writeln!(f),
			Separator::Tab => write!(f, "\t"),
			Separator::None => Ok(()),
		}
	}
}

impl fmt::Display for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Node::Number(Number::Int(n)) => write!(f, "{}", n),
			Node::Number(Number::Float(fl)) => write!(f, "{}", fl),
			Node::Number(Number::Real(real)) => write!(f, "{}", real),
			Node::Number(n) => write!(f, "{:?}", n),
			Text(s) | Symbol(s) => write!(f, "{}", s),
			Char(c) => write!(f, "{}", c),
			List(items, bracket, separator) => {
				write!(f, "{}", bracket)?;
				for (i, item) in items.iter().enumerate() {
					if i > 0 {
						write!(f, "{} ", separator)?;
					}
					write!(f, "{}", item)?;
				}
				write!(f, "{}", bracket.closing())
			}
			Meta { node, .. } => write!(f, "{}", node),
			Data(data) => match crate::units::describe(data) {
				Some(text) => write!(f, "{}", text),
				None => write!(f, "{:?}", self),
			},
			_ => write!(f, "{:?}", self),
		}
	}
}

pub fn print(p0: String) {
	println!("{}", p0);
}

pub fn text_node(p0: String) -> Node {
	Text(p0)
}

pub fn node(p0: &str) -> Node {
	Text(p0.s())
}

// ============ Free Convenience Constructors ============
// Short, ergonomic functions for creating Node values

pub fn data<T: 'static + Clone + PartialEq>(value: T) -> Node { Data(DataValue::new(value)) }

pub fn int(n: i64) -> Node { Number(Number::Int(n)) }

pub fn float(n: f64) -> Node {
	Number(Number::Float(n))
}

pub fn text(s: &str) -> Node {
	Text(s.to_string())
}

pub fn symbol(s: &str) -> Node {
	Symbol(s.to_string())
}

pub fn error(s: &str) -> Node {
	Error(Box::new(Text(s.to_string())))
}

pub fn error_node(n: Node) -> Node {
	Error(Box::new(n))
}

pub fn codepoint(c: char) -> Node {
	Char(c)
}

pub fn key(k: &str, v: Node) -> Node {
	Key(Box::new(Symbol(k.to_string())), Op::Colon, Box::new(v))
}

pub fn types(name: &str) -> Node {
	// Returns a Symbol with the type name, matching what type() introspection returns
	Symbol(name.to_string())
}


pub fn key_op(k: Node, op: Op, v: Node) -> Node {
	Key(Box::new(k), op, Box::new(v))
}

pub fn key_ops(k: String, op: Op, v: Node) -> Node {
	Key(Box::new(Symbol(format!(".{}", k))), op, Box::new(v))
}

pub fn list(xs: Vec<Node>) -> Node {
	List(xs, Bracket::Square, Separator::None)
}

pub fn block(xs: Vec<Node>) -> Node {
	List(xs, Bracket::Curly, Separator::None)
}

pub fn parens(xs: Vec<Node>) -> Node {
	List(xs, Bracket::Round, Separator::None)
}

pub fn ints(xs: Vec<i32>) -> Node {
	List(
		xs.into_iter().map(|x| int(x as i64)).collect(),
		Bracket::Square,
		Separator::None,
	)
}

pub fn floats(xs: Vec<f64>) -> Node {
	List(
		xs.into_iter().map(float).collect(),
		Bracket::Square,
		Separator::None,
	)
}

pub fn texts(xs: Vec<&str>) -> Node {
	List(
		xs.into_iter().map(text).collect(),
		Bracket::Square,
		Separator::None,
	)
}

pub fn symbols(xs: Vec<&str>) -> Node {
	List(
		xs.into_iter().map(symbol).collect(),
		Bracket::Square,
		Separator::None,
	)
}

pub fn strings(p0: Vec<&str>) -> Node {
	List(
		map(p0, |s| Text(s.to_string())),
		Bracket::Square,
		Separator::None,
	)
}

/// Operators the parser reads in front of one operand, with ø as the left one (`#x`, `-x`, `not x`, `√x`, `if c`)
fn writes_as_prefix(op: &Op) -> bool {
	op.is_prefix() || matches!(op, Op::Hash | Op::Sub | Op::Add | Op::If | Op::While)
}

/// An item of a class body that is code, a method in any of its forms (`f() := …`, `fn f() {…}`, `f = x => …`), not a
/// field declaration (`name: text`, `age: int = 0`): the lowering passes reach it as they reach a function
fn is_class_code(item: &Node) -> bool {
	let is_name = |node: &Node| matches!(node.drop_meta(), Symbol(_));
	match item.drop_meta() {
		Key(name, Op::Colon, _) if is_name(name) => return false,
		Key(declared, Op::Assign, _) if matches!(declared.drop_meta(), Key(name, Op::Colon, _) if is_name(name)) => return false,
		List(words, _, _) if words.first().is_some_and(|first| matches!(first.drop_meta(), Symbol(word) if crate::operators::is_function_keyword(word))) => return true,
		_ => {}
	}
	let mut code = false;
	item.visit(&mut |part| code |= matches!(part, Key(_, Op::Define | Op::Arrow | Op::FatArrow, _) | List(_, Bracket::Curly, _)));
	code
}
