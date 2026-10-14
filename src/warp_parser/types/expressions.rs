//! Type expressions: names of types, list types `[Int]`, array type suffixes `Int[]`
use super::*;

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
	format!(
		"{} of {}",
		noted_list_type_head(name),
		arguments.split_whitespace().collect::<Vec<_>>().join(" ")
	)
}

/// The head of a list type: `Vec`, `vector`, `Array`… are list
pub(crate) fn list_type_head(name: &str) -> &str {
	if FOREIGN_LIST_HEADS.contains(&name) {
		LIST_HEAD
	} else {
		name
	}
}

/// list_type_head, with a note naming warp's word for a foreign one
pub(crate) fn noted_list_type_head(name: &str) -> &str {
	if FOREIGN_LIST_HEADS.contains(&name) {
		crate::diagnostic::note_alias(name, LIST_HEAD);
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
}
