#![allow(dead_code, unused_imports)]

extern crate regex;

use std::fmt;
use regex::Regex;

use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use std::ops::Index; // node[i]
use std::any::Any;
use crate::extensions::lists::{Filter, map, VecExtensions, VecExtensions2};

// pub enum AllowedListTypes {
//     Int,
//     Float,
//     String,
//     Symbol,
//     Node,
// }


// #[derive(Clone, Debug)]
// pub struct Dada(Box<dyn Any>);
// pub struct Dada(Box<dyn Any + Clone>);

// impl Dada {}


#[derive(Clone)]
pub enum Node {
    Empty,
    // Number(i32),
    Number(Number),
    Symbol(String),
    Text(String),
    // Data(Box<dyn Any>), // use via if let Some(i) = data.downcast_ref::<myType>() {
    // Data(Dada), // use via if let Some(i) = data.downcast_ref::<myType>() {
    KeyValue(String, Box<Node>),
    Block(Vec<Node>, Kind, Bracket),
    List(Vec<Node>), // same as Block
    // List(Vec<Box<dyn Any>>), // ⚠️ Any means MIXTURE of any type, not just Node or int …
    // List(Vec<AllowedListTypes>), // ⚠️ must be explicit types
    // List(Vec<T>) // turns whole Node into a generic type :(
}


impl Index<usize> for Node {
    type Output = Node;

    fn index(&self, i: usize) -> &Self::Output {
        match self {
            Node::List(elements) => elements.get(i).unwrap_or(&Node::Empty),
            Node::Block(nodes, ..) => nodes.get(i).unwrap_or(&Node::Empty),
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
            _ => &Node::Empty,
        }
    }
}

impl Node {
    // associated 'static' functions
    pub fn new() -> Node { Node::Empty }
    pub fn key(s: &str, v: Node) -> Self { Node::KeyValue(s.to_string(), Box::new(v)) }
    pub fn keys(s: &str, v: &str) -> Self { Node::KeyValue(s.to_string(), Box::new(Node::Text(v.to_string()))) }
    pub fn text(s: &str) -> Self { Node::Text(s.to_string()) }
    pub fn symbol(s: &str) -> Self { Node::Symbol(s.to_string()) }
    pub fn number(n: i64) -> Self { Node::Number(Number::Int(n)) }
    pub fn float(n: f64) -> Self { Node::Number(Number::Float(n)) }
    pub fn list(xs:Vec<Node>) -> Self { Node::List(xs) }
    // pub fn ints(xs:Vec<i32>) -> Self { Node::List(xs.into_iter().map(Node::Number).collect()) }
    pub fn ints(xs:Vec<i32>) -> Self { Node::List(map(xs, |x| Node::Number(Number::Int(x as i64))))}
    //  }

    // pub fn liste<T>(xs:Vec<T>) -> Self {
    //     match T {}
    // }
    // member functions taking self
    pub fn size(&self) -> usize {
        match self {
            Node::List(elements,..)  => elements.len(),
            Node::Block(nodes,..) => nodes.len(),
            _ => 0,
        }
    }
    
    pub fn get(&self, i: usize) -> &Node {
        match self {
            Node::List(elements) => elements.get(i).unwrap(),
            Node::Block(nodes,..) => nodes.get(i).unwrap(),
            _ => &Node::Empty,
        }
    }

    pub fn serialize(&self) -> String {
        let s = format!("{:?}", self);
        s.trim().to_string()
    }
}

impl fmt::Debug for Node {
    // impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Symbol(s) => write!(f, "{}", s),
            Node::Number(n) => write!(f, "{}", n),
            Node::Text(t) => write!(f, "'{}'", t),
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
            Node::KeyValue(k, v) => write!(f, "{}: {:?}", k, v),
            Node::List(l) => write!(f, "{:?}", l), // always as [a,b,c] !
            Node::Empty => write!(f, "ø"),
            // Node::Data(x) => write!(f, "{:?}", x)
            // _ => write!(f, "ø"),
        }
    }
}

#[derive(Clone)]
pub enum Kind {
    Object,
    // {}
    Group,
    // ()
    Pattern,
    // []
    // Other, // <…>
    // Other(String, String),
    Other(char, char),
}

#[derive(Clone)]
pub enum Bracket {
    Curly,
    Square,
    Round,
    // brace or parenthesis
    Other(char, char),
}




use std::cmp::PartialEq;

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Node::Empty => {
                match other {
                    Node::Empty => true,
                    Node::Symbol(s) => s.is_empty(), // disallow empty symbol
                    Node::Text(s) => s.is_empty(), // disallow empty symbol
                    Node::Number(n) => n == &Number::Int(0), // ⚠️ CAREFUL!
                    _ => self.size() == 0,
                }
            }
            Node::Number(n) => {
                match other {
                    Node::Number(n2) => n == n2,
                    _ => false,
                }

            }
            // Node::Symbol(_) => {}
            // Node::Text(_) => {}
            // Node::Data(_) => {}
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