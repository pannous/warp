use crate::extensions::numbers::Number;
use crate::node::Node::{Empty, Error};
use crate::node::{Bracket, Grouper, Node};
use log::warn;
use std::fs;
use crate::meta::MetaData;

/// Read and parse a WASP file
pub fn parse_file(path: &str) -> Node {
    match fs::read_to_string(path) {
        Ok(content) => WaspParser::parse(&content),
        _ => Error(format!("Failed to read {}", path)),
    }
}


pub fn parse(input: &str) -> Node {
    if input.ends_with(".wasp") {
        return parse_file(input);
    }
    WaspParser::parse(input)
}


pub struct WaspParser {
    input: String,
    pos: usize,
    line: usize,
    column: usize,
    char: char,
    pub current_line: String,
}

impl WaspParser {
    pub fn new(input: String) -> Self {
        let current_line = input.lines().next().unwrap_or("").to_string();
        WaspParser {
            input,
            pos: 0,
            line: 1,
            column: 1,
            char: '\0',
            current_line,
        }
    }

    pub fn parse(input: &str) -> Node {
        let mut parser = WaspParser::new(input.to_string());
        let mut current_expression = Vec::new();
        let mut root_nodes = Vec::new();

        parser.skip_whitespace_and_comments();
        let char_count = parser.input.chars().count(); // for unicode correctness
        let _len = parser.input.len();
        while parser.pos < char_count {
            let c = parser.current_char();
            let node = parser.parse_value();
            // if node != Node::Empty { // Todo: Skip empty nodes only at END, if at all!
            // [lame:'Joe' last:ø] meaningful nodes!
            current_expression.push(node);
            // }
            parser.skip_whitespace_and_comments();
            if c == Some(';') {
                root_nodes.push(Node::list(current_expression));
                current_expression = Vec::new();
                parser.advance()
            }
        }
        if root_nodes.is_empty() {
            root_nodes = current_expression
        } else if !current_expression.is_empty() {
            root_nodes.push(Node::list(current_expression));
        }

        // If only one value, return it directly for backward compatibility
        if root_nodes.len() == 1 {
            root_nodes[0].clone()
        } else if root_nodes.is_empty() {
            Empty
        } else {
            Node::List(root_nodes)
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.input.chars().nth(self.pos + offset)
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.char = ch; // debug
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
                // Update current_line to the new line
                let lines: Vec<&str> = self.input.lines().collect();
                if self.line > 0 && self.line <= lines.len() {
                    self.current_line = lines[self.line - 1].to_string();
                } else {
                    self.current_line = String::new();
                }
            } else {
                self.column += 1;
            }
        }
        self.pos += 1;
    }

    fn get_position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn prev_char(&self) -> Option<char> {
        if self.pos > 0 {
            self.input.chars().nth(self.pos - 1)
        } else {
            None
        }
    }

    fn is_at_line_start(&self) -> bool {
        // Check if we're at the very beginning or right after whitespace/newline
        self.pos == 0 || self.prev_char().map_or(false, |ch| ch.is_whitespace())
    }


    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn consume_rest_of_line(&mut self) -> String {
        let mut line_comment = String::new();
        while let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.advance();
                break;
            }
            line_comment.push(ch);
            self.advance();
        }
        line_comment.trim().to_string()
    }

    fn skip_whitespace_and_comments(&mut self) -> Option<String> {
        let mut comment = None;
        loop {
            self.skip_whitespace();
            // Check for # line comment (shell-style, shebang)
            // Only treat # as comment if at line start (to allow list#index later)
            if self.current_char() == Some('#') && self.is_at_line_start() {
                self.advance(); // skip #
                comment = Some(self.consume_rest_of_line());
                continue;
            }

            // Check for // line comment
            if self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
                self.advance(); // skip first /
                self.advance(); // skip second /
                comment = Some(self.consume_rest_of_line());
                continue;
            }

            if self.pos >= self.input.len() {
                return None;
            }

            // Check for /* block comment */
            if self.current_char() == Some('/') && self.peek_char(1) == Some('*') {
                self.advance(); // skip /
                self.advance(); // skip *
                let mut block_comment = String::new();
                while let Some(ch) = self.current_char() {
                    if ch == '*' && self.peek_char(1) == Some('/') {
                        self.advance(); // skip *
                        self.advance(); // skip /
                        break;
                    }
                    block_comment.push(ch);
                    self.advance();
                }
                comment = Some(block_comment.trim().to_string());
                continue;
            }

            break;
        }
        comment
    }

    fn is_at_line_end(&self) -> bool {
        self.column == 0 && self.current_char() == Some('\n') || self.pos >= self.input.len()
    }

    fn parse_value(&mut self) -> Node {
        let comment = self.skip_whitespace_and_comments();

        // Capture position before parsing
        let (line, column) = self.get_position();
        if self.is_at_line_end() {
            return Empty;
        };
        let mut node = match self.current_char() {
            None => {
                self.advance();
                Empty // end of input ≠ ø !
            }
            Some(ch) => {
                match ch {
                    '"' | '\'' | '“' | '‘' | '«' => self.parse_string(),
                    '[' => self.parse_list(),
                    '{' => self.parse_block(),
                    // assert!(!'三'.is_numeric());
                    _ if ch.is_numeric() || ch == '-' => self.parse_number(),
                    _ if ch.is_alphabetic() || ch == '_' => self.parse_symbol_or_named_block(),
                    // _ if ch.is_ascii_graphic()
                    // _ if ch.is_control() => self.
                    _ => {
                        // todo implement the rest like operators, etc.
                        warn!("Unexpected character '{}' at line {}, column {}", ch, line, column);
                        self.advance();
                        Error(format!("Unexpected character '{}'", ch))
                    }
                }
            }
        };
        if node == Empty {
            return node;
        }
        // Attach metadata with position and comment
        let mut meta = MetaData::with_position(line, column);
        if let Some(c) = comment {
            meta.comment = Some(c);
        }
        node = node.with_meta(meta);
        node
    }

    fn parse_string(&mut self) -> Node {
        let quote = self.current_char().unwrap_or('"');
        self.advance(); // skip opening quote

        let mut s = String::new();
        while let Some(ch) = self.current_char() {
            if ch == quote {
                self.advance(); // skip closing quote

                // Single quotes with exactly one character become Codepoint
                if quote == '\'' {
                    let mut chars = s.chars();
                    if let Some(c) = chars.next() {
                        if chars.next().is_none() {
                            // Exactly one character
                            return Node::codepoint(c);
                        }
                    }
                }

                return Node::text(&s);
            }
            if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char() {
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        _ => s.push(escaped),
                    }
                    self.advance();
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }
        Error("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Node {
        let mut num_str = String::new();
        let mut has_dot = false;

        // todo edge case: leading plus
        if self.current_char() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        while let Some(ch) = self.current_char() {
            // assert!(!'三'.is_numeric());
            if ch.is_numeric() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if has_dot {
            num_str
                .parse::<f64>()
                .map(Node::float)
                .unwrap_or_else(|_| Error(format!("Invalid float: {}", num_str)))
        } else {
            num_str
                .parse::<i64>()
                .map(Node::int)
                .unwrap_or_else(|_| Error(format!("Invalid int: {}", num_str)))
        }
    }

    fn parse_symbol(&mut self) -> Result<String, String> {
        let mut symbol = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                symbol.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if symbol.is_empty() {
            Err("Empty symbol".to_string())
        } else {
            Ok(symbol)
        }
    }

    fn parse_symbol_or_named_block(&mut self) -> Node {
        let symbol = match self.parse_symbol() {
            Ok(s) => s,
            Err(e) => return Error(e),
        };
        self.skip_whitespace();

        // Check for named block: name{...} or name(...)
        match self.current_char() {
            Some('{') => {
                let block = self.parse_block();
                // Create Tag for named blocks: html{...} -> Tag("html", None, body)
                Node::tag(&symbol, block)
            }
            Some('(') => {
                // Function-like: def name(params){body} or name(params):value
                let params = match self.parse_parenthesized() {
                    Ok(p) => p,
                    Err(e) => return Error(e),
                };
                self.skip_whitespace();

                if self.current_char() == Some('{') {
                    let body = self.parse_block();
                    // Use Pair for function syntax: name(params) : body
                    let signature = Node::text(&format!("{}{}", symbol, params));
                    Node::pair(signature, body)
                } else if self.current_char() == Some(':') || self.current_char() == Some('=') {
                    // name(params):value or name(params)=value
                    self.advance();
                    self.skip_whitespace();
                    let value = self.parse_value();
                    let key = format!("{}{}", symbol, params);
                    Node::key(&key, value)
                } else {
                    // Just a call: name(params)
                    Node::text(&format!("{}{}", symbol, params))
                }
            }
            Some(':') | Some('=') => {
                // Key-value pair (both : and = are supported)
                self.advance();
                self.skip_whitespace();
                let value = self.parse_value();
                Node::key(&symbol, value)
            }
            _ => {
                // Just a symbol
                Node::symbol(&symbol)
            }
        }
    }

    fn parse_parenthesized(&mut self) -> Result<String, String> {
        let mut result = String::new();
        result.push('(');
        self.advance(); // skip '('

        let mut depth = 1;
        while let Some(ch) = self.current_char() {
            result.push(ch);
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                self.advance();
                if depth == 0 {
                    return Ok(result);
                }
                continue;
            }
            self.advance();
        }

        Err("Unterminated parentheses".to_string())
    }

    fn parse_list(&mut self) -> Node {
        self.advance(); // skip '['
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();

            if self.current_char() == Some(']') {
                self.advance();
                break;
            }

            let value = self.parse_value();
            if value != Empty {
                items.push(value);
            }
            self.skip_whitespace();

            match self.current_char() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => {}
            }
        }
        Node::list(items)
    }

    fn parse_block(&mut self) -> Node {
        self.advance(); // skip '{'
        let mut items = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.current_char() == Some('}') {
                self.advance();
                break;
            }

            if self.current_char().is_none() {
                return Error("Unterminated block".to_string());
            }

            let pos_before = self.pos;
            let value = self.parse_value();
            if value != Empty {
                items.push(value);
            }
            self.skip_whitespace_and_comments();

            // Optional comma/semicolon separator
            if let Some(ch) = self.current_char() {
                if ch == ',' || ch == ';' {
                    self.advance();
                }
            }

            // Safety check: if we haven't made progress and we're not at the end,
            // we're in an infinite loop - skip the problematic character
            if self.pos == pos_before && self.current_char() != Some('}') {
                if self.current_char().is_some() {
                    // Skip unexpected character to avoid infinite loop
                    self.advance();
                }
            }
        }

        Node::Block(items, Grouper::Object, Bracket::Curly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eq;

    #[test]
    fn test_parse_number() {
        let node = WaspParser::parse("42");
        eq!(node, 42);
    }

    #[test]
    fn test_parse_string() {
        let node = WaspParser::parse(r#""hello""#);
        eq!(node, "hello");
    }

    #[test]
    fn test_parse_symbol() {
        let node = WaspParser::parse("red");
        if let Node::Symbol(s) = node {
            eq!(s, "red");
        }
    }

    #[test]
    fn test_parse_list() {
        let node = WaspParser::parse("[1, 2, 3]");
        if let Node::List(items) = node {
            eq!(items.len(), 3);
            eq!(items[0], 1);
        }
    }

    #[test]
    fn test_parse_key_value() {
        let node = WaspParser::parse(r#"name: "Alice""#);
        eq!(node.get_key(), "name");
    }

    #[test]
    fn test_parse_named_block() {
        let node = WaspParser::parse("html{ }");
        // Named blocks become Tags
        if let Node::Tag { title, .. } = node.unwrap_meta() {
            eq!(title, "html");
        } else {
            panic!("Expected Tag node");
        }
    }

    #[test]
    fn test_parse_complex() {
        let input = r#"html{
            ul{ li:"hi" li:"ok" }
            colors=[red, green, blue]
        }"#;
        let node = WaspParser::parse(input);
        println!("{:?}", node);
        if let Node::Tag { title, .. } = node.unwrap_meta() {
            eq!(title, "html");
        } else {
            panic!("Expected Tag node");
        }
    }

    #[test]
    fn test_parse_function() {
        let input = "def myfun(a, b){ return a + b }";
        let node = WaspParser::parse(input);
        println!("{:?}", node);
        // Should be Pair(signature, body)
        if let Node::Pair(sig, body) = node {
            println!("Signature: {:?}, Body: {:?}", sig, body);
        }
    }

    #[test]
    fn test_parse_multiple_values() {
        // Multiple numbers
        let node = WaspParser::parse("1 2 3");
        if let Node::List(items) = node {
            eq!(items.len(), 3);
            eq!(items[0], 1);
            eq!(items[1], 2);
            eq!(items[2], 3);
        } else {
            panic!("Expected List node, got {:?}", node);
        }

        // Multiple symbols
        let node = WaspParser::parse("hello world");
        if let Node::List(items) = node {
            eq!(items.len(), 2);
            if let Node::Symbol(s) = &items[0].unwrap_meta() {
                eq!(s, "hello");
            }
            if let Node::Symbol(s) = &items[1].unwrap_meta() {
                eq!(s, "world");
            }
        } else {
            panic!("Expected List node, got {:?}", node);
        }

        // Single value should not be wrapped in List
        let node = WaspParser::parse("42");
        eq!(node, 42);

        // Empty input
        let node = WaspParser::parse("");
        eq!(node, Empty);
    }

    #[test]
    fn test_current_line() {
        let input = "line1\nline2\nline3";
        let parser = WaspParser::new(input.to_string());
        eq!(parser.current_line, "line1");

        let mut parser = WaspParser::new(input.to_string());
        // Advance to second line
        while parser.line == 1 && parser.current_char().is_some() {
            parser.advance();
        }
        eq!(parser.current_line, "line2");
    }
}
