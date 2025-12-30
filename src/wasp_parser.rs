use crate::node::{Node, Kind, Bracket, Meta};
use crate::extensions::numbers::Number;

pub struct WaspParser {
    input: String,
    pos: usize,
    line: usize,
    column: usize,
}

impl WaspParser {
    pub fn new(input: String) -> Self {
        WaspParser { input, pos: 0, line: 1, column: 1 }
    }

    pub fn parse(input: &str) -> Result<Node, String> {
        let mut parser = WaspParser::new(input.to_string());
        parser.parse_value()
    }

    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.input.chars().nth(self.pos + offset)
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        self.pos += 1;
    }

    fn get_position(&self) -> (usize, usize) {
        (self.line, self.column)
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

    fn skip_whitespace_and_comments(&mut self) -> Option<String> {
        let mut comment = None;
        loop {
            self.skip_whitespace();

            // Check for // line comment
            if self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
                self.advance(); // skip first /
                self.advance(); // skip second /
                let mut line_comment = String::new();
                while let Some(ch) = self.current_char() {
                    if ch == '\n' {
                        self.advance();
                        break;
                    }
                    line_comment.push(ch);
                    self.advance();
                }
                comment = Some(line_comment.trim().to_string());
                continue;
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

    fn parse_value(&mut self) -> Result<Node, String> {
        let comment = self.skip_whitespace_and_comments();

        // Capture position before parsing
        let (line, column) = self.get_position();

        let mut node = match self.current_char() {
            Some('"') | Some('\'') => self.parse_string(),
            Some('[') => self.parse_list(),
            Some('{') => self.parse_block(),
            Some(ch) if ch.is_ascii_digit() || ch == '-' => self.parse_number(),
            Some(ch) if ch.is_alphabetic() || ch == '_' => self.parse_symbol_or_named_block(),
            Some(ch) => Err(format!("Unexpected character: {}", ch)),
            None => Err("Unexpected end of input".to_string()),
        }?;

        // Attach metadata with position and comment
        let mut meta = Meta::with_position(line, column);
        if let Some(c) = comment {
            meta.comment = Some(c);
        }
        node = node.with_meta(meta);

        Ok(node)
    }

    fn parse_string(&mut self) -> Result<Node, String> {
        let quote = self.current_char().unwrap();
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
                            return Ok(Node::codepoint(c));
                        }
                    }
                }

                return Ok(Node::text(&s));
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
        Err("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Result<Node, String> {
        let mut num_str = String::new();
        let mut has_dot = false;

        if self.current_char() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
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
            num_str.parse::<f64>()
                .map(Node::float)
                .map_err(|_| format!("Invalid float: {}", num_str))
        } else {
            num_str.parse::<i64>()
                .map(Node::int)
                .map_err(|_| format!("Invalid integer: {}", num_str))
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

    fn parse_symbol_or_named_block(&mut self) -> Result<Node, String> {
        let symbol = self.parse_symbol()?;
        self.skip_whitespace();

        // Check for named block: name{...} or name(...)
        match self.current_char() {
            Some('{') => {
                let block = self.parse_block()?;
                // Create Tag for named blocks: html{...} -> Tag("html", None, body)
                Ok(Node::tag(&symbol, block))
            }
            Some('(') => {
                // Function-like: def name(params){body}
                let params = self.parse_parenthesized()?;
                self.skip_whitespace();

                if self.current_char() == Some('{') {
                    let body = self.parse_block()?;
                    // Use Pair for function syntax: name(params) : body
                    let signature = Node::text(&format!("{}{}", symbol, params));
                    Ok(Node::pair(signature, body))
                } else {
                    // Just a call: name(params)
                    Ok(Node::text(&format!("{}{}", symbol, params)))
                }
            }
            Some(':') | Some('=') => {
                // Key-value pair (both : and = are supported)
                self.advance();
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(Node::key(&symbol, value))
            }
            _ => {
                // Just a symbol
                Ok(Node::symbol(&symbol))
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

    fn parse_list(&mut self) -> Result<Node, String> {
        self.advance(); // skip '['
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();

            if self.current_char() == Some(']') {
                self.advance();
                break;
            }

            items.push(self.parse_value()?);
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

        Ok(Node::list(items))
    }

    fn parse_block(&mut self) -> Result<Node, String> {
        self.advance(); // skip '{'
        let mut items = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.current_char() == Some('}') {
                self.advance();
                break;
            }

            if self.current_char().is_none() {
                return Err("Unterminated block".to_string());
            }

            items.push(self.parse_value()?);
            self.skip_whitespace_and_comments();

            // Optional comma/semicolon separator
            if let Some(ch) = self.current_char() {
                if ch == ',' || ch == ';' {
                    self.advance();
                }
            }
        }

        Ok(Node::Block(items, Kind::Object, Bracket::Curly))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let node = WaspParser::parse("42").unwrap();
        assert_eq!(node, 42);
    }

    #[test]
    fn test_parse_string() {
        let node = WaspParser::parse(r#""hello""#).unwrap();
        assert_eq!(node, "hello");
    }

    #[test]
    fn test_parse_symbol() {
        let node = WaspParser::parse("red").unwrap();
        if let Node::Symbol(s) = node {
            assert_eq!(s, "red");
        }
    }

    #[test]
    fn test_parse_list() {
        let node = WaspParser::parse("[1, 2, 3]").unwrap();
        if let Node::List(items) = node {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], 1);
        }
    }

    #[test]
    fn test_parse_key_value() {
        let node = WaspParser::parse(r#"name: "Alice""#).unwrap();
        assert_eq!(node.get_key().unwrap(), "name");
    }

    #[test]
    fn test_parse_named_block() {
        let node = WaspParser::parse("html{ }").unwrap();
        // Named blocks become Tags
        if let Node::Tag(name, _, _) = node.unwrap_meta() {
            assert_eq!(name, "html");
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
        let node = WaspParser::parse(input).unwrap();
        println!("{:?}", node);
        if let Node::Tag(name, _, _) = node.unwrap_meta() {
            assert_eq!(name, "html");
        } else {
            panic!("Expected Tag node");
        }
    }

    #[test]
    fn test_parse_function() {
        let input = "def myfun(a, b){ return a + b }";
        let node = WaspParser::parse(input).unwrap();
        println!("{:?}", node);
        // Should be Pair(signature, body)
        if let Node::Pair(sig, body) = node {
            println!("Signature: {:?}, Body: {:?}", sig, body);
        }
    }
}
