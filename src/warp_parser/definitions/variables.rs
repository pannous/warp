//! Variable declarations: global, nonlocal, typed globals, modifiers

use super::*;
use crate::words::GLOBAL_WORD;

/// `global float y = 2` as `global y: float = 2` (card w0-globals: the type word was dropped, so y held the int 2);
/// `global int k` without a value stays the declaration of k
pub(crate) fn typed_global(declaration: Node, type_word: Option<String>) -> Node {
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
		let global =
			|declaration: Node| Node::Key(Box::new(Symbol(GLOBAL_WORD.to_string())), Op::Colon, Box::new(declaration));
		// `global a, b` (Python): one declaration per name
		let mut names = vec![declaration];
		while keyword == GLOBAL_WORD
			&& matches!(names.last().map(Node::drop_meta), Some(Symbol(_)))
			&& self.current_char() == ','
		{
			self.advance();
			self.skip_spaces();
			names.push(self.parse_atom());
		}
		Some(match names.len() {
			1 => global(names.remove(0)),
			_ => Node::List(names.into_iter().map(global).collect(), Bracket::None, Separator::Semicolon),
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
		Node::Key(Box::new(Symbol(NONLOCAL_WORD.to_string())), Op::Colon, Box::new(declared))
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

	/// `l = "en"` or `k:text` (a typed parameter or declaration) makes l, k text variables; any `x = …` a variable
	pub(crate) fn note_variable(&mut self, lhs: &Node, op: Op, rhs: &Node) {
		let Node::Symbol(name) = lhs.drop_meta() else { return };
		if matches!(op, Op::Assign | Op::Define) {
			self.variables.insert(name.clone());
		}
		let holds_text = match (op, rhs.drop_meta()) {
			(Op::Assign | Op::Define, Node::Text(_)) => true,
			(Op::Colon, Node::Symbol(type_word)) => crate::type_tests::is_text_type_word(type_word),
			_ => false,
		};
		if holds_text {
			self.text_variables.insert(name.clone());
		}
	}

	/// R's `<-` at the cursor (P145): `x <- 3` assigns with a note to write `x = 3`; the cramped `x<-3` is an error
	/// naming `x = 3` and `x < -3`
	pub(crate) fn left_arrow_assignment(&self, op: Op, target: &Node) -> Result<(), Node> {
		if op != Op::Assign || (self.current_char(), self.peek_char(1)) != ('<', '-') {
			return Ok(());
		}
		let ends_value = |ch: &char| ch.is_whitespace() || matches!(ch, ';' | '(' | ')' | ']' | '}');
		let rest: Vec<char> = self.chars[self.pos + 2..].iter().copied().skip_while(|ch| *ch == ' ').collect();
		let token: String = rest.iter().take_while(|ch| !ends_value(ch)).collect();
		let continues = rest.get(token.chars().count()).is_some_and(|ch| !matches!(ch, '\n' | ';' | ')' | ']' | '}'));
		let value = if continues { format!("{token} …") } else { token };
		let name = target.serialize();
		let assignment = format!("{name} = {value}");
		let is_spaced = self.peek_char(2).is_whitespace();
		let (written, readings, fallback) = match is_spaced {
			true => (format!("{name} <- {value}"), vec![reading("the assignment", &assignment)], Fallback::Warning),
			false => (
				format!("{name}<-{value}"),
				vec![reading("the assignment", &assignment), reading("the comparison", &format!("{name} < -{value}"))],
				Fallback::Error,
			),
		};
		let forms: Vec<String> = readings.iter().map(|reading| reading.explicit_form.clone()).collect();
		let question = format!("`{written}`: write {}", forms.join(" or "));
		let question =
			Ask::new(LEFT_ARROW_TOPIC, question, readings, fallback).written(&written).at(self.line_nr, self.column);
		ask(&question).map(|_| ())
	}

	/// Code stands before the cursor on its line (`x = 7 // note`, not a `// note` line of its own)
	/// Code that could be divided ends right before: a name, a number, `)` or `]` (`xs[1] // 2`); after a text, a comma
	/// or a brace `//` is plainly a comment
	pub(crate) fn follows_operand_on_its_line(&self) -> bool {
		let last = self.chars[..self.pos].iter().rev().take_while(|&&c| c != '\n').find(|c| !c.is_whitespace());
		last.is_some_and(|&c| c.is_alphanumeric() || matches!(c, '_' | ')' | ']'))
	}

	/// The `//` at the cursor is set apart from the code by a tab or two spaces: `x = 5  // 14`, a result written beside
	/// it (card hint-comment-floordiv), never a spaced-out division
	pub(crate) fn is_set_apart(&self) -> bool {
		let before = &self.chars[..self.pos];
		before.ends_with(&['\t']) || before.ends_with(&[' ', ' '])
	}

	/// `class circle{pi = 3}`, `class C{pi:int}`: a named number declared in a type body names the type's own field, which
	/// shadows nothing outside; the type's methods read the field
	pub(crate) fn names_field(&mut self, symbol: &str, constant: &Node) -> bool {
		// `{empty: yes}`, `class Cell{empty: bool}`: a key named like ø is a name (card object-empty-field)
		let is_word = symbol.chars().all(char::is_alphabetic);
		if matches!(constant, Node::Empty) && is_word && self.type_annotation_follows() && self.peek_char(1) != ':' {
			return true;
		}
		if !matches!(constant, Node::Number(_)) {
			return false;
		}
		let declares = self.assignment_follows() || self.type_annotation_follows();
		let Some(fields) = self.type_fields.as_mut() else {
			return false;
		};
		if declares {
			fields.insert(symbol.to_string());
		}
		fields.contains(symbol)
	}

	/// `:type` right after the name (`pi:int`, `pi: int`), not the definition `:=` nor a ternary's `c ? pi : 0`
	fn type_annotation_follows(&self) -> bool {
		self.current_char() == ':' && self.peek_char(1) != '='
	}

	/// `pi = 4` (P130, user: "loud error, if it was declared constant before which it should be"): a named number is a
	/// declared constant, an assignment to it is an error
	pub(crate) fn refuse_constant_assignment(&self, symbol: &str) -> Option<Node> {
		let column = self.column.saturating_sub(symbol.chars().count());
		let message = format!("{symbol} is a constant");
		self.assignment_follows().then(|| {
			Diagnostic { message, line: self.line_nr, column, ..Default::default() }.fix("another name").into_error()
		})
	}

	/// `= …` or `:= …` after blanks, not the comparison `==` nor the arrow `=>`
	fn assignment_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let defines = self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) == '=';
		let assigns = self.peek_char(blanks) == '=' && !matches!(self.peek_char(blanks + 1), '=' | '>'); // `None => 0` is a match arm
		defines || assigns
	}
}

/// `global a, b` as parsed: the declarations `global: a`, `global: b`
pub(crate) fn declarations_of_several_globals(item: &Node) -> Option<Vec<Node>> {
	let Node::List(declarations, Bracket::None, Separator::Semicolon) = item.drop_meta() else {
		return None;
	};
	let is_global = |declaration: &Node| matches!(declaration.drop_meta(), Node::Key(keyword, Op::Colon, _) if keyword.is_symbol(GLOBAL_WORD));
	declarations.iter().all(is_global).then(|| declarations.clone())
}
