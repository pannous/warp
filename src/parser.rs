#![allow(dead_code, unused_imports)]

extern crate regex;

use std::fmt;
use regex::Regex;

use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;

pub enum Node {
    Symbol(String),
    Number(Number),
    Text(String),
    // "quoted text"
    Block(Vec<Node>, Kind, Bracket),
    KeyValue(String, Box<Node>),
    List(Vec<Node>),
    Empty,
}
impl Node {
    pub fn new() -> Node {
        Node::Empty
    }
    pub fn symbol(s: &str) -> Node {
        Node::Symbol(s.to_string())
    }
    pub fn number(n: i64) -> Node {
        Node::Number(Number::Int(n))
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
            Node::Empty => write!(f, "Empty"),
            // _ => {}
        }
    }
}

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

pub enum Bracket {
    Curly,
    Square,
    Round,
    // brace or parenthesis
    Other(char, char),
}


pub struct Parser {
    tokens: Vec<String>,
    current: usize,
}

fn tokenize(input: &str) -> Vec<String> {
    // Match sequences of alphanumeric characters or any single non-alphanumeric character
    let re = Regex::new(r"(\w+|\W)").unwrap();
    re.find_iter(input)
        .map(|mat| mat.as_str().to_string())
        .filter(|s| !s.is_empty() && s != " " && s != "\n")
        .collect()
}

impl Parser {
    pub fn new(input: &str) -> Self {
        println!("tokenizing: {:?}", input);
        let tokens = tokenize(input);
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Node {
        self.parse_code()
    }

    fn parse_code(&mut self) -> Node {
        if self.current >= self.tokens.len() {
            return Node::Empty;
        }
        let tokens: &str = self.tokens.get(self.current).unwrap();
        let token: String = tokens.to_string();
        self.current += 1;
        if Number::is_number(&token) {
            return Node::Number(Number::parse(&token));
        }
        match tokens {
            " " => Node::Empty,
            "'" | "\"" => self.parse_quote(&token),
            "{" => {
                self.parse_block(&token, "}")
            }
            "[" => {
                self.parse_block(&token, "]")
            }
            _ => {
                // we already progressed with self.current += 1;
                // look ahead to see if it's a key value pair
                if let Some(next) = self.tokens.get(self.current) {
                    // if next == "," { // currently only in parse_block
                    //     self.current+=1;
                    //     return self.parse_sequence(token);
                    // }
                    if next == ":" {
                        self.current += 1;
                        let key = token.parse().unwrap();
                        let value = self.parse_code();
                        return Node::KeyValue(key, Box::new(value));
                    }
                }
                Node::Symbol(token)
            }
        }
    }

        // fn parse_sequence(&mut self) -> Node {
        //     let mut sequence = Vec::new();
    // while let Some(next) = self.tokens.get(self.current) {
    //             if next == "," {
    //                 self.current += 1;
    //                 if let Some(token) = self.tokens.get(self.current) {
    //                     sequence.push(Node::Symbol(token.to_string()));
    //                     self.current += 1;
    //                 }
    //             } else {
    //                 break;
    //             }
    //         }


    fn parse_sequence(&mut self, initial_token: String) -> Node {
        let mut sequence = vec![Node::Symbol(initial_token)];
        while let Some(token) = self.tokens.get(self.current){
            match token.as_str() {
                "," | ";" | " " => {
                    self.current += 1; // Consume the ","
                }
                _ => {
                    let element = self.parse_code();
                    sequence.push(element);
                }
            }
        }
        Node::List(sequence)
    }

    fn parse_quote(&mut self, kind: &String) -> Node {
        let mut text = String::new();
        while !self.check(kind) && !self.eof() {
            if let Some(token) = self.tokens.get(self.current) {
                text.push_str(token);
                self.current += 1;
            }
        }
        self.expect_token(kind);
        Node::Text(text)
    }

    // fn closing_bracket(&mut self, kind: &String) -> String {
    //     match kind.as_str() {
    //         "{" => "}".to_string(),
    //         "[" => "]".to_string(),
    //         _ => panic!("Invalid bracket kind")
    //     }
    // }

    // maybe char was premature? begin … end  TAB indent … dedent
    fn closing_bracket(&mut self, kind: char) -> char {
        match kind {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            _ => panic!("Unknown closing bracket for kind {}", kind)
        }
    }

    fn bracket_kind(&mut self, kind: char) -> Kind {
        match kind {
            '{' => Kind::Object,
            '(' => Kind::Group,
            '[' => Kind::Pattern,
            _ => Kind::Other(kind, self.closing_bracket(kind))
        }
    }

    fn parse_block(&mut self, bracket: &String, closing: &str) -> Node {
        let mut nodes = Vec::new();
        while !self.check(closing) && !self.eof() {
            let node = self.parse_code() ;
            nodes.push(node);
            self.skip_token(",");
            self.skip_token(";");
        }
        self.expect_token(closing);
        Node::Block(nodes, self.bracket_kind(bracket.first_char()), Bracket::Curly)
    }


    fn skip_token(&mut self, expected: &str) {
        if self.tokens.get(self.current).map_or(false, |t| t == expected) {
            self.current += 1;
        }
    }

    fn expect_token(&mut self, expected: &str) {
        if self.tokens.get(self.current).map_or(false, |t| t == expected) {
            self.current += 1;
        } else {
            panic!("Expected token '{}', found {:?}", expected, self.tokens.get(self.current));
        }
    }


    fn lookahead_token(&mut self, expected: &str) -> bool {
        for i in self.current..self.tokens.len() {
            if self.tokens.get(i).map_or(false, |t| t == expected) {
                return true;
            }
            if self.tokens.get(i).map_or(false, |t| t == "\n") { // stop at newline
                return false;
            }
        }
        false
    }

    fn match_token(&mut self, expected: &str) -> bool {
        if self.check(expected) {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &str) -> bool {
        self.tokens.get(self.current).map_or(false, |t| t == expected)
    }
    fn eof(&self) -> bool {
        self.current >= self.tokens.len()
    }
}

pub fn test_parser() {
    let code = "{ key: [ value, { key2: value2, num:123, text:'yeah' } ] }";
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    println!("{:#?}", ast);
    println!("Parsed: {:?}", code);
}
