//! Type expressions: names of types, list types `[Int]`, array type suffixes `Int[]`
use super::*;
use crate::words::LIST_WORD;

/// A word that can take type arguments: a capitalised type, a built-in type word, a plural type word, `list`, `map` …
pub(crate) fn names_a_type(word: &str) -> bool {
	word.chars().next().is_some_and(char::is_uppercase)
		|| crate::analyzer::type_word_kind(&word.to_lowercase()).is_some()
		|| crate::analyzer::plural_element_type(word).is_some()
		|| GENERIC_TYPE_HEADS.contains(&word)
}

/// The type `name<arguments>` written in words: `list<list<int>>` is `list of list of int`
pub(crate) fn type_application_name(name: &str, arguments: &str) -> String {
	let arguments = arguments.replace('<', " of ").replace('>', "");
	format!("{} of {}", noted_list_type_head(name), arguments.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// The head of a list type: `Vec`, `vector`, `Array`… are list
pub(crate) fn list_type_head(name: &str) -> &str {
	if FOREIGN_LIST_HEADS.contains(&name) {
		LIST_WORD
	} else {
		name
	}
}

/// list_type_head, with a note naming warp's word for a foreign one
pub(crate) fn noted_list_type_head(name: &str) -> &str {
	if FOREIGN_LIST_HEADS.contains(&name) {
		crate::errors::diagnostic::note_alias(name, LIST_WORD);
	}
	list_type_head(name)
}

impl WarpParser {
	/// The Java/C array type `int[]` is the list type `[int]`
	pub(crate) fn try_parse_array_type_suffix(&mut self, lhs: &Node) -> Option<Node> {
		let Node::Symbol(word) = lhs.drop_meta() else {
			return None;
		};
		if !is_identifier_char(self.prev_char()) || self.peek_char(1) != ']' || !names_a_type(word) {
			return None;
		}
		self.advance_by(2);
		Some(Node::List(vec![lhs.clone()], Bracket::Square, Separator::Space))
	}

	/// `full name` of a declared `class full name {…}` is the one name `full-name`, any other word itself
	pub(crate) fn joined_type_name(&mut self, word: String) -> String {
		let found = self.multi_word_types.iter().find_map(|words| {
			(words[0] == word)
				.then(|| self.words_ahead_length(&words[1..]))
				.flatten()
				.map(|length| (words.join("-"), length))
		});
		match found {
			Some((name, length)) => {
				self.advance_by(length);
				name
			}
			None => word,
		}
	}

	/// The length of the blanks and `words` right ahead, each a whole word: ` name` of `full name`
	fn words_ahead_length(&self, words: &[String]) -> Option<usize> {
		let mut offset = 0;
		for word in words {
			let blanks = (offset..).take_while(|&at| matches!(self.peek_char(at), ' ' | '\t')).count() - offset;
			if blanks == 0 {
				return None;
			}
			offset += blanks;
			if !word.chars().enumerate().all(|(index, ch)| self.peek_char(offset + index) == ch) {
				return None;
			}
			offset += word.chars().count();
			if is_identifier_char(self.peek_char(offset)) || self.peek_char(offset) == '-' {
				return None;
			}
		}
		Some(offset)
	}

	/// `tuple<s64, list<node>>` → `tuple` followed by the angle-bracketed argument list
	/// Length of the `<int>`, `<list<int>>`, `<text, int>`, `<int(int)>` directly at the cursor when everything inside is a type word;
	/// `a<b`, `a<<b` and `a<b>c` are not type applications
	pub(crate) fn type_application_length(&self) -> Option<usize> {
		let mut depth = 0;
		let mut word = String::new();
		let mut words = Vec::new();
		for offset in 0.. {
			let ch = self.peek_char(offset);
			match ch {
				'<' => depth += 1,
				'>' => depth -= 1,
				ch if ch.is_alphanumeric() || ch == '_' || ch == '?' => {
					word.push(ch);
					continue;
				}
				',' | ' ' | '(' | ')' => {} // C++'s function type `function<int(int)>`
				_ => return None,
			}
			if !word.is_empty() {
				words.push(std::mem::take(&mut word));
			}
			if depth == 0 {
				let is_type_argument = |word: &String| {
					let bare = word.trim_end_matches('?');
					crate::analyzer::type_word_kind(bare).is_some()
						|| crate::analyzer::plural_element_type(bare).is_some()
						|| GENERIC_TYPE_HEADS.contains(&bare)
						|| bare.chars().next().is_some_and(char::is_uppercase)
				};
				return (offset > 1 && words.iter().all(is_type_argument)).then_some(offset + 1);
			}
		}
		None
	}

	pub(crate) fn parse_type_application(&mut self, type_name: String) -> Node {
		self.advance(); // skip '<'
		let arguments = self.parse_list_with_separators(Some('>'), Bracket::Less);
		Node::List(vec![Symbol(type_name), arguments], Bracket::None, Separator::None)
	}
}

/// `left: ref Node?` (WebAssembly's reference types): the field's type is the word after `ref`; every class value is a
/// reference already
pub(crate) fn without_ref_words(items: Vec<Node>) -> Vec<Node> {
	let mut fields: Vec<Node> = Vec::with_capacity(items.len());
	for item in items {
		let after_ref = matches!(fields.last().map(Node::drop_meta), Some(Node::Key(_, Op::Colon, value)) if value.is_symbol(REF_TYPE_WORD));
		match (after_ref, item.drop_meta()) {
			(true, Node::Symbol(_)) => {
				let Some(Node::Key(name, op, _)) = fields.pop().map(|field| field.drop_meta().clone()) else {
					unreachable!("checked")
				};
				fields.push(Node::Key(name, op, Box::new(item)));
			}
			_ => fields.push(item),
		}
	}
	fields
}

/// `items: list of text`: the words `of text` after a field's type continue it, the one type `list of text` (as
/// `list<text>` is read); `list of list of int` too
pub(crate) fn with_type_phrases(items: Vec<Node>) -> Vec<Node> {
	let mut fields: Vec<Node> = Vec::with_capacity(items.len());
	let mut items = items.into_iter().peekable();
	while let Some(item) = items.next() {
		let is_of = item.is_symbol(OF_WORD);
		let field_type = match fields.last().map(Node::drop_meta) {
			Some(Node::Key(_, Op::Colon, value)) => match value.drop_meta() {
				Node::Symbol(type_name) => Some(type_name.clone()),
				_ => None,
			},
			_ => None,
		};
		let element = items.peek().and_then(|next| match next.drop_meta() {
			Node::Symbol(element) => Some(element.clone()),
			_ => None,
		});
		match (is_of, field_type, element) {
			(true, Some(type_name), Some(element)) => {
				items.next();
				let Some(Node::Key(name, op, _)) = fields.pop().map(|field| field.drop_meta().clone()) else {
					unreachable!("checked")
				};
				fields.push(Node::Key(
					name,
					op,
					Box::new(Node::Symbol(format!(
						"{} {} {element}",
						super::noted_list_type_head(&type_name),
						OF_WORD
					))),
				));
			}
			_ => fields.push(item),
		}
	}
	fields
}
