//! Foreign declaration modifiers: `public static fun f()`, `private x = 4`

use super::*;
use crate::warp_parser::atoms::note_alias_at;

/// Modifiers of other languages with no meaning in warp: skipped with a note (user decision P78)
const FOREIGN_MODIFIERS: [&str; 19] = [
	"public",
	"private",
	"protected",
	"internal",
	"static",
	"extern",
	"external",
	"C",
	"inline",
	"local",
	"virtual",
	"override",
	"abstract",
	"constexpr",
	"volatile",
	"thread_local",
	"synchronized",
	"transient",
	"native",
];
/// Modifiers with a warp meaning that change nothing before a function definition (a function is global and constant)
const DEFINITION_MODIFIERS: [&str; 7] = ["global", "export", "import", "const", "final", "mutable", "mut"];
const FOREIGN_MODIFIER_TOPIC: &str = "foreign-modifier";
/// Words before a class declaration that change nothing in warp: `data class` (a warp class compares by value already),
/// `open`, `abstract`, `sealed`, `final`, visibility
const CLASS_MODIFIERS: [&str; 8] = ["data", "open", "abstract", "sealed", "final", "public", "private", "internal"];
/// `new Point(1, 2)`: the construction `Point(1, 2)`
const NEW_WORD: &str = "new";
/// `constant x = 3` is `const x = 3`, with a note naming const (card constant-alias)
const CONSTANT_ALIAS: &str = "constant";
const CONST_WORD: &str = "const";
/// Ruby's field declarations `attr_accessor :x, :y`
const RUBY_FIELD_WORDS: [&str; 3] = ["attr_accessor", "attr_reader", "attr_writer"];

impl WarpParser {
	/// `public static fun f(){…}`, `private x = 4` (P78): a run of modifiers that ends before a function definition, or of
	/// modifiers without warp meaning before a name, is skipped, each meaningless one with a note; the statement after it
	pub(crate) fn skip_foreign_modifiers(&mut self, first: &str) -> Option<Node> {
		let is_modifier = |word: &str| FOREIGN_MODIFIERS.contains(&word) || DEFINITION_MODIFIERS.contains(&word);
		if !is_modifier(first) {
			return None;
		}
		let mut words = vec![first.to_string()];
		let mut offset = 0;
		let next = loop {
			let blanks = (offset..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
			if blanks == 0 {
				return None; // `public = 3`, `public(…)`: a name
			}
			let start = offset + blanks;
			let word: String =
				(start..).map(|at| self.peek_char(at)).take_while(|ch| is_identifier_char(*ch)).collect();
			offset = start + word.chars().count();
			if !is_modifier(&word) {
				break word;
			}
			words.push(word);
		};
		let before_definition = crate::operators::is_function_keyword(&next);
		let foreign_only = words.iter().all(|word| FOREIGN_MODIFIERS.contains(&word.as_str()));
		if next.is_empty() || !(before_definition || foreign_only) {
			return None;
		}
		let (line, column) = (self.line_nr, self.column.saturating_sub(first.chars().count()));
		let is_static = words.iter().any(|word| word == STATIC_KEYWORD);
		for word in words.iter().filter(|word| FOREIGN_MODIFIERS.contains(&word.as_str()) && *word != STATIC_KEYWORD) {
			let question = Ask::new(
				FOREIGN_MODIFIER_TOPIC,
				format!("{word} has no meaning in warp"),
				vec![reading("skip it", &next)],
				Fallback::Warning,
			)
			.written(word)
			.at(line, column);
			if let Err(error) = ask(&question) {
				return Some(error);
			}
		}
		self.advance_by(offset - next.chars().count());
		self.skip_spaces();
		let statement = self.parse_expr(0);
		Some(if is_static { statement.with_attribute(STATIC_KEYWORD, Node::True) } else { statement })
	}

	/// Another language's way of writing a warp form: `new Point(1, 2)`, `constant`, Ruby's `attr_accessor :x`,
	/// `operator +`, Kotlin's `enum class` and `data class`, `mixin Name {…}`
	pub(crate) fn foreign_form(&mut self, symbol: &str) -> Option<Node> {
		let start = self.start_of(symbol);
		// `new Point(1, 2)` (Java, JavaScript, C#) of a declared class: the construction `Point(1, 2)`
		if symbol == NEW_WORD && !self.options.data_mode && self.declared_type_after_blanks() {
			self.skip_blanks();
			let construction = self.parse_atom();
			let class = crate::node::leading_name(&construction);
			note_alias_at(start, &format!("{NEW_WORD} {class}"), &class);
			return Some(construction);
		}
		if symbol == CONSTANT_ALIAS && !self.options.data_mode && self.identifier_after_blanks() {
			note_alias_at(start, CONSTANT_ALIAS, CONST_WORD);
			return Some(Symbol(CONST_WORD.to_string()));
		}
		// Ruby's `attr_accessor :x, :y` in a class body: the fields x and y
		if RUBY_FIELD_WORDS.contains(&symbol) && self.type_fields.is_some() {
			let line: String = (0..)
				.map(|offset| self.peek_char(offset))
				.take_while(|ch| !matches!(ch, '\n' | '\0' | ';' | '}'))
				.collect();
			let names: Vec<String> =
				line.split(',').map(|name| name.trim().trim_start_matches(':').to_string()).collect();
			if names.iter().all(|name| !name.is_empty() && name.chars().all(is_identifier_char)) {
				self.advance_by(line.chars().count());
				note_alias_at(start, &format!("{symbol}{line}"), &names.join("; "));
				return Some(Node::List(names.into_iter().map(Symbol).collect(), Bracket::None, Separator::Semicolon));
			}
		}
		if symbol == OPERATOR_WORD && !self.options.data_mode && matches!(self.current_char(), ' ' | '\t') {
			self.skip_blanks();
			if let Some(head) = self.try_parse_operator_method_head() {
				note_alias_at(start, &format!("{OPERATOR_WORD} {}", head.first().name()), &head.first().name());
				return Some(head);
			}
		}
		// Kotlin's `enum class Color {…}`: the enum
		if symbol == ENUM_WORD && !self.options.wit_mode && self.class_keyword_after_blanks() == Some("class") {
			self.skip_blanks();
			self.advance_by("class".len());
			note_alias_at(start, &format!("{ENUM_WORD} class"), ENUM_WORD);
			return Some(Symbol(symbol.to_string()));
		}
		// `data class P(…)` (Kotlin), `open class`, `abstract class`: a modifier of a class declaration, the class itself
		if !self.options.wit_mode && CLASS_MODIFIERS.contains(&symbol) {
			if let Some(keyword) = self.class_keyword_after_blanks() {
				note_alias_at(start, &format!("{symbol} {keyword}"), keyword);
				self.skip_blanks();
				self.advance_by(keyword.len());
				return Some(self.parse_type_declaration());
			}
		}
		if !self.options.wit_mode && symbol == MIXIN_WORD && self.name_and_block_follow() {
			return Some(match self.parse_type_declaration() {
				Node::Type { name, body } => {
					Node::Type { name: Box::new(name.with_attribute(MIXIN_WORD, Node::True)), body }
				}
				other => other,
			});
		}
		None
	}
}
