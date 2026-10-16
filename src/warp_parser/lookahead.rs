//! Looking ahead: what can start an atom, operators ahead

use super::*;
use crate::operators::AliasPosition;

/// `∜x`, read as `√√x`
pub(super) const FOURTH_ROOT: char = '∜';
/// The prefix operators written as words: `sqrt x`, `cbrt x`, `abs x`, `not x` (`norm x` is the prelude's word: the
/// absolute value of a number, the Euclidean length of a list, decision S3)
pub const PREFIX_OPERATOR_WORDS: [(&str, Op); 4] =
	[("sqrt", Op::Sqrt), ("cbrt", Op::Cbrt), ("abs", Op::Abs), ("not", Op::Not)];
/// operators::OPERATOR_SPELLINGS, the longest first: `<=` before `<`, `is less than` before `is`
static LONGEST_SPELLINGS_FIRST: std::sync::LazyLock<Vec<(&str, Op)>> = std::sync::LazyLock::new(|| {
	let mut spellings: Vec<_> = crate::operators::all_spellings().collect();
	spellings.sort_by_key(|(spelling, _)| std::cmp::Reverse(spelling.chars().count()));
	spellings
});
/// Infix operators that may follow a suffix: `10% + 1`, `x abs * 2`
const SUFFIX_FOLLOWERS: [char; 6] = ['+', '-', '*', '/', '<', '>'];
/// Operator words that stay operators before a colon: `if c then: a else: b`, `defp f(x), do: x`
const BLOCK_COLON_WORDS: [&str; 3] = ["then", "else", "do"];

impl WarpParser {
	/// Check if current character can start an atom (for implicit application)
	pub(super) fn can_start_atom(&self) -> bool {
		let ch = self.current_char();
		ch.is_alphanumeric()
			|| ch == '_'
			|| (ch == '$' && self.is_identifier_start(1)) // a bound name `$h`
			|| ch == '"'
			|| ch == '\''
			|| ch == '('
			|| ch == '['
			|| ch == '{'
			|| self.number_starts_at(0)
			|| self.starts_function_reference()
			|| self.starts_path_literal()
			|| self.url_follows()
	}

	/// A digit, or a leading-dot decimal like `.5`
	pub(super) fn number_starts_at(&self, offset: usize) -> bool {
		let ch = self.peek_char(offset);
		ch.is_ascii_digit() || (ch == '.' && self.peek_char(offset + 1).is_ascii_digit())
	}

	/// An operand may start after `offset` characters and following blanks: not a closing bracket, separator or the end
	pub(super) fn operand_follows(&self, offset: usize) -> bool {
		let mut position = offset;
		while matches!(self.peek_char(position), ' ' | '\t') {
			position += 1;
		}
		!matches!(self.peek_char(position), '\0' | '\n' | '\r' | '>' | ')' | ']' | '}' | ',' | ';' | '=')
	}

	/// `{from:1 to:2}`: an operator word directly before a key colon names the key; `else:` and `then:` open blocks
	fn word_names_key(&self) -> bool {
		let word: String = (0..).map(|offset| self.peek_char(offset)).take_while(|&c| is_identifier_char(c)).collect();
		let after = word.chars().count();
		!word.is_empty()
			&& !BLOCK_COLON_WORDS.contains(&word.as_str())
			&& self.peek_char(after) == ':'
			&& !matches!(self.peek_char(after + 1), ':' | '=')
	}

	/// Peek ahead for an infix operator, returns (Op, chars_to_consume) if found: the longest spelling of
	/// operators::OPERATOR_SPELLINGS that fits here and its context allows
	pub(super) fn peek_operator(&self) -> Option<(Op, usize)> {
		if (self.current_char(), self.peek_char(1)) == ('?', ':') {
			return None; // the elvis `?:` is no ternary, `try_parse_elvis` takes it
		}
		// `fetch ://host/path`: the colon starts a URL (card pannous-com), no key
		if self.word_names_key() || self.starts_path_literal() || self.url_follows() {
			return None;
		}
		if let Some(length) = self.down_to_length() {
			return Some((Op::To, length));
		}
		if let Some(found) = self.alias_here(AliasPosition::Infix) {
			return Some(found);
		}
		LONGEST_SPELLINGS_FIRST
			.iter()
			.filter(|(spelling, _)| self.spelled_here(spelling))
			.find_map(|(spelling, op)| self.operator_in_context(spelling, *op))
			.map(|(op, spelling)| (op, spelling.chars().count()))
	}

	/// An alias the program declares (`operator ⊕ := +`), or for a prefix operator one of lib/math.warp (`prefix operator
	/// √ := sqrt`); lib/math.warp's infix aliases are spellings of the table (operators::all_spellings)
	fn alias_here(&self, position: AliasPosition) -> Option<(Op, usize)> {
		let math = crate::operators::math_aliases().iter().filter(|_| position == AliasPosition::Prefix);
		self.source_aliases
			.iter()
			.chain(math)
			.filter(|alias| alias.position == position && self.spelled_here(&alias.spelling))
			.max_by_key(|alias| alias.spelling.chars().count())
			.map(|alias| (alias.op, alias.spelling.chars().count()))
	}

	/// The spelling starts here; a word ends here too: `and` but not `android`, `plus a` but not the call `plus(a, b)`
	fn spelled_here(&self, spelling: &str) -> bool {
		let length = spelling.chars().count();
		let after = self.peek_char(length);
		spelling.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !(spelling.ends_with(char::is_alphabetic) && (is_identifier_char(after) || after == '('))
	}

	/// The operator a fitting spelling is here, None where the context reads it otherwise
	fn operator_in_context<'a>(&self, spelling: &'a str, op: Op) -> Option<(Op, &'a str)> {
		let (after, prev) = (self.peek_char(spelling.chars().count()), self.prev_char());
		let allowed = match spelling {
			"else" => !self.else_arm_key(),
			"if" => !self.condition_has_branch(2),
			"?." => after.is_alphabetic() || after == '_', // `x?.name`; `x ?.5 : 1` is a ternary
			// R's `x <- 3` assigns with a note, the cramped `x<-3` is an error (P145, left_arrow_assignment)
			"<-" => after.is_whitespace() || !prev.is_whitespace(),
			"<<" | "<" | ">" => !self.options.wit_mode, // angle brackets only delimit type arguments
			// `list<list<int>>` closes two generics, a shift has an operand behind it
			">>" => !self.options.wit_mode && self.operand_follows(2),
			"+-" => after.is_whitespace() && prev.is_whitespace(),
			"/" => after != '/',                      // Don't treat // as division - it's a comment
			"//" | "//=" => self.at_floor_division(), // `x // note` stays a comment
			// `div{…}` and `div {…}` are the html tag, and data never computes
			"div" => !self.options.data_mode && !self.word_opens_block("div"),
			// `xs contains x` in code only, data keeps the words as they are
			"contains" | "has" | "includes" => !self.options.data_mode && self.operand_after(spelling.len()),
			// `a .5` is a list of two values, `a.5` a member access
			"." => !(prev.is_whitespace() && self.number_starts_at(0)),
			"&" => !(self.starts_function_reference() && prev.is_whitespace()), // `map &square xs`
			_ => true,
		};
		// `solid #8884`: the color is the next word of the value; `#main .x`: the descendant selector's next part
		let selector_part =
			self.in_style_sheet && matches!(spelling, "." | "#") && prev.is_whitespace() && self.is_identifier_start(1);
		let style_word =
			spelling.chars().count() == 1 && (self.css_color_length().is_some() || self.at_pseudo_class_selector());
		let op = match spelling {
			"=" if self.equals_compares => Op::Eq,
			"until" if !self.in_for_header => Op::While, // `i++ until done`, Kotlin's `for i in 0 until n` ranges
			_ => op,
		};
		// a word operator stands before its operand, `use same;` names the module same; `then`, `else`, `do` may end a line
		let operand_after_word = !spelling.ends_with(char::is_alphabetic)
			|| matches!(op, Op::Then | Op::Else | Op::Do)
			|| self.operand_follows(spelling.chars().count());
		(allowed && operand_after_word && !selector_part && !style_word).then_some((op, spelling))
	}

	/// At an opening bracket: a comma separates the group's own items (`[a -1, 2]`), not one of a nested group or text
	pub(super) fn group_holds_comma(&self) -> bool {
		let (mut depth, mut quote) = (0i32, None);
		for offset in 1.. {
			let ch = self.peek_char(offset);
			match (quote, ch) {
				(_, '\0') => return false,
				(Some(open), _) if ch == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(ch),
				(None, '(' | '[' | '{') => depth += 1,
				(None, ')' | ']' | '}') if depth == 0 => return false,
				(None, ')' | ']' | '}') => depth -= 1,
				(None, ',') if depth == 0 => return true,
				_ => {}
			}
		}
		false
	}

	/// `and print "x"`: an `and` followed by a statement, a word with an argument after it
	pub(super) fn and_starts_statement(&self) -> bool {
		if self.options.data_mode || !self.matches_keyword(AND_KEYWORD) {
			return false;
		}
		let blanks = |from: usize| (from..).take_while(|&at| matches!(self.peek_char(at), ' ' | '\t')).count();
		let word_start = AND_KEYWORD.len() + blanks(AND_KEYWORD.len());
		let word = self.word_at(word_start);
		if word.is_empty() || joins_operands(&word) || !self.peek_char(word_start).is_alphabetic() {
			return false;
		}
		let word_end = word_start + word.chars().count();
		let argument = word_end + blanks(word_end);
		if argument == word_end {
			return false; // `and f(3)`, `and x`: an operand
		}
		let next = self.peek_char(argument);
		let argument_word = self.word_at(argument);
		(next.is_alphanumeric() || matches!(next, '"' | '\'' | '[' | '{')) && !joins_operands(&argument_word)
	}

	/// Peek for prefix operators (unary operators that bind to right operand)
	pub(super) fn peek_prefix_operator(&self) -> Option<(Op, usize)> {
		// `{abs:5}`: the word names a key, as an infix operator word does
		if self.word_names_key() {
			return None;
		}
		if self.matches_keyword("while") {
			return Some((Op::While, 5));
		}
		if let Some((word, op)) = PREFIX_OPERATOR_WORDS.into_iter().find(|(word, _)| self.matches_keyword(word)) {
			return (!self.defines_named(word.len())).then_some((op, word.len()));
		}
		if self.matches_keyword("if") {
			return Some((Op::If, 2));
		}
		if let Some(found) = self.alias_here(AliasPosition::Prefix) {
			return Some(found);
		}

		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let variable_follows = c3.is_alphabetic() || c3 == '_';
		match c1 {
			'-' if c2 == '>' => None, // Ruby's stabby lambda `->(x) { … }` is an atom
			'+' if c2 == '+' && variable_follows => Some((Op::Inc, 2)),
			'-' if c2 == '-' && variable_follows => Some((Op::Dec, 2)),
			// unary plus glued to its operand, `+5`, `+x`, `+(a)`: the operand itself; a spaced `+` stays the operator
			// word (`fold + xs`)
			'+' if c2.is_alphanumeric() || matches!(c2, '_' | '(' | '.') => Some((Op::Add, 1)),
			'-' => Some((Op::Neg, 1)),
			dash if glyph_operator(dash) == Some(Op::Sub) => Some((Op::Neg, 1)),
			'!' | '¬' => Some((Op::Not, 1)),
			FOURTH_ROOT => Some((Op::Sqrt, 1)), // ∜x is √√x (expressions.rs)
			'‖' => Some((Op::Abs, 1)),
			'#' => self.css_color_length().is_none().then_some((Op::Hash, 1)), // prefix # means count/length
			// `> 100 => "big"` (C#'s relational pattern): a comparison without its left side, a match arm compares the
			// subject; a glued `<tag` stays a bracket
			'>' | '<' if !self.options.xml_mode && !self.options.data_mode => {
				let (op, length) = match (c1, c2) {
					('>', '=') => (Op::Ge, 2),
					('<', '=') => (Op::Le, 2),
					('>', _) => (Op::Gt, 1),
					_ => (Op::Lt, 1),
				};
				matches!(self.peek_char(length), ' ' | '\t').then_some((op, length))
			}
			_ => None,
		}
	}

	/// Peek for suffix operators (unary operators that bind to left operand)
	pub(super) fn peek_suffix_operator(&self) -> Option<(Op, usize)> {
		match (self.current_char(), self.peek_char(1)) {
			('+', '+') => Some((Op::Inc, 2)),
			('-', '-') => Some((Op::Dec, 2)),
			// `width: 50% height: 10px`: in a style sheet a % glued to a number is a percentage (card web-styles-limits)
			('%', _) if self.in_style_sheet && self.prev_char().is_ascii_digit() => Some((Op::Mod, 1)),
			// `10%`, `10% + x`: a percent, no remainder, when no operand follows
			('%', next) if next != '=' && self.expression_ends_after(1) => Some((Op::Mod, 1)),
			// `ℯ⌟`: the natural log when no base follows
			('⌟', _) if self.expression_ends_after(1) => Some((Op::LogOf, 1)),
			_ => None,
		}
	}

	/// After the suffix at offset 0 (`%`, `abs`) of `offset` characters: the end of the expression, or an infix operator
	/// standing apart (`10% + 1`, never the `-3` of `7 % -3`)
	pub(super) fn expression_ends_after(&self, offset: usize) -> bool {
		let mut position = offset;
		while matches!(self.peek_char(position), ' ' | '\t') {
			position += 1;
		}
		let infix_apart = position > offset
			&& SUFFIX_FOLLOWERS.contains(&self.peek_char(position))
			&& matches!(self.peek_char(position + 1), ' ' | '\t');
		!self.operand_follows(offset) || infix_apart
	}

	/// `$main`, `$ii_i`: names keep their sigil (WAT identifiers, DOM selectors)
	pub(super) fn parse_dollar_name(&mut self) -> Node {
		self.symbol_after(1, |name| Symbol(format!("${name}")))
	}

	pub(super) fn unwrap_single(group: Node) -> Node {
		match group {
			Node::List(mut items, _, _) if items.len() == 1 => items.remove(0),
			other => other,
		}
	}

	/// `?` directly after a type name, then `=` (not `==`) or a closing bracket or separator
	pub(super) fn ends_optional_type(&self, next: char, after: char) -> bool {
		matches!(next, ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0') || (next == '=' && after != '=')
	}

	/// In a type's fields, blanks and then the next field `right: …` after `width` characters: `left: Node? right: Node?` on
	/// one line is two optional fields, no ternary
	pub(super) fn next_field_follows(&self, width: usize) -> bool {
		let blanks = (width..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let name = self.word_at(width + blanks).chars().count();
		let colon = width + blanks + name;
		self.type_fields.is_some()
			&& blanks > 0
			&& name > 0
			&& self.peek_char(colon) == ':'
			&& !matches!(self.peek_char(colon + 1), '=' | ':')
	}

	/// Do blanks and then `bracket` follow (`c {`, not the glued `c{`)
	pub(super) fn blanks_then(&self, bracket: char) -> bool {
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == bracket
	}

	/// Do blanks and then a block of fields follow the cursor: `{}` or `{ name: …`, never a statement block like `{ out += x }`
	/// `assignments`: C#'s object initializer `Point { X = 3 }` of a declared class sets its fields with `=` too
	pub(super) fn block_after_blanks(&self, assignments: bool) -> bool {
		let blanks_from = |start: usize| (start..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let blanks = blanks_from(0);
		if blanks == 0 || self.peek_char(blanks) != '{' {
			return false;
		}
		// the block's first entry may start on the next line (`Person {⏎ name: "Alice" …}`)
		let first = blanks + 1 + (blanks + 1..).take_while(|at| self.peek_char(*at).is_whitespace()).count();
		let name =
			(first..).take_while(|at| self.peek_char(*at).is_alphanumeric() || self.peek_char(*at) == '_').count();
		let after_name = first + name + blanks_from(first + name);
		let assigns = assignments && self.peek_char(after_name) == '=' && self.peek_char(after_name + 1) != '=';
		self.peek_char(first) == '}'
			|| (name > 0 && ((self.peek_char(after_name) == ':' && self.peek_char(after_name + 1) != '=') || assigns))
	}

	/// Do blanks and then a closing bracket, `,`, `;`, an assignment `=`, a comment or the line end follow `offset`:
	/// nothing a ternary could be followed by
	pub(super) fn closes_after_blanks(&self, offset: usize) -> bool {
		let blanks = (offset..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let at = offset + blanks;
		let comment_starts = self.peek_char(at) == '/' && matches!(self.peek_char(at + 1), '/' | '*');
		blanks > 0 && (comment_starts || self.ends_optional_type(self.peek_char(at), self.peek_char(at + 1)))
	}
}

/// What ends a path literal: whitespace, a closing bracket or a separator
pub(crate) fn ends_path(c: char) -> bool {
	c.is_whitespace() || matches!(c, '\0' | ']' | ')' | '}' | ',' | ';')
}

pub(crate) fn starts_code(first: &Node) -> bool {
	match first.drop_meta() {
		Node::Symbol(word) => {
			crate::operators::is_function_keyword(word)
				|| super::ACCESSOR_WORDS.contains(&word.as_str())
				|| super::MEMBER_MODIFIERS.contains(&word.as_str())
		}
		// `init(n) {…}`, JavaScript's `sum() {…}` and `constructor(x, y) {…}`
		Node::List(call, Bracket::Round, _) => matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(_))),
		_ => false,
	}
}
