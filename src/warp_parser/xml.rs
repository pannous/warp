//! XML inside warp: text content, processing instructions, comments, doctype, CDATA, tags

use super::*;

impl WarpParser {
	/// Parse XML text content (everything until '<' or end of input)
	pub(super) fn parse_xml_text_content(&mut self) -> String {
		let mut text = String::new();
		while !self.end_of_input() && self.current_char() != '<' {
			text.push(self.current_char());
			self.advance();
		}
		text.trim().to_string()
	}

	/// Skip XML processing instruction: <?xml ... ?>
	pub(super) fn skip_processing_instruction(&mut self) -> Node {
		self.advance(); // skip '?'
		while !self.end_of_input() {
			if self.current_char() == '?' && self.peek_char(1) == '>' {
				self.advance_by(2);
				return Empty;
			}
			self.advance();
		}
		Empty
	}

	/// Skip XML comment: <!--...-->
	pub(super) fn skip_xml_comment(&mut self) -> Node {
		self.advance_by(2); // skip '--'
		while !self.end_of_input() {
			if self.current_char() == '-' && self.peek_char(1) == '-' && self.peek_char(2) == '>' {
				self.advance_by(3);
				return Empty;
			}
			self.advance();
		}
		Empty
	}

	/// Skip DOCTYPE declaration: <!DOCTYPE ...>
	/// Handles both simple and complex DOCTYPE with internal subset
	pub(super) fn skip_doctype(&mut self) -> Node {
		// Skip until '>', handling nested brackets in internal subset
		let mut bracket_depth = 0;

		while !self.end_of_input() {
			let ch = self.current_char();

			if ch == '[' {
				bracket_depth += 1;
			} else if ch == ']' {
				bracket_depth -= 1;
			} else if ch == '>' && bracket_depth == 0 {
				self.advance(); // skip '>'
				return Empty; // DOCTYPE declarations are skipped
			}

			self.advance();
		}

		Empty
	}

	/// Parse CDATA section: <![CDATA[...]]>
	pub(super) fn parse_cdata(&mut self) -> Node {
		// Expect: [CDATA[
		let marker = "[CDATA[";
		if !marker.chars().enumerate().all(|(i, c)| self.peek_char(i) == c) {
			return Empty;
		}
		self.advance_by(marker.len());

		let mut content = String::new();
		while !self.end_of_input() {
			if self.current_char() == ']' && self.peek_char(1) == ']' && self.peek_char(2) == '>' {
				self.advance_by(3);
				return Node::Text(content);
			}
			content.push(self.current_char());
			self.advance();
		}
		Node::Text(content)
	}

	pub(super) fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == '\n' || self.pos >= self.input.len()
	}

	/// A `;` that ends its line is a statement terminator of the same strength as the newline after it
	pub(super) fn only_blanks_before_newline(&self) -> bool {
		self.chars[self.pos..].iter().find(|ch| !matches!(ch, ' ' | '\t' | '\r')) == Some(&'\n')
	}

	/// Parse XML tag: <tag attr="value">content</tag> or <tag />
	pub(super) fn parse_xml_tag(&mut self) -> Node {
		self.advance(); // skip '<'

		// Handle XML directives and special constructs
		if self.current_char() == '?' {
			// Processing instruction: <?xml ... ?> or <?xml-stylesheet ... ?>
			return self.skip_processing_instruction();
		}

		if self.current_char() == '!' {
			// Could be: <!--comment-->, <!DOCTYPE...>, or <![CDATA[...]]>
			self.advance(); // skip '!'

			if self.current_char() == '-' && self.peek_char(1) == '-' {
				// Comment: <!--...-->
				return self.skip_xml_comment();
			}

			if self.current_char() == '[' {
				// CDATA: <![CDATA[...]]>
				return self.parse_cdata();
			}

			// DOCTYPE or other declaration: <!DOCTYPE...>
			return self.skip_doctype();
		}

		if self.current_char() == '/' {
			// an unmatched closing tag; a matched one is consumed by its element
			let tag_name = self.parse_closing_tag();
			return error(&format!("Unmatched closing tag </{}>", tag_name));
		}

		let tag_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(e) => return error(&e),
		};

		let mut attributes = Vec::new();
		self.skip_whitespace_and_comments();
		while self.current_char() != '>' && self.current_char() != '/' && !self.end_of_input() {
			let Ok(attr_name) = self.parse_symbol() else { break };
			attributes.push(key_ops(attr_name, Op::Assign, self.parse_attribute_value()));
			self.skip_whitespace_and_comments();
		}

		if self.current_char() == '/' {
			self.advance(); // skip '/'
			self.skip_whitespace_and_comments();
			if self.current_char() == '>' {
				self.advance();
			}
			return xml_element(tag_name, curly_or_empty(attributes));
		}

		if self.current_char() == '>' {
			self.advance();
		}

		let mut body_items = attributes;
		while !self.end_of_input() {
			if self.current_char() == '<' && self.peek_char(1) == '/' {
				self.advance(); // skip '<'
				let closing_name = self.parse_closing_tag();
				if closing_name != tag_name {
					return error(&format!("Mismatched tags: <{}> closed with </{}>", tag_name, closing_name));
				}
				break;
			}
			if self.current_char() == '<' {
				let nested = self.parse_xml_tag();
				if nested != Empty {
					body_items.push(nested);
				}
				continue;
			}
			let text = self.parse_xml_text_content();
			if !text.is_empty() {
				body_items.push(Node::Text(text));
			}
		}

		let body = match <[Node; 1]>::try_from(body_items) {
			Ok([only]) => only,
			Err(items) => curly_or_empty(items),
		};
		xml_element(tag_name, body)
	}

	/// The value after an attribute name: `="quoted"`, `=unquoted`, or true for a bare boolean attribute
	fn parse_attribute_value(&mut self) -> Node {
		self.skip_whitespace_and_comments();
		if self.current_char() != '=' {
			return Node::True;
		}
		self.advance(); // skip '='
		self.skip_whitespace_and_comments();
		if self.current_char() == '"' || self.current_char() == '\'' {
			self.parse_string()
		} else {
			self.parse_symbol().map_or(Empty, Node::Text)
		}
	}

	/// `/name>` after a `<`: the closing tag's name
	fn parse_closing_tag(&mut self) -> String {
		self.advance(); // skip '/'
		let name = self.parse_symbol().unwrap_or_default();
		self.skip_until('>');
		self.advance(); // skip '>'
		name
	}
}

/// `tag: body`, how an XML element is written in warp
fn xml_element(tag_name: String, body: Node) -> Node {
	Node::Key(Box::new(Symbol(tag_name)), Op::Colon, Box::new(body))
}

fn curly_or_empty(items: Vec<Node>) -> Node {
	if items.is_empty() { Empty } else { Node::List(items, Bracket::Curly, Separator::None) }
}
