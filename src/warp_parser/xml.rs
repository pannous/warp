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

		// Check for closing tag </tag>
		if self.current_char() == '/' {
			// This is a closing tag, should be handled by parent
			// Return error for unmatched closing tag
			self.advance(); // skip '/'
			let tag_name = self.parse_symbol().unwrap_or_default();
			self.skip_until('>');
			self.advance(); // skip '>'
			return error(&format!("Unmatched closing tag </{}>", tag_name));
		}

		let tag_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(e) => return error(&e),
		};

		let mut attributes = Vec::new();
		self.skip_whitespace_and_comments();

		while self.current_char() != '>' && self.current_char() != '/' && !self.end_of_input() {
			let attr_name = match self.parse_symbol() {
				Ok(name) => name,
				Err(_) => break,
			};

			self.skip_whitespace_and_comments();

			// Check for = sign
			if self.current_char() == '=' {
				self.advance(); // skip '='
				self.skip_whitespace_and_comments();

				// Parse attribute value (must be quoted)
				let attr_value = if self.current_char() == '"' || self.current_char() == '\'' {
					self.parse_string()
				} else {
					// Try to parse unquoted value
					match self.parse_symbol() {
						Ok(val) => Node::Text(val),
						Err(_) => Empty,
					}
				};

				// Store attribute as dotted key
				attributes.push(key_ops(attr_name, Op::Assign, attr_value));
				// attributes.push(Node::Key(Box::new(Symbol(format!(".{}", attr_name))), Op::Assign, Box::new(attr_value)));
			} else {
				// Boolean attribute (no value)
				attributes.push(Node::Key(
					Box::new(Symbol(format!(".{}", attr_name))),
					Op::Assign,
					Box::new(Node::True),
				));
			}

			self.skip_whitespace_and_comments();
		}

		// Check for self-closing tag
		if self.current_char() == '/' {
			self.advance(); // skip '/'
			self.skip_whitespace_and_comments();
			if self.current_char() == '>' {
				self.advance(); // skip '>'
			}
			// Return self-closing tag with only attributes
			return if attributes.is_empty() {
				Node::Key(Box::new(Symbol(tag_name)), Op::Colon, Box::new(Empty))
			} else {
				Node::Key(
					Box::new(Symbol(tag_name)),
					Op::Colon,
					Box::new(Node::List(attributes, Bracket::Curly, Separator::None)),
				)
			};
		}

		// Skip closing '>' of opening tag
		if self.current_char() == '>' {
			self.advance();
		}

		// Parse content until closing tag
		let mut content_items = Vec::new();

		while !self.end_of_input() {
			// Check for closing tag (before skipping whitespace)
			if self.current_char() == '<' && self.peek_char(1) == '/' {
				self.advance(); // skip '<'
				self.advance(); // skip '/'
				let closing_name = self.parse_symbol().unwrap_or_default();
				self.skip_until('>');
				self.advance(); // skip '>'

				if closing_name != tag_name {
					return error(&format!(
						"Mismatched tags: <{}> closed with </{}>",
						tag_name, closing_name
					));
				}
				break; // Successfully closed
			}

			// Check for nested tag
			if self.current_char() == '<' && self.peek_char(1) != '/' {
				let nested = self.parse_xml_tag();
				if nested != Empty {
					content_items.push(nested);
				}
				continue;
			}

			// Parse text content until next tag
			let text = self.parse_xml_text_content();
			if !text.is_empty() {
				content_items.push(Node::Text(text));
			}
		}

		// Combine attributes and content
		let mut body_items = attributes;
		body_items.extend(content_items);

		if body_items.is_empty() {
			Node::Key(
				Box::new(Symbol(tag_name.clone())),
				Op::Colon,
				Box::new(Empty),
			)
		} else if body_items.len() == 1 {
			Node::Key(
				Box::new(Symbol(tag_name)),
				Op::Colon,
				Box::new(body_items.into_iter().next().unwrap()),
			)
		} else {
			Node::Key(
				Box::new(Symbol(tag_name)),
				Op::Colon,
				Box::new(Node::List(body_items, Bracket::Curly, Separator::None)),
			)
		}
	}
}
