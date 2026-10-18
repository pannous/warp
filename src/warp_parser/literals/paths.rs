//! Paths and URLs: where a path literal starts and a URL ends
use super::*;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
/// What follows the scheme of a URL read as one text, whatever the scheme: `https://pannous.com`, `ssh://host` (card add-http)
const URL_MARK: &str = "://";

/// A text written as a path without quotes (`~/notes.txt`, `./a.wav`, `/tmp`): a call of a word with a method for the
/// class Path passes it as a Path (lib/shell.warp, `sort ./names.txt`; traits generics.rs path_arguments)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PathLiteral;

/// The node is a path written without quotes
pub fn is_path_literal(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => {
			matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<PathLiteral>().is_some())
				|| is_path_literal(node)
		}
		_ => false,
	}
}

/// The variables every assignment of which is a path written without quotes: `notes = ~/notes.txt`
pub fn path_variables(node: &Node) -> std::collections::HashSet<String> {
	let (mut paths, mut others) = (std::collections::HashSet::new(), std::collections::HashSet::new());
	node.visit(&mut |part| {
		if let Node::Key(name, Op::Assign | Op::Define, value) = part.drop_meta() {
			if let Node::Symbol(name) = name.drop_meta() {
				match is_path_literal(value) {
					true => paths.insert(name.clone()),
					false => others.insert(name.clone()),
				};
			}
		}
	});
	paths.difference(&others).cloned().collect()
}

/// A path written without quotes, or one of the `path_variables`: a Path where a method takes one
pub fn is_written_path(argument: &Node, path_variables: &std::collections::HashSet<String>) -> bool {
	is_path_literal(argument) || argument.symbol_name().is_some_and(|name| path_variables.contains(name))
}

impl WarpParser {
	/// A file path without quotes starts here, glued to what follows and not to an operand before it (card unix-words):
	/// `./mozart.mp3`, `../songs/a.wav`, `./` alone (`ls ./`; `xs ./ 2` and `xs./2` divide each element); `~/notes.txt`, and `~` alone (`cd ~`;
	/// `a ~ b` stays the comparison); `/etc/hosts` where no operand stands before it (`x = /tmp`, `[/a, /b]`) or after
	/// a word taking it (`ls /tmp`, `cp /a /b`); after a variable it divides (`width /height`)
	pub(crate) fn starts_path_literal(&self) -> bool {
		let glued_to_operand = is_operand_end(self.prev_char());
		!glued_to_operand
			&& match self.current_char() {
				'.' => {
					let dots = if self.peek_char(1) == '.' { 2 } else { 1 };
					let folder_alone = ends_path(self.peek_char(dots + 1)) && !self.operand_follows(dots + 1);
					self.peek_char(dots) == '/' && (!ends_path(self.peek_char(dots + 1)) || folder_alone)
				}
				'~' => self.peek_char(1) == '/' || !self.operand_follows(1),
				'/' => {
					starts_path_name(self.peek_char(1))
						&& self.operand_before().is_none_or(|word| self.takes_path(&word))
				}
				_ => false,
			}
	}

	/// The word before the blanks here when an operand ends there: Some("ls") in `ls /tmp`, Some("") after `)`, None
	/// after an operator, a separator or an opening bracket
	fn operand_before(&self) -> Option<String> {
		let end = self.chars[..self.pos].iter().rposition(|c| !matches!(c, ' ' | '\t'))?;
		if !is_operand_end(self.chars[end]) {
			return None;
		}
		let start = self.chars[..=end].iter().rposition(|c| !is_identifier_char(*c)).map_or(0, |at| at + 1);
		Some(self.chars[start..=end].iter().collect())
	}

	/// A word before ` /name` takes a path: the operand before is a path (`cp /a /b`), or the word is no variable but a
	/// function (the program's wherever it is defined, a variable holding one, a standard module's like ls, a built-in like
	/// print); after a variable or a name a definition binds as a parameter it divides: `width /height`, `area(scale, h)
	/// := scale /h`
	fn takes_path(&self, word: &str) -> bool {
		let after_path =
			self.path_end.is_some_and(|end| self.chars[end..self.pos].iter().all(|c| matches!(c, ' ' | '\t')));
		after_path
			|| (!self.variables.contains(word)
				&& !self.parameters_in_scope.iter().any(|parameter| parameter == word)
				&& (self.functions.contains(word)
					|| self.program_functions.contains(word)
					|| crate::lowering::negated_calls::is_builtin_callable(word)
					|| crate::modules::std_function_names().contains(word)))
	}

	/// Check if character terminates a URL
	pub(crate) fn is_url_terminator(&self, ch: char) -> bool {
		ch == '\0'
			|| ch == ' '
			|| ch == '\t'
			|| ch == '\n'
			|| ch == '\r'
			|| ch == ';'
			|| ch == ')'
			|| ch == ']'
			|| ch == '}'
			|| ch == '>'
			|| ch == '"'
			|| ch == '\''
			|| ch == ','
			|| ch == '«'
	}

	/// The path literal at the cursor, up to whitespace, a closing bracket or a separator
	pub(crate) fn parse_path_literal(&mut self) -> Node {
		let path: String =
			(0..).map(|offset| self.peek_char(offset)).take_while(|&c| !super::lookahead::ends_path(c)).collect();
		self.advance_by(path.chars().count());
		self.path_end = Some(self.pos);
		Node::Text(path).with_meta_data(PathLiteral)
	}

	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	pub(crate) fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::words::VERSION_KEYWORD || self.options.data_mode {
			return None;
		}
		let blanks = self.chars[self.pos..].iter().take_while(|c| **c == ' ').count();
		let length = crate::versions::operand_len(&self.chars[self.pos + blanks..]);
		if blanks == 0 || length == 0 {
			return None;
		}
		let operand: String = self.chars[self.pos + blanks..self.pos + blanks + length].iter().collect();
		self.advance_by(blanks + length);
		Some(Node::List(vec![Node::Symbol(symbol.to_string()), Node::Symbol(operand)], Bracket::None, Separator::Space))
	}

	/// `http://…`, `file://…`, `ssh://…`: the scheme of a URL, the rest of which reads as one text
	pub(crate) fn url_follows(&self) -> bool {
		URL_MARK.chars().enumerate().all(|(offset, mark)| self.peek_char(offset) == mark)
	}

	pub(crate) fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}
}

/// An operand ends with this character: a name, a number, a closing bracket or a quote
fn is_operand_end(c: char) -> bool {
	is_identifier_char(c) || matches!(c, ')' | ']' | '}' | '"' | '\'')
}

/// What may follow the `/` of an absolute path: a name, a hidden name or a parent `..`; not `//` or `/*` (comments)
fn starts_path_name(c: char) -> bool {
	c.is_alphabetic() || matches!(c, '_' | '.')
}
