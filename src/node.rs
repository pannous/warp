#![allow(dead_code, unused_imports)]

extern crate regex;

use  std::fmt;
use regex::Regex;
use std::cmp::PartialEq;
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use std::ops::Index; // node[i]
use std::any::Any;
use crate::extensions::lists::{Filter, map, VecExtensions, VecExtensions2};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::SerializeStruct;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Meta {
    pub comment: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl Meta {
    pub fn new() -> Self {
        Meta { comment: None, line: None, column: None }
    }

    pub fn with_comment(comment: String) -> Self {
        Meta { comment: Some(comment), line: None, column: None }
    }

    pub fn with_position(line: usize, column: usize) -> Self {
        Meta { comment: None, line: Some(line), column: Some(column) }
    }
}

// Custom trait for cloneable Any types with equality support
pub trait CloneAny: Any {
    fn clone_any(&self) -> Box<dyn CloneAny>;
    fn as_any(&self) -> &dyn Any;
    fn eq_any(&self, other: &dyn CloneAny) -> bool;
}

impl<T: 'static + Clone + PartialEq> CloneAny for T {
    fn clone_any(&self) -> Box<dyn CloneAny> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_any(&self, other: &dyn CloneAny) -> bool {
        if let Some(other_t) = other.as_any().downcast_ref::<T>() {
            self == other_t
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Vec,
    Tuple,
    Struct,
    Primitive,
    String,
    Other,
}

pub struct Dada {
    data: Box<dyn CloneAny>,
    pub type_name: String,
    pub data_type: DataType,
}

// most generic container for any kind of data not captured by other node types
// Vec, tuples, primitives, custom structs, etc.
// let v = Node::data(vec![1, 2, 3]);
// let t = Node::data((42, "answer"));
// let n = Node::data(CustomData { id: 42, name: "test" });

impl Dada {
    pub fn new<T: 'static + Clone + PartialEq>(data: T) -> Self {
        let type_name = std::any::type_name::<T>().to_string();
        let data_type = Self::infer_type(&type_name);
        Dada {
            data: Box::new(data),
            type_name,
            data_type,
        }
    }

    fn infer_type(type_name: &str) -> DataType {
        if type_name.starts_with("alloc::vec::Vec") || type_name.starts_with("std::vec::Vec") {
            DataType::Vec
        } else if type_name.starts_with('(') && type_name.ends_with(')') {
            DataType::Tuple
        } else if type_name.contains("::String") || type_name == "str" || type_name == "&str" {
            DataType::String
        } else if type_name.contains("::") {
            DataType::Struct
        } else if matches!(type_name, "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                                     | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
                                     | "f32" | "f64" | "bool" | "char") {
            DataType::Primitive
        } else {
            DataType::Other
        }
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.as_any().downcast_ref::<T>()
    }
}

impl Clone for Dada {
    fn clone(&self) -> Self {
        Dada {
            data: self.data.clone_any(),
            type_name: self.type_name.clone(),
            data_type: self.data_type.clone(),
        }
    }
}

impl fmt::Debug for Dada {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dada({:?}:{})", self.data_type, self.type_name)
    }
}

impl PartialEq for Dada {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq_any(other.data.as_ref())
    }
}

impl Serialize for Dada {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Dada", 2)?;
        state.serialize_field("type_name", &self.type_name)?;
        state.serialize_field("data_type", &self.data_type)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Dada {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DadaHelper {
            type_name: String,
            data_type: DataType,
        }

        let helper = DadaHelper::deserialize(deserializer)?;
        // Create a placeholder Dada with empty string
        Ok(Dada {
            data: Box::new(String::from("<deserialized>")),
            type_name: helper.type_name,
            data_type: helper.data_type,
        })
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub enum Node {
    Empty, // Null, Nill, None, Ø, ø null nill none nil
    // Number(i32),
    Number(Number),
    Text(String),
    Codepoint(char), // Single Unicode codepoint/character like 'a', '🍏'
    // String(String),
    Symbol(String),
    // Keyword(String), Call, Declaration … AST or here? AST!
    // Data(Box<dyn Any>), // use via if let Some(i) = data.downcast_ref::<myType>() {
    KeyValue(String, Box<Node>),
    Pair(Box<Node>, Box<Node>),
    Tag(String, Box<Node>, Box<Node>), // name, attributes, body - for html/xml: <tag attr="val">body or tag{body}  (use Empty for no attrs)
    Block(Vec<Node>, Grouper, Bracket),
    List(Vec<Node>), // same as Block
    Data(Dada), // most generic container for any kind of data not captured by other node types
    WithMeta(Box<Node>, Meta), // Wrapper to attach metadata to any node
    // List(Vec<Box<dyn Any>>), // ⚠️ Any means MIXTURE of any type, not just Node or int …
    // List(Vec<AllowedListTypes>), // ⚠️ must be explicit types
    // List(Vec<T>) // turns whole Node into a generic type :(
}

impl Node {
    pub fn todo(p0: String) -> Node {
        Node::Text(format!("TODO: {}", p0))
    }
}

impl Index<usize> for Node {
    type Output = Node;

    fn index(&self, i: usize) -> &Self::Output {
        match self {
            Node::List(elements) => elements.get(i).unwrap_or(&Node::Empty),
            Node::Block(nodes, ..) => nodes.get(i).unwrap_or(&Node::Empty),
            Node::WithMeta(node, _) => &node[i],
            _ => &Node::Empty,
        }
    }
}


impl Index<&String> for Node {
    type Output = Node;

    fn index(&self, i: &String) -> &Self::Output {
        match self {
            // Node::List(elements) => elements.filter(|node|
            //     match node {
            //         Node::KeyValue(k, _) => k == *i,
            //         Node::Text(t) => t == *i,
            //         _ => false
            //     }
            // ).next().unwrap_or(&Node::Empty),

            // Node::Block(nodes, ..) => nodes.get(i).unwrap_or(&Node::Empty),
            Node::Block(nodes, ..) => nodes.find2(&|node|
                match node {
                    Node::KeyValue(k, _) => *k == *i,
                    Node::Text(t) => *t == *i,
                    _ => false
                }
            ).unwrap_or(&Node::Empty),
            Node::WithMeta(node, _) => &node[i],
            _ => &Node::Empty,
        }
    }
}

impl Node {
    // associated 'static' functions
    pub fn new() -> Node { Node::Empty }
    pub fn pair(a: Node, b: Node) -> Self { Node::Pair(Box::new(a), Box::new(b)) }
    pub fn key(s: &str, v: Node) -> Self { Node::KeyValue(s.to_string(), Box::new(v)) }
    pub fn tag(name: &str, body: Node) -> Self {
        Node::Tag(name.to_string(), Box::new(Node::Empty), Box::new(body))
    }
    pub fn tag_with_attrs(name: &str, attrs: Node, body: Node) -> Self {
        Node::Tag(name.to_string(), Box::new(attrs), Box::new(body))
    }
    pub fn keys(s: &str, v: &str) -> Self { Node::KeyValue(s.to_string(), Box::new(Node::Text(v.to_string()))) }
    pub fn text(s: &str) -> Self { Node::Text(s.to_string()) }
    pub fn codepoint(c: char) -> Self { Node::Codepoint(c) }
    pub fn symbol(s: &str) -> Self { Node::Symbol(s.to_string()) }
    pub fn data<T: 'static + Clone + PartialEq>(value: T) -> Self { Node::Data(Dada::new(value)) }
    pub fn number(n: Number) -> Self { Node::Number(n) }
    pub fn int(n: i64) -> Self { Node::Number(Number::Int(n)) }
    pub fn float(n: f64) -> Self { Node::Number(Number::Float(n)) }
    pub fn list(xs:Vec<Node>) -> Self { Node::List(xs) }
    // pub fn ints(xs:Vec<i32>) -> Self { Node::List(xs.into_iter().map(Node::Number).collect()) }
    pub fn ints(xs:Vec<i32>) -> Self { Node::List(map(xs, |x| Node::Number(Number::Int(x as i64))))}

    pub fn with_meta(self, meta: Meta) -> Self { Node::WithMeta(Box::new(self), meta) }
    pub fn with_comment(self, comment: String) -> Self {
        Node::WithMeta(Box::new(self), Meta::with_comment(comment))
    }

    pub fn get_meta(&self) -> Option<&Meta> {
        match self {
            Node::WithMeta(_, meta) => Some(meta),
            _ => None,
        }
    }

    pub fn unwrap_meta(&self) -> &Node {
        match self {
            Node::WithMeta(node, _) => node.unwrap_meta(),
            _ => self,
        }
    }
    //  }

    // pub fn liste<T>(xs:Vec<T>) -> Self {
    //     match T {}
    // }
    // member functions taking self
    pub fn size(&self) -> usize {
        match self {
            Node::List(elements,..)  => elements.len(),
            Node::Block(nodes,..) => nodes.len(),
            Node::WithMeta(node, _) => node.size(),
            _ => 0,
        }
    }

    pub fn get(&self, i: usize) -> &Node {
        match self {
            Node::List(elements) => elements.get(i).unwrap(),
            Node::Block(nodes,..) => nodes.get(i).unwrap(),
            Node::WithMeta(node, _) => node.get(i),
            _ => &Node::Empty,
        }
    }

    pub fn get_key(&self) -> Option<&str> {
        match self {
            Node::KeyValue(k, _) => Some(k),
            Node::WithMeta(node, _) => node.get_key(),
            _ => None,
        }
    }

    pub fn get_value(&self) -> Option<&Node> {
        match self {
            Node::KeyValue(_, v) => Some(v),
            Node::WithMeta(node, _) => node.get_value(),
            _ => None,
        }
    }

    pub fn serialize(&self) -> String {
        let s = format!("{:?}", self);
        s.trim().to_string()
    }

    pub fn iter(&self) -> NodeIter {
        match self {
            Node::List(items) => NodeIter::new(items.clone()),
            Node::Block(items, _, _) => NodeIter::new(items.clone()),
            Node::WithMeta(node, _) => node.iter(),
            _ => NodeIter::new(vec![]),
        }
    }

    pub fn into_iter(self) -> NodeIter {
        match self {
            Node::List(items) => NodeIter::new(items),
            Node::Block(items, _, _) => NodeIter::new(items),
            Node::WithMeta(node, _) => (*node).clone().into_iter(),
            _ => NodeIter::new(vec![]),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let value = self.to_json_value();
        serde_json::to_string_pretty(&value)
    }

    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        let value = self.to_json_value();
        serde_json::to_string(&value)
    }

    fn to_json_value(&self) -> serde_json::Value {
        use serde_json::{Value, Map};

        match self {
            Node::Empty => Value::Null,
            Node::Number(Number::Int(n)) => Value::Number((*n).into()),
            Node::Number(Number::Float(f)) => {
                serde_json::Number::from_f64(*f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            Node::Number(n) => Value::String(format!("{}", n)),
            Node::Text(s) | Node::Symbol(s) => Value::String(s.clone()),
            Node::Codepoint(c) => Value::String(c.to_string()),
            Node::List(items) => {
                Value::Array(items.iter().map(|n| n.to_json_value()).collect())
            }
            Node::KeyValue(k, v) => {
                let mut map = Map::new();
                map.insert(k.clone(), v.to_json_value());
                Value::Object(map)
            }
            Node::Pair(a, b) => {
                Value::Array(vec![a.to_json_value(), b.to_json_value()])
            }
            Node::Tag(name, attrs, body) => {
                let mut map = Map::new();
                if **attrs != Node::Empty {
                    // Include attributes if present
                    map.insert("_attrs".to_string(), attrs.to_json_value());
                }
                map.insert(name.clone(), body.to_json_value());
                Value::Object(map)
            }
            Node::Block(items, _kind, bracket) => {
                // Curly braces -> object with items, Square/Round -> array
                match bracket {
                    Bracket::Curly => {
                        let mut map = Map::new();
                        for item in items {
                            match item {
                                Node::KeyValue(k, v) => {
                                    map.insert(k.clone(), v.to_json_value());
                                }
                                Node::Tag(name, attrs, body) => {
                                    // Tags become named keys
                                    if **attrs != Node::Empty {
                                        let mut tag_map = Map::new();
                                        tag_map.insert("_attrs".to_string(), attrs.to_json_value());
                                        tag_map.insert(name.clone(), body.to_json_value());
                                        map.extend(tag_map);
                                    } else {
                                        map.insert(name.clone(), body.to_json_value());
                                    }
                                }
                                Node::Block(nested, _, Bracket::Curly) => {
                                    // Nested blocks become nested objects
                                    for nested_item in nested {
                                        if let Node::KeyValue(k, v) = nested_item {
                                            map.insert(k.clone(), v.to_json_value());
                                        }
                                    }
                                }
                                other => {
                                    // Non-KeyValue items: try to infer a key
                                    let key = format!("item_{}", map.len());
                                    map.insert(key, other.to_json_value());
                                }
                            }
                        }
                        Value::Object(map)
                    }
                    _ => Value::Array(items.iter().map(|n| n.to_json_value()).collect())
                }
            }
            Node::Data(d) => {
                let mut map = Map::new();
                map.insert("_type".to_string(), Value::String(d.type_name.clone()));
                Value::Object(map)
            }
            Node::WithMeta(node, meta) => {
                let mut value = node.to_json_value();
                if let Some(comment) = &meta.comment {
                    if let Value::Object(ref mut map) = value {
                        map.insert("_comment".to_string(), Value::String(comment.clone()));
                    }
                }
                value
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
        match self {
            Node::Symbol(s) => write!(f, "{}", s),
            Node::Number(n) => write!(f, "{}", n),
            Node::Text(t) => write!(f, "'{}'", t),
            Node::Codepoint(c) => write!(f, "'{}'", c),
            Node::Block(nodes, _kind, bracket) => {
                if nodes.len() == 1 {
                    write!(f, "{:?} ", nodes.get(0).unwrap())
                } else {
                    match bracket {
                        Bracket::Curly => write!(f, "{{{:?}}}", nodes),
                        Bracket::Square => write!(f, "[{:?}]", nodes),
                        Bracket::Round => write!(f, "({:?})", nodes),
                        Bracket::Other(open, close) => write!(f, "{}{:?}{}", open, nodes, close),
                        // _ => panic!("Unknown bracket type {:?}", bracket.into())
                    }
                }
            }
            Node::KeyValue(k, v) => write!(f, "{}={:?}", k, v), // todo vs
            Node::Pair(a, b) => write!(f, "{:?}:{:?}", a, b),
            Node::Tag(name, attrs, body) => {
                match **attrs {
                    Node::Empty => write!(f, "{}{{{:?}}}", name, body),
                    _ => write!(f, "<{} {:?}>{:?}", name, attrs, body),
                }
            }
            Node::List(l) => write!(f, "{:?}", l), // always as [a,b,c] !
            Node::Data(d) => write!(f, "{:?}", d),
            Node::WithMeta(node, meta) => {
                if let Some(comment) = &meta.comment {
                    write!(f, "{:?} /* {} */", node, comment)
                } else {
                    write!(f, "{:?}", node)
                }
            }
            Node::Empty => write!(f, "ø"),
        }
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

impl<'a> IntoIterator for &'a Node {
    type Item = Node;
    type IntoIter = NodeIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Grouper {
    // see Type
    Object,
    // {}
    Group,
    // ()
    Pattern,
    // []
    // Other, // <…>
    Expression, // List of Symbols/Nodes to be evaluated without grouping () // MAYBE ';' ;)
    // Other(String, String),
    Other(char, char),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Bracket {
    Curly,
    Square,
    Round,
    // brace or parenthesis
    Other(char, char),
}





impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Node::Empty => {
                match other {
                    Node::Empty => true,
                    Node::Symbol(s) => s.is_empty(), // todo disallow empty symbol
                    Node::Text(s) => s.is_empty(),
                    Node::Number(n) => n == &Number::Int(0), // ⚠️ CAREFUL
                    Node::Block(b,_,_) => b.is_empty(),
                    Node::List(l) => l.is_empty(),
                    _ => self.size() == 0,
                }
            }
            Node::Number(n) => {
                match other {
                    Node::Number(n2) => n == n2,
                    _ => false,
                }

            }
            Node::Symbol(s) => {
                match other {
                Node::Symbol(s2) => s == s2,
                    // todo variable values? nah not here
                _ => return false,
            }}
            Node::Text(s) => {
                match other {
                    Node::Text(s2) => s == s2,
                    _ => false,
                }
            }
            Node::Codepoint(c) => {
                match other {
                    Node::Codepoint(c2) => c == c2,
                    _ => false,
                }
            }
            Node::Data(d) => {
                match other {
                    Node::Data(d2) => d == d2,
                    _ => false,
                }
            }
            Node::WithMeta(node, _) => {
                // Ignore metadata when comparing equality
                node.as_ref().eq(other)
            }
            // Node::KeyValue(_, _) => {}
            // Node::Block(_, _, _) => {}
            // Node::List(_) => {}
            _ => {
                panic!("unimplemented");
                // false
            },
        }
    }
}

impl PartialEq<i64> for Node {
    fn eq(&self, other: &i64) -> bool {
        match self {
            Node::Number(Number::Int(n)) => n == other,
            Node::Number(Number::Float(f)) => *f == *other as f64,
            Node::WithMeta(node, _) => node.as_ref().eq(other),
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
            Node::Empty => !*other,
            Node::Symbol(s) => s.is_empty() == !*other,
            Node::Text(s) => s.is_empty() == !*other,
            Node::Block(b, _ ,_) => b.is_empty() == !*other,
            Node::List(l) => l.is_empty() == !*other,
            Node::KeyValue(_,_) => *other, // todo NEVER false OR check value k=v ?
            Node::Pair(_, _)  => *other, // // todo NEVER false OR check value k:v ?
            _ => false,
        }
    }
}

impl PartialEq<i32> for Node {
    fn eq(&self, other: &i32) -> bool {
        self == &(*other as i64)
    }
}

impl PartialEq<f64> for Node {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Node::Number(Number::Float(f)) => f == other,
            Node::Number(Number::Int(n)) => *n as f64 == *other,
            Node::WithMeta(node, _) => node.as_ref().eq(other),
            _ => false,
        }
    }
}

impl PartialEq<&str> for Node {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Node::Text(s) => s == *other,
            Node::Symbol(s) => s == *other,
            Node::WithMeta(node, _) => node.as_ref().eq(other),
            _ => false,
        }
    }
}

impl PartialEq<char> for Node {
    fn eq(&self, other: &char) -> bool {
        match self {
            Node::Codepoint(c) => c == other,
            Node::Text(s) => {
                // Check if string is exactly one char
                let mut chars = s.chars();
                chars.next() == Some(*other) && chars.next().is_none()
            }
            Node::Symbol(s) => {
                // Check if string is exactly one char
                let mut chars = s.chars();
                chars.next() == Some(*other) && chars.next().is_none()
            }
            Node::WithMeta(node, _) => node.as_ref().eq(other),
            _ => false,
        }
    }
}

impl PartialEq<&Node> for Node {
    fn eq(&self, other: &&Node) -> bool {
        self == *other
    }
}

impl PartialOrd<i32> for Node {
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        match self {
            Node::Number(Number::Int(n)) => (*n as i32).partial_cmp(other),
            Node::Number(Number::Float(f)) => (*f as i32).partial_cmp(other),
            Node::WithMeta(node, _) => node.as_ref().partial_cmp(other),
            _ => None,
        }
    }
}

impl PartialOrd<i64> for Node {
    fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
        match self {
            Node::Number(Number::Int(n)) => n.partial_cmp(other),
            Node::Number(Number::Float(f)) => (*f as i64).partial_cmp(other),
            Node::WithMeta(node, _) => node.as_ref().partial_cmp(other),
            _ => None,
        }
    }
}

impl PartialOrd<f64> for Node {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        match self {
            Node::Number(Number::Int(n)) => (*n as f64).partial_cmp(other),
            Node::Number(Number::Float(f)) => f.partial_cmp(other),
            Node::WithMeta(node, _) => node.as_ref().partial_cmp(other),
            _ => None,
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Number(Number::Int(n)) => write!(f, "{}", n),
            Node::Number(Number::Float(fl)) => write!(f, "{}", fl),
            Node::Number(n) => write!(f, "{:?}", n),
            Node::Text(s) | Node::Symbol(s) => write!(f, "{}", s),
            Node::Codepoint(c) => write!(f, "{}", c),
            Node::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            },
            Node::WithMeta(node, _) => write!(f, "{}", node),
            _ => write!(f, "{:?}", self),
        }
    }
}