//! Variable declarations: global, nonlocal, typed globals, modifiers

use super::*;

/// `global float y = 2` as `global y: float = 2` (card w0-globals: the type word was dropped, so y held the int 2);
/// `global int k` without a value stays the declaration of k
fn typed_global(declaration: Node, type_word: Option<String>) -> Node {
	match (declaration.drop_meta(), type_word) {
		(Node::Key(name, Op::Assign, value), Some(type_word)) if matches!(name.drop_meta(), Symbol(_)) => {
			let typed_name = Node::Key(name.clone(), Op::Colon, Box::new(Symbol(type_word)));
			Node::Key(Box::new(typed_name), Op::Assign, value.clone())
		}
		_ => declaration,
	}
}

impl WarpParser {
	/// `global [modifiers] name[=value]` and the same after `export`: a variable declared at module level.
	/// `export` without a name after it, and `export f:=it*2`, a function, stay ordinary code.
	pub(crate) fn try_parse_global_declaration(&mut self, keyword: &str) -> Option<Node> {
		let before_declaration = self.mark();
		self.skip_whitespace();
		let type_word = self.skip_declaration_modifiers();
		if keyword == "export" && !self.at_identifier_start() {
			self.rewind(before_declaration);
			return None;
		}
		let declaration = typed_global(self.parse_expr(0), type_word); // name, name=value or name:=value
		let defines_function =
			matches!(declaration.drop_meta(), Node::Key(name, Op::Define, _) if self.functions.contains(&name.name()));
		if keyword == "export" && defines_function {
			return Some(declaration);
		}
		let global = |declaration: Node| {
			Node::Key(
				Box::new(Symbol(crate::node::GLOBAL_DECLARATION.to_string())),
				Op::Colon,
				Box::new(declaration),
			)
		};
		// `global a, b` (Python): one declaration per name
		let mut names = vec![declaration];
		while keyword == crate::node::GLOBAL_DECLARATION
			&& matches!(names.last().map(Node::drop_meta), Some(Symbol(_)))
			&& self.current_char() == ','
		{
			self.advance();
			self.skip_spaces();
			names.push(self.parse_atom());
		}
		Some(match names.len() {
			1 => global(names.remove(0)),
			_ => Node::List(
				names.into_iter().map(global).collect(),
				Bracket::None,
				Separator::Semicolon,
			),
		})
	}

	/// A name follows after at least one blank: `nonlocal y`, not `nonlocal = 3` or `nonlocal(…)`
	pub(crate) fn name_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let next = self.peek_char(blanks);
		blanks > 0 && (next.is_alphabetic() || next == '_')
	}

	/// `nonlocal y` → `nonlocal:y`, like `global y`; `nonlocal a, b` declares each name
	pub(crate) fn parse_nonlocal_declaration(&mut self) -> Node {
		let mut names = vec![];
		loop {
			self.skip_spaces();
			match self.parse_symbol() {
				Ok(name) if !name.is_empty() => names.push(Symbol(name)),
				Ok(_) => break,
				Err(e) => return error(&e),
			}
			self.skip_spaces();
			if self.current_char() != ',' || !self.name_after_comma() {
				break;
			}
			self.advance();
		}
		let declared = match names.len() {
			1 => names.remove(0),
			_ => Node::List(names, Bracket::None, Separator::Colon),
		};
		Node::Key(
			Box::new(Symbol(NONLOCAL_WORD.to_string())),
			Op::Colon,
			Box::new(declared),
		)
	}

	/// `, b` after a declared name: another name, not the next item of a list
	pub(crate) fn name_after_comma(&self) -> bool {
		let blanks = (1..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let next = self.peek_char(1 + blanks);
		next.is_alphabetic() || next == '_'
	}

	/// Modifier and type words between the keyword and the name (`global const int k=7`): the global holds the value, the words are dropped
	/// Skips the modifiers before a global's name: the last type word among them, `float` of `global float y = 2`
	pub(crate) fn skip_declaration_modifiers(&mut self) -> Option<String> {
		let mut type_word = None;
		loop {
			let before_word = self.mark();
			let word = self.parse_symbol().unwrap_or_default();
			let is_modifier = DECLARATION_MODIFIERS.contains(&word.as_str())
				|| crate::words::CONSTANT_KEYWORDS.contains(&word.as_str())
				|| crate::analyzer::type_word_kind(&word).is_some();
			self.skip_spaces();
			if !is_modifier || !self.at_identifier_start() {
				self.rewind(before_word); // `global int` names the variable int
				return type_word;
			}
			if crate::analyzer::type_word_kind(&word).is_some() {
				type_word = Some(word);
			}
		}
	}
}
