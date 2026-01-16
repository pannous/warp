#![allow(dead_code, unused_imports)]
// type string = str; NO! ugly for a reason!
extern crate regex;
use crate::extensions::lists::{map, Filter, VecExtensions, VecExtensions2};
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::{CloneAny, Dada, DataType, LineInfo};
use crate::wasm_gc_reader::GcObject;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::cmp::PartialEq;
use std::fmt;
use std::ops::{Add, Div, Index, IndexMut, Mul, Not, Sub};
use crate::operators::{is_function_keyword, Op};
// use warp::type_kinds::{AstKind, NodeKind};
use crate::node::Node::*;
use crate::type_kinds::{AstKind, Kind};
use crate::wasp_parser::parse;



// use warp::Node;
// use warp::*; !
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
	Data(Dada), // most generic container for any kind of data not captured by other node types
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
	pub fn class(&self) -> Node {
		todo!("class via kind and/or metadata?")
	}
	pub fn typ(&self) -> Node {
		// including Ast(Node, AstKind) !
		todo!("typ via kind or field and/or metadata?")
	}
	pub fn is_nil(&self) -> bool {
		*self == Empty
	}
	pub fn data_value(&self) -> Dada {
		// 💡use via
		// let val = data.data_value().downcast_ref::<MyType>().unwrap().clone();
		match self {
			Data(dada) => dada.clone(),
			Meta { node, .. } => node.data_value(),
			_ => Dada {
				data: Box::new(()),
				type_name: "ø".to_string(),
				data_type: DataType::None,
			},
		}
	}
	pub fn remove(&self, from: i32, to: i32) {
		todo!("remove from {} to {}", from, to)
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
	pub fn laste(&self) -> Node {
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
			Meta { node, .. } => node.laste(),
			_ => Empty,
		}
	}
	pub fn print(&self) {
		println!("{:?}", self);
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
			_ => todo!("rewrap a.add(b) => (a b) ?"),
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
				Number::Int(_) => Kind::Int,
				Number::Float(_) => Kind::Float,
				_ => Kind::Float, // Quotient, Complex, Nan, Inf → Float
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

	// pub fn value(&self) -> Dada {
	//     match self {
	//         Node::Number(n) => Dada::new(n.clone()),
	//         Node::Text(s) => Dada::new(s.clone()),
	//         Node::Codepoint(c) => Dada::new(*c),
	//         Node::Data(dada) => dada.clone(),
	//         Node::Meta(node, _) => node.value(),
	//         Node::Key(_, _, v) => v.value(),
	//         _ => Dada::new(()), // empty Dada
	//     }
	// }

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

	pub fn name(&self) -> String {
		match self {
			Symbol(name) => name.clone(),
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
	/// Layout: kind (i64), data (ref null any), value (ref null $Node)
	pub fn from_gc_object(obj: &GcObject) -> Node {
		// Read the kind field (i64), lower 8 bits are the tag
		let kind = match obj.kind() {
			Ok(k) => k,
			Err(_) => return Empty, // Null ref becomes Empty
		};
		let tag = (kind & 0xFF) as u8;

		match tag {
			t if t == Kind::Empty as u8 => Empty,

			t if t == Kind::Int as u8 => {
				// data field contains boxed i64
				match obj.read_boxed_i64() {
					Ok(val) => Node::Number(Number::Int(val)),
					Err(_) => Node::Number(Number::Int(0)),
				}
			}

			t if t == Kind::Float as u8 => {
				// data field contains boxed f64
				match obj.read_boxed_f64() {
					Ok(val) => Node::Number(Number::Float(val)),
					Err(_) => Node::Number(Number::Float(0.0)),
				}
			}

			t if t == Kind::Text as u8 => {
				// data field contains $String struct
				match obj.text() {
					Ok(s) => Text(s),
					Err(e) => Text(format!("Error reading text: {}", e)),
				}
			}

			t if t == Kind::Codepoint as u8 => {
				// data field contains i31ref with codepoint
				match obj.read_i31() {
					Ok(code) => {
						if let Some(c) = char::from_u32(code as u32) {
							Char(c)
						} else {
							Char('\0')
						}
					}
					Err(_) => Char('\0'),
				}
			}

			t if t == Kind::Symbol as u8 => {
				// data field contains $String struct
				match obj.text() {
					Ok(s) => Symbol(s),
					Err(e) => Symbol(format!("Error reading symbol: {}", e)),
				}
			}

			t if t == Kind::Key as u8 => {
				// data field contains key node, value field contains value node
				// op_info is encoded in upper bits of kind: (op_info << 8) | Key
				let op_code = (kind >> 8) & 0xFF;
				let op = crate::wasm_gc_emitter::code_to_op(op_code);
				let key = match obj.data_as_node() {
					Ok(child_obj) => Box::new(Node::from_gc_object(&child_obj)),
					Err(_) => Box::new(Empty),
				};
				let value = match obj.value() {
					Ok(child_obj) => Box::new(Node::from_gc_object(&child_obj)),
					Err(_) => Box::new(Empty),
				};
				Key(key, op, value)
			}

			t if t == Kind::Block as u8 => {
				// data=first item, value=rest, bracket info in upper bits of kind
				Self::read_list_from_gc(obj, Bracket::Curly, kind)
			}

			t if t == Kind::List as u8 => {
				// data=first item, value=rest, bracket info in upper bits of kind
				let bracket_info = (kind >> 8) & 0xFF;
				let bracket = match bracket_info {
					0 => Bracket::Curly,
					1 => Bracket::Square,
					2 => Bracket::Round,
					3 => Bracket::Less,
					_ => Bracket::None,
				};
				Self::read_list_from_gc(obj, bracket, kind)
			}

			t if t == Kind::Data as u8 => {
				// For now, read type_name from text
				let type_name = obj.text().unwrap_or_default();
				Data(Dada {
					data: Box::new(format!("<wasm data: {}>", type_name)),
					type_name,
					data_type: DataType::Other,
				})
			}

			t if t == Kind::TypeDef as u8 => {
				// data = name node, value = body node
				let name = match obj.data_as_node() {
					Ok(child_obj) => Box::new(Node::from_gc_object(&child_obj)),
					Err(_) => Box::new(Empty),
				};
				let body = match obj.value() {
					Ok(child_obj) => Box::new(Node::from_gc_object(&child_obj)),
					Err(_) => Box::new(Empty),
				};
				Type { name, body }
			}

			_ => Text(format!("Unknown Kind: {}", tag)),
		}
	}

	/// Read a list from compact GC representation
	fn read_list_from_gc(obj: &GcObject, bracket: Bracket, _kind: i64) -> Node {
		let mut items = Vec::new();

		// data = first item
		if let Ok(first_obj) = obj.data_as_node() {
			let first = Self::from_gc_object(&first_obj);
			if first != Empty {
				items.push(first);
			}
		}

		// value = rest (either single item or nested list)
		if !obj.value_is_null() {
			if let Ok(rest_obj) = obj.value() {
				let rest = Self::from_gc_object(&rest_obj);
				match rest {
					List(rest_items, _, _) => items.extend(rest_items),
					Empty => {}
					other => items.push(other),
				}
			}
		}

		List(items, bracket, Separator::None)
	}

	pub fn todo(p0: String) -> Node {
		Text(format!("TODO: {}", p0))
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
//
// impl Index<Node> for Node {
// 	type Output = Node;
//
// 	fn index(&self, n: Node) -> &Self::Output {
// 		match self {
// 			List(elements, _, _) => match n {
// 				Number(Number::Int(i)) => elements.get(i).unwrap_or(&Empty),
// 				_ => &Empty,
// 			},
// 			Key(k, _, v) => {
// 				if **k == n {
// 					&v /* (a:b)[a]==b */
// 				} else {
// 					&v[n]  // Pass through to value: person:{x y}[0] => x
// 				}
// 			}
// 			Meta { node, .. } => &node[n],
// 			_ => &Empty,
// 		}
// 	}
// }

impl Index<usize> for Node {
	type Output = Node;

	fn index(&self, i: usize) -> &Self::Output {
		match self {
			List(elements, _, _) => elements.get(i).unwrap_or(&Empty),
			Key(_, _, v) => &v[i], // Pass through to value: person:{x y}[0] => x
			Meta { node, .. } => &node[i],
			_ => &Empty,
		}
	}
}

impl Index<&String> for Node {
	type Output = Node;

	fn index(&self, i: &String) -> &Self::Output {
		match self {
			List(nodes, _, _) => {
				if let Some(found) = nodes.find2(&|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => *t == *i,
					_ => false,
				}) {
					// If we found a Key, return its value instead of the whole Key
					match found.drop_meta() {
						Key(_, _, v) => v.as_ref(),
						other => other,
					}
				} else {
					&Empty
				}
			}
			Key(k, _, v) => match k.drop_meta() {
				Symbol(key) | Text(key) if key == i => v.as_ref(),
				_ => &v[i], // Pass through to value
			},
			Meta { node, data } => {
				if node[i] != Empty {
					&node[i]
				} else {
					&data[i]
				}
			}
			_ => &Empty,
		}
	}
}

// bob['a'] -> bob["a"] supperfluous but convenient
impl Index<&char> for Node {
	type Output = Node;
	fn index(&self, i: &char) -> &Self::Output {
		self.index(i.to_string().as_str())
	}
}

impl Index<&str> for Node {
	type Output = Node;

	fn index(&self, i: &str) -> &Self::Output {
		match self {
			List(nodes, _, _) => {
				// First, search directly in this list
				if let Some(found) = nodes.find2(&|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => t == i,
					_ => false,
				}) {
					// If we found a Key, return its value instead of the whole Key
					match found.drop_meta() {
						Key(_, _, v) => v.as_ref(),
						other => other,
					}
				} else {
					// For tag structures like ((name params...) body), search in params
					// Pattern: first element is a list containing [symbol, params...]
					if let Some(first) = nodes.first() {
						let first = first.drop_meta();
						if let List(inner, _, _) = first {
							// Skip the first element (tag name) and search in params
							for param in inner.iter().skip(1) {
								let result = &param[i];
								if result != &Empty {
									return result;
								}
							}
						}
					}
					&Empty
				}
			}
			Key(k, _, v) => match k.drop_meta() {
				Symbol(key) | Text(key) if key == i => v.as_ref(),
				_ => &v[i], // Pass through to value: person:{name:"Joe"}["name"] => "Joe"
			},
			Meta { node, data } => {
				if node[i] != Empty {
					&node[i]
				} else {
					&data[i]
				}
			}
			_ => &Empty,
		}
	}
}

impl Index<char> for Node {
	type Output = Node;

	fn index(&self, i: char) -> &Self::Output {
		self.index(i.to_string().as_str())
	}
}

impl IndexMut<usize> for Node {
	fn index_mut(&mut self, i: usize) -> &mut Self::Output {
		match self {
			List(elements, _, _) => {
				if i < elements.len() {
					&mut elements[i]
				} else {
					panic!("Index out of bounds")
				}
			}
			Meta { node, .. } => &mut node[i],
			_ => panic!("Cannot mutably index this node type"),
		}
	}
}

impl IndexMut<&String> for Node {
	fn index_mut(&mut self, i: &String) -> &mut Self::Output {
		match self {
			List(nodes, _, _) => {
				if let Some(found) = nodes.iter_mut().find(|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => t == i,
					_ => false,
				}) {
					// If we found a Key, return mutable reference to its value
					// Need to unwrap Meta first if present
					match found {
						Meta { node, .. } => match node.as_mut() {
							Key(_, _, v) => v.as_mut(),
							other => other,
						},
						Key(_, _, v) => v.as_mut(),
						other => other,
					}
				} else {
					panic!("Key '{}' not found", i)
				}
			}
			Key(_, _, v) => &mut v[i], // Pass through to value
			Meta { node, .. } => &mut node[i],
			_ => panic!("Cannot mutably index this node type"),
		}
	}
}

impl IndexMut<&str> for Node {
	fn index_mut(&mut self, i: &str) -> &mut Self::Output {
		&mut self[&i.to_string()]
	}
}

impl IndexMut<char> for Node {
	fn index_mut(&mut self, i: char) -> &mut Self::Output {
		&mut self[&i.to_string()]
	}
}

impl Node {
	// fn new() -> Self {
	// 	// can be extended via .add a[b]=c !?! test_mark_as_map wished ;)
	// 	Empty
	// }

	// associated 'static' functions
	pub fn key(s: &str, v: Node) -> Self {
		Key(Box::new(Symbol(s.to_string())), Op::Colon, Box::new(v))
	}
	pub fn key_with_op(k: Node, op: Op, v: Node) -> Self {
		Key(Box::new(k), op, Box::new(v))
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
		Data(Dada::new(value))
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
	// pub fn ints(xs:Vec<i32>) -> Self { Node::List(xs.into_iter().map(Node::Number).collect()) }
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

	pub fn with_comment(self, comment: String) -> Self {
		let comment = Node::key("comment", Node::text(&comment));
		Meta {
			node: Box::new(self),
			data: Box::new(comment),
		}
	}

	pub fn drop_meta(&self) -> &Node {
		match self {
			Meta { node, .. } => node.drop_meta(),
			_ => self,
		}
	}

	// get_meta data directly or Empty
	pub fn get_meta(&self) -> &Node {
		match self {
			// Meta { node: _ , data} =>
			Meta { data, .. } => data.as_ref(),
			_ => &Empty,
		}
	}

	pub fn get_meta_data(&self) -> Option<&Node> {
		match self {
			// Meta { node: _ , data} => Some(data.as_ref()),
			Meta { data, .. } => Some(data.as_ref()),
			_ => None,
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
	pub fn size(&self) -> usize {
		match self {
			List(elements, _, _) => elements.len(),
			Meta { node, .. } => node.size(),
			_ => 0,
		}
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

	pub fn serialize(&self) -> String {
		self.serialize_recurse(false)
	}

	pub fn meta_string(&self) -> String {
		// todo as impl for Meta?
		// Extract MetaData from Data node if present
		if self["comment"] != Empty {
			return format!("/* {} */", self["comment"]);
		}
		if let Data(dada) = self {
			if let Some(_info) = dada.downcast_ref::<LineInfo>() {
				// if line_info { // noone ever cares!
				// 	format!("/* line:{} column:{} */", info.line, info.column);
				// }
			} else {
				return format!("{:?}", dada);
			}
		}
		"".to_string()
	}

	pub fn serialize_recurse(&self, meta: bool) -> String {
		match self {
			Symbol(s) => s.clone(),
			Node::Number(n) => format!("{}", n),
			Text(t) => format!("'{}'", t),
			Char(c) => format!("'{}'", c),
			List(nodes, bracket, separator) => {
				let close = bracket.closing();
				if nodes.is_empty() {
					format!("{}{}", bracket, close)
				} else if nodes.len() == 1 {
					format!("{}{}{}", bracket, nodes[0].serialize(), close)
				} else {
					let items: Vec<String> = nodes.iter().map(|n| n.serialize()).collect();
					format!(
						"{}{}{}",
						bracket,
						items.join(&*(separator.to_string() + " ")),
						close
					)
				}
			}
			Key(k, op, v) => format!("{}{}{}", k, op, v.serialize_recurse(meta)),
			Error(e) => format!("Error({})", e.serialize_recurse(meta)),
			Empty => "ø".to_string(),
			True => "true".to_string(),
			False => "false".to_string(),
			Meta { node, data } => {
				let inner = node.serialize_recurse(meta);
				if meta {
					format!("{} {}", inner, data.meta_string())
				} else {
					inner
				}
			}
			Type { name, body } => format!("type {} {}", name.serialize_recurse(meta), body.serialize_recurse(meta)),
			Data(d) => format!("Data({:?})", d),
			// _ => format!("{:?}", self),
		}
	}

	pub fn iter(&self) -> NodeIter {
		match self {
			List(items, _, _) => NodeIter::new(items.clone()),
			Meta { node, .. } => node.iter(),
			_ => NodeIter::new(vec![]),
		}
	}

	// method `into_iter` can be confused for the standard trait method `std::iter::IntoIterator::into_iter`
	// Help: consider implementing the trait `std::iter::IntoIterator` or choosing a less ambiguous method name
	// Help: for further information visit
	// https://rust-lang.github.io/rust-clippy/rust-1.92.0/index.html#should_implement_trait
	// Note: `#[warn(clippy::should_implement_trait)]` on by default
	pub fn into_iter(self) -> NodeIter {
		match self {
			List(items, _, _) => NodeIter::new(items),
			Meta { node, .. } => (*node).clone().into_iter(),
			_ => NodeIter::new(vec![]),
		}
	}

	// fixme unify with size() ?
	// fixme create one variant which counts meta comment nodes and one which ignores them
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

	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		let value = self.to_json_value();
		serde_json::to_string_pretty(&value)
	}

	pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
		let value = self.to_json_value();
		serde_json::to_string(&value)
	}

	/// Convert Node to XML string representation
	/// Key nodes become XML tags, dotted keys (.attr) become attributes
	pub fn to_xml(&self) -> String {
		match self.drop_meta() {
			Key(tag_name, _, body) => {
				let mut attributes = Vec::new();
				let mut content_parts = Vec::new();

				// Separate attributes (dotted keys) from content
				match body.as_ref() {
					List(items, _, _) => {
						for item in items {
							match item.drop_meta() {
								Key(k, _, v) => {
									if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
										if let Some(attr_name) = key_str.strip_prefix('.') {
											// This is an attribute
											// Remove leading dot
											match v.as_ref() {
												True => {
													// Boolean attribute (no value)
													attributes.push(attr_name.to_string());
												}
												Text(s) | Symbol(s) => {
													attributes
														.push(format!("{}=\"{}\"", attr_name, s));
												}
												Number(n) => {
													attributes
														.push(format!("{}=\"{}\"", attr_name, n));
												}
												_ => {
													let val = Node::serialize(v);
													attributes
														.push(format!("{}=\"{}\"", attr_name, val));
												}
											}
										} else {
											// Non-attribute key - treat as content
											content_parts.push(item.to_xml());
										}
									} else {
										// Non-string key - treat as content
										content_parts.push(item.to_xml());
									}
								}
								_ => {
									// This is content
									content_parts.push(item.to_xml());
								}
							}
						}
					}
					Empty => {
						// Empty body
					}
					other => {
						// Single content item
						content_parts.push(other.to_xml());
					}
				}

				// Build XML tag
				let attrs_str = if attributes.is_empty() {
					String::new()
				} else {
					format!(" {}", attributes.join(" "))
				};

				if content_parts.is_empty() {
					// Self-closing tag
					format!("<{}{} />", tag_name, attrs_str)
				} else {
					// Tag with content
					let content = content_parts.join("");
					format!("<{}{}>{}</{}>", tag_name, attrs_str, content, tag_name)
				}
			}
			Text(s) => s.clone(),
			Symbol(s) => s.clone(),
			List(items, _, _) => {
				// Multiple items - convert each to XML
				items
					.iter()
					.map(|item| item.to_xml())
					.collect::<Vec<_>>()
					.join("")
			}
			Empty => String::new(),
			_ => {
				// For other node types, fall back to serialize
				self.serialize()
			}
		}
	}

	fn to_json_value(&self) -> serde_json::Value {
		use serde_json::{Map, Value};

		match self {
			True => Value::Bool(true),
			False => Value::Bool(false),
			Empty => Value::Null,
			Node::Number(Number::Int(n)) => Value::Number((*n).into()),
			Node::Number(Number::Float(f)) => serde_json::Number::from_f64(*f)
				.map(Value::Number)
				.unwrap_or(Value::Null),
			Node::Number(n) => Value::String(format!("{}", n)),
			Text(s) | Symbol(s) => Value::String(s.clone()),
			Char(c) => Value::String(c.to_string()),
			List(items, bracket, _) => {
				// Curly braces -> object with items, Square/Round -> array
				match bracket {
					Bracket::Curly => {
						let mut map = Map::new();
						for item in items {
							match item {
								Key(k, _, v) => {
									if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
										map.insert(key_str.clone(), v.to_json_value());
									}
								}
								List(nested, Bracket::Curly, _) => {
									// Nested curly lists become nested objects
									for nested_item in nested {
										if let Key(k, _, v) = nested_item {
											if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
												map.insert(key_str.clone(), v.to_json_value());
											}
										}
									}
								}
								other => {
									// let key = format!("item_{}", map.len());
									let key = format!("{}", map.len()); // just the number
									map.insert(key, other.to_json_value());
								}
							}
						}
						Value::Object(map)
					}
					_ => Value::Array(items.iter().map(|n| n.to_json_value()).collect()),
				}
			}
			Key(k, _, v) => {
				let mut map = Map::new();
				if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
					map.insert(key_str.clone(), v.to_json_value());
				}
				Value::Object(map)
			}
			Data(d) => {
				let mut map = Map::new();
				map.insert("_type".to_string(), Value::String(d.type_name.clone()));
				Value::Object(map)
			}
			Meta { node, data } => {
				// Encode metadata as dotted keys or .meta array
				if let List(items, ..) = data.as_ref() {
					let has_keys = items.iter().any(|n| matches!(n, Key(..)));

					if has_keys {
						// Extract dotted keys from metadata
						let mut map = Map::new();
						for item in items {
							if let Key(k, _, v) = item {
								map.insert(format!(".{}", k), v.to_json_value());
							}
						}
						// Add the wrapped value
						map.insert("_value".to_string(), node.to_json_value());
						Value::Object(map)
					} else {
						// Non-Key metadata: use .meta array
						let mut map = Map::new();
						map.insert(".meta".to_string(), data.to_json_value());
						map.insert("_value".to_string(), node.to_json_value());
						Value::Object(map)
					}
				} else if let Some(_info) = data.get_lineinfo() {
					node.to_json_value() // ignore lineinfo
				} else if **data != Empty {
					let inner = node.to_json_value();
					let meta_val = data.to_json_value();
					match inner {
						Value::Object(mut map) => {
							map.insert(".meta".to_string(), meta_val);
							Value::Object(map)
						}
						_ => {
							let mut map = Map::new();
							map.insert("_value".to_string(), inner);
							map.insert(".meta".to_string(), meta_val);
							Value::Object(map)
						}
					}
				} else {
					// No metadata, just unwrap
					node.to_json_value()
				}
			}
			Type { name, body } => {
				let mut map = Map::new();
				map.insert("_type".to_string(), name.to_json_value());
				map.insert("fields".to_string(), body.to_json_value());
				Value::Object(map)
			}
			Error(e) => {
				let mut map = Map::new();
				map.insert("_error".to_string(), e.to_json_value());
				Value::Object(map)
			}
		}
	}

	pub fn from_json(json: &str) -> Result<Node, serde_json::Error> {
		serde_json::from_str(json)
	}
}

impl fmt::Debug for Node {
	// impl fmt::Debug for Node {
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
		self.into_iter()
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
	fn opening(&self) -> char {
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
	pub fn from_char(ch: char) -> Self {
		match ch {
			' ' => Separator::Space,
			',' => Separator::Colon,
			';' => Separator::Semicolon,
			'\n' => Separator::Newline,
			'\t' => Separator::Tab,
			_ => Separator::None,
		}
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

impl PartialEq for Node {
	fn eq(&self, other: &Self) -> bool {
		match self {
			True => {
				match other {
					True => true,
					False => false,
					_ => other == self, // flip symmetric cases
				}
			}
			False => {
				match other {
					True => false,
					False => true,
					_ => other == self, // flip symmetric cases
				}
			} // flip symmetric cases:
			Empty => {
				match other {
					True => false,
					False => true,
					Empty => true,
					Symbol(s) => s.is_empty(), // todo disallow empty symbol
					Text(s) => s.is_empty(),
					Node::Number(n) => n == &Number::Int(0), // ⚠️ CAREFUL
					List(l, _, _) => l.is_empty(),
					_ => self.size() == 0,
				}
			}
			Node::Number(n) => match other {
				True => !n.zero(), //  2 == true ? sUrE?? hardcore todo Truthy rules
				// Node::True => match n {
				//     Number::Int(i) => *i == 1,
				//     Number::Float(f) => *f == 1.0,
				//     _ => false,
				// }
				False => n.zero(),
				Node::Number(n2) => n == n2,
				_ => false,
			},
			Symbol(s) => {
				match other {
					True => !s.is_empty(),
					False => s.is_empty(),
					Symbol(s2) | Text(s2) => s == s2,
					_ => false,
				}
			}
			Text(s) => match other {
				True => !s.is_empty(),
				False => s.is_empty(),
				Text(s2) | Symbol(s2) => s == s2,
				_ => false,
			},

			Char(c) => match other {
				True => c != &'\0',
				False => c == &'\0',
				Char(c2) => c == c2,
				Text(c2) => *c == c2.first(),
				_ => false,
			},
			Data(d) => match other {
				Data(d2) => d == d2,
				_ => false,
			},
			Meta { node, .. } => {
				// Ignore metadata when comparing equality - unwrap both sides
				let other_unwrapped = match other {
					Meta {
						node: other_node, ..
					} => other_node.as_ref(),
					_ => other,
				};
				node.as_ref().eq(other_unwrapped)
			}
			Key(k1, op1, v1) => match other {
				Key(k2, op2, v2) => k1 == k2 && op1 == op2 && v1 == v2,
				_ => false,
			},
			List(items1, _, _) => match other {
				List(items2, _, _) => items1 == items2,
				// ignore bracket [1,2]=={1,2} and separators [1;2]==[1,2]
				Meta { node, .. } => self == node.as_ref(), // unwrap Meta
				_ => false,
			},
			Type { name: n1, body: b1 } => match other {
				Type { name: n2, body: b2 } => n1 == n2 && b1 == b2,
				_ => false,
			},
			Error(e1) => match other {
				Error(e2) => e1 == e2,
				_ => false,
			},
		}
	}
}
impl PartialEq<str> for Node {
	fn eq(&self, other: &str) -> bool {
		match self {
			Text(s) => s == other,
			Symbol(s) => s == other,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<i64> for Node {
	fn eq(&self, other: &i64) -> bool {
		match self {
			Node::Number(Number::Int(n)) => n == other,
			Node::Number(Number::Float(f)) => *f == *other as f64,
			Key(_, _, v) => v.as_ref().eq(other), // Compare value of Key
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<bool> for Node {
	fn eq(&self, other: &bool) -> bool {
		match self {
			// todo 2 == true? NO only in truthy if(2) …
			Node::Number(n) => n == &if *other { 1 } else { 0 },
			// Node::Number(Number::Int(n)) => n == &if *other { 1 } else { 0 },
			// Node::Number(Number::Float(f)) => *f == if *other { 1.0 } else { 0.0 },
			Empty => !*other,
			Symbol(s) => s.is_empty() != *other,
			Text(s) => s.is_empty() != *other,
			List(l, _, _) => l.is_empty() != *other,
			Key(_, _, v) => v.is_nil() != *other, // Key is true if its value is non-empty
			_ => false,
		}
	}
}

impl PartialEq<i32> for Node {
	fn eq(&self, other: &i32) -> bool {
		self == (*other as i64)
	}
}

impl PartialEq<f64> for Node {
	fn eq(&self, other: &f64) -> bool {
		match self {
			Node::Number(Number::Float(f)) => f == other,
			Node::Number(Number::Int(n)) => *n as f64 == *other,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<&str> for Node {
	fn eq(&self, other: &&str) -> bool {
		match self {
			Text(s) => s == *other,
			Char(c) => *c == other.chars().next().unwrap_or('\0'),
			Symbol(s) => s == *other,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

// Reverse comparison: &str == Node
impl PartialEq<Node> for &str {
	fn eq(&self, other: &Node) -> bool {
		other.eq(self)
	}
}

impl PartialEq<char> for Node {
	fn eq(&self, other: &char) -> bool {
		match self {
			Char(c) => c == other,
			Text(s) => {
				// Check if string is exactly one char
				let mut chars = s.chars();
				chars.next() == Some(*other) && chars.next().is_none()
			}
			Symbol(s) => {
				// Check if string is exactly one char
				let mut chars = s.chars();
				chars.next() == Some(*other) && chars.next().is_none()
			}
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<&Node> for Node {
	fn eq(&self, other: &&Node) -> bool {
		self == *other
	}
}

// Allow &Node == primitive comparisons
impl PartialEq<i64> for &Node {
	fn eq(&self, other: &i64) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<i32> for &Node {
	fn eq(&self, other: &i32) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<f64> for &Node {
	fn eq(&self, other: &f64) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<bool> for &Node {
	fn eq(&self, other: &bool) -> bool {
		(*self).eq(other)
	}
}

// Reverse: bool == Node and bool == &Node
impl PartialEq<Node> for bool {
	fn eq(&self, other: &Node) -> bool {
		other.eq(self)
	}
}

impl PartialEq<&Node> for bool {
	fn eq(&self, other: &&Node) -> bool {
		(*other).eq(self)
	}
}

impl PartialEq<char> for &Node {
	fn eq(&self, other: &char) -> bool {
		(*self).eq(other)
	}
}

// Note: &str comparison works via blanket impl: impl<A,B> PartialEq<&B> for &A where A: PartialEq<B>
// Since Node implements PartialEq<str>, &Node automatically gets PartialEq<&str>

// Allow Box<Node> comparisons with str (for Key nodes with Symbol/Text keys)
impl PartialEq<str> for Box<Node> {
	fn eq(&self, other: &str) -> bool {
		self.as_ref().eq(other)
	}
}

impl PartialEq<&str> for Box<Node> {
	fn eq(&self, other: &&str) -> bool {
		self.as_ref().eq(*other)
	}
}

impl PartialOrd<i32> for Node {
	fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => (*n as i32).partial_cmp(other),
			Node::Number(Number::Float(f)) => (*f as i32).partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

impl PartialOrd<i64> for Node {
	fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => n.partial_cmp(other),
			Node::Number(Number::Float(f)) => (*f as i64).partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

impl PartialOrd<f64> for Node {
	fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => (*n as f64).partial_cmp(other),
			Node::Number(Number::Float(f)) => f.partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

// PartialEq with serde_json::Value for primitive types
impl PartialEq<serde_json::Value> for Node {
	fn eq(&self, other: &serde_json::Value) -> bool {
		use serde_json::Value;

		// Fully unwrap all nested Meta nodes (consistent with Node::eq behavior)
		let mut self_unwrapped = self;
		while let Meta { node, .. } = self_unwrapped {
			self_unwrapped = node.as_ref();
		}

		match (self_unwrapped, other) {
			// Null comparison
			(Empty, Value::Null) => true,

			// Boolean comparisons
			(True, Value::Bool(true)) => true,
			(False, Value::Bool(false)) => true,

			// Number comparisons
			(Node::Number(Number::Int(n)), Value::Number(json_n)) => json_n.as_i64() == Some(*n),
			(Node::Number(Number::Float(f)), Value::Number(json_n)) => json_n.as_f64() == Some(*f),

			// String comparisons (Text, Symbol, Char all map to JSON strings)
			(Text(s), Value::String(json_s)) => s == json_s,
			(Symbol(s), Value::String(json_s)) => s == json_s,
			(Char(c), Value::String(json_s)) => &c.to_string() == json_s,

			// List comparison (arrays and objects)
			(List(items, bracket, _), Value::Array(json_arr)) => {
				// Non-curly lists map to arrays
				if !matches!(bracket, Bracket::Curly) {
					if items.len() != json_arr.len() {
						return false;
					}
					items
						.iter()
						.zip(json_arr.iter())
						.all(|(node, json_val)| node == json_val)
				} else {
					false
				}
			}
			(List(items, bracket, _), Value::Object(json_obj)) => {
				// Curly lists map to objects
				if matches!(bracket, Bracket::Curly) {
					// Compare as object: each item should be a Key node
					if items.len() != json_obj.len() {
						return false;
					}
					items.iter().all(|item| {
						if let Key(k, _, v) = item {
							if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
								json_obj
									.get(key_str.as_str())
									.is_some_and(|json_val| v.as_ref() == json_val)
							} else {
								false
							}
						} else {
							false
						}
					})
				} else {
					false
				}
			}

			// Key comparison (single-key objects)
			(Key(k, _, v), Value::Object(json_obj)) => {
				if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
					json_obj.len() == 1
						&& json_obj
							.get(key_str.as_str())
							.is_some_and(|json_val| v.as_ref() == json_val)
				} else {
					false
				}
			}

			// All other combinations are not equal
			_ => false,
		}
	}
}

// Reverse comparison: serde_json::Value == Node
impl PartialEq<Node> for serde_json::Value {
	fn eq(&self, other: &Node) -> bool {
		other == self
	}
}

/// Trait for types that can be compared with Node::Data(GcObject)
/// Used by wasm_struct! macro to enable `is!("code", struct_value)` comparisons
pub trait GcComparable {
	/// Try to create Self from a GcObject
	fn try_from_gc(gc_obj: &crate::gc_traits::GcObject) -> Option<Self> where Self: Sized;
	/// Compare self with another instance
	fn gc_eq(&self, other: &Self) -> bool;
}

impl Node {
	/// Compare with a GcComparable type by extracting from Data variant
	pub fn eq_gc<T: GcComparable + std::fmt::Debug>(&self, other: &T) -> bool {
		if let Data(dada) = self {
			if let Some(gc_obj) = dada.downcast_ref::<crate::gc_traits::GcObject>() {
				if let Some(extracted) = T::try_from_gc(gc_obj) {
					return other.gc_eq(&extracted);
				}
			}
		}
		false
	}
}

/// Blanket impl: Node can be compared with any GcComparable type
/// This enables `assert_eq!(result, alice)` where alice is a wasm_struct! type
impl<T: GcComparable + std::fmt::Debug> PartialEq<T> for Node {
	fn eq(&self, other: &T) -> bool {
		self.eq_gc(other)
	}
}

// Implement Not for owned Node - returns Node for compatibility with existing tests
impl Not for Node {
	type Output = Node;

	fn not(self) -> Self::Output {
		if self.to_bool() {
			False
		} else {
			True
		}
	}
}

// Implement Not for &Node to support !&node["key"] syntax - returns bool
impl Not for &Node {
	type Output = bool;

	fn not(self) -> Self::Output {
		!self.to_bool()
	}
}

// From trait implementations for automatic conversion in assignments
impl From<&str> for Node {
	fn from(s: &str) -> Self {
		Text(s.to_string())
	}
}

impl From<String> for Node {
	fn from(s: String) -> Self {
		Text(s)
	}
}

impl From<i32> for Node {
	fn from(n: i32) -> Self {
		Node::Number(Number::Int(n as i64))
	}
}

impl From<i64> for Node {
	fn from(n: i64) -> Self {
		Node::Number(Number::Int(n))
	}
}

impl From<f32> for Node {
	fn from(n: f32) -> Self {
		Node::Number(Number::Float(n as f64))
	}
}

impl From<f64> for Node {
	fn from(n: f64) -> Self {
		Node::Number(Number::Float(n))
	}
}

impl From<bool> for Node {
	fn from(b: bool) -> Self {
		if b {
			True
		} else {
			False
		}
	}
}

impl From<char> for Node {
	fn from(c: char) -> Self {
		Char(c)
	}
}

// Allow Node to be converted to bool via .into() or bool::from()
impl From<Node> for bool {
	fn from(node: Node) -> Self {
		node.to_bool()
	}
}

impl From<&Node> for bool {
	fn from(node: &Node) -> Self {
		node.to_bool()
	}
}

// ============ Arithmetic Operators ============

// Add implementations
impl Add<&Node> for &Node {
	type Output = Node;

	fn add(self, rhs: &Node) -> Self::Output {
		// Handle Meta wrappers
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 + *n2),
			(True, True) => Node::Number(Number::Int(2)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) + *n),
			(Node::Number(n), True) => Node::Number(*n + Number::Int(1)),
			(False, Node::Number(n)) | (Node::Number(n), False) => Node::Number(*n),
			(Empty, Node::Number(n)) | (Node::Number(n), Empty) => Node::Number(*n),
			_ => panic!("Cannot add {:?} and {:?}", left, right),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Add<i64> for &Node {
	type Output = Node;
	fn add(self, rhs: i64) -> Self::Output {
		self + &Node::int(rhs)
	}
}

impl Add<f64> for &Node {
	type Output = Node;
	fn add(self, rhs: f64) -> Self::Output {
		self + &Node::float(rhs)
	}
}

impl Add<i32> for &Node {
	type Output = Node;
	fn add(self, rhs: i32) -> Self::Output {
		self + &Node::int(rhs as i64)
	}
}

impl Add<&Node> for i64 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::int(self) + rhs
	}
}

impl Add<&Node> for f64 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::float(self) + rhs
	}
}

impl Add<&Node> for i32 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) + rhs
	}
}

// Sub implementations
impl Sub<&Node> for &Node {
	type Output = Node;

	fn sub(self, rhs: &Node) -> Self::Output {
		// Handle Meta wrappers
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 - *n2),
			(True, True) => Node::Number(Number::Int(0)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) - *n),
			(Node::Number(n), True) => Node::Number(*n - Number::Int(1)),
			(Node::Number(n), False) => Node::Number(*n),
			(False, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Empty, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Node::Number(n), Empty) => Node::Number(*n),
			_ => panic!("Cannot subtract {:?} and {:?}", left, right),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Sub<i64> for &Node {
	type Output = Node;
	fn sub(self, rhs: i64) -> Self::Output {
		self - &Node::int(rhs)
	}
}

impl Sub<f64> for &Node {
	type Output = Node;
	fn sub(self, rhs: f64) -> Self::Output {
		self - &Node::float(rhs)
	}
}

impl Sub<i32> for &Node {
	type Output = Node;
	fn sub(self, rhs: i32) -> Self::Output {
		self - &Node::int(rhs as i64)
	}
}

impl Sub<&Node> for i64 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::int(self) - rhs
	}
}

impl Sub<&Node> for f64 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::float(self) - rhs
	}
}

impl Sub<&Node> for i32 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) - rhs
	}
}

// Mul implementations
impl Mul<&Node> for &Node {
	type Output = Node;

	fn mul(self, rhs: &Node) -> Self::Output {
		// Handle Meta wrappers
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 * *n2),
			(True, Node::Number(n)) | (Node::Number(n), True) => Node::Number(*n),
			(False, _) | (_, False) => Node::Number(Number::Int(0)),
			(Empty, _) | (_, Empty) => Node::Number(Number::Int(0)),
			_ => panic!("Cannot multiply {:?} and {:?}", left, right),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Mul<i64> for &Node {
	type Output = Node;
	fn mul(self, rhs: i64) -> Self::Output {
		self * &Node::int(rhs)
	}
}

impl Mul<f64> for &Node {
	type Output = Node;
	fn mul(self, rhs: f64) -> Self::Output {
		self * &Node::float(rhs)
	}
}

impl Mul<i32> for &Node {
	type Output = Node;
	fn mul(self, rhs: i32) -> Self::Output {
		self * &Node::int(rhs as i64)
	}
}

impl Mul<&Node> for i64 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::int(self) * rhs
	}
}

impl Mul<&Node> for f64 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::float(self) * rhs
	}
}

impl Mul<&Node> for i32 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) * rhs
	}
}

// Div implementations
impl Div<&Node> for &Node {
	type Output = Node;

	fn div(self, rhs: &Node) -> Self::Output {
		// Handle Meta wrappers
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 / *n2),
			(Node::Number(n), True) => Node::Number(*n / Number::Int(1)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) / *n),
			(False, Node::Number(_)) => Node::Number(Number::Int(0)),
			(Empty, Node::Number(_)) => Node::Number(Number::Int(0)),
			_ => panic!("Cannot divide {:?} and {:?}", left, right),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Div<i64> for &Node {
	type Output = Node;
	fn div(self, rhs: i64) -> Self::Output {
		self / &Node::int(rhs)
	}
}

impl Div<f64> for &Node {
	type Output = Node;
	fn div(self, rhs: f64) -> Self::Output {
		self / &Node::float(rhs)
	}
}

impl Div<i32> for &Node {
	type Output = Node;
	fn div(self, rhs: i32) -> Self::Output {
		self / &Node::int(rhs as i64)
	}
}

impl Div<&Node> for i64 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::int(self) / rhs
	}
}

impl Div<&Node> for f64 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::float(self) / rhs
	}
}

impl Div<&Node> for i32 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) / rhs
	}
}

impl fmt::Display for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Node::Number(Number::Int(n)) => write!(f, "{}", n),
			Node::Number(Number::Float(fl)) => write!(f, "{}", fl),
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

// Now no pass can accidentally drop meta!
fn map_node(n: Node, f: &impl Fn(Node) -> Node) -> Node {
	match n {
		Meta { node: inner, data } => Meta {
			node: Box::new(map_node(*inner, f)),
			data,
		},
		// Now no pass can accidentally drop meta!
		List(xs, br, sep) => List(xs.into_iter().map(|x| map_node(x, f)).collect(), br, sep),

		other => f(other),
	}
}

// ============ Free Convenience Constructors ============
// Short, ergonomic functions for creating Node values

pub fn data<T: 'static + Clone + PartialEq>(value: T) -> Node { Data(Dada::new(value)) }

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
	Node::Symbol(name.to_string())
}

pub fn type_definition(name: &str, body: Node) -> Node {
	Type {
		name: Box::new(Node::Symbol(name.to_string())),
		body: Box::new(body),
	}
}


pub fn key_op(k: Node, op: Op, v: Node) -> Node {
	Key(Box::new(k), op, Box::new(v))
}

// pub fn key_ops(k: &str, op: Op, v: Node) -> Node {
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