use crate::diagnostic::{ask, reading, Ask, Diagnostic, Fallback};
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Symbol};
use crate::extensions::reals::{Exact, Rational, Real};
use crate::node::{error, key_ops, Bracket, Node, Separator};
use crate::operators::{glyph_operator, is_function_keyword, Op};
use crate::normalize::{hints as norm, set_hint_position, ListTypeStyle};
use crate::*;
use log::warn;
use std::fs::read_to_string;
use unicode_normalization::UnicodeNormalization;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
/// The unit words a for loop walks a text by, the item being `it` (wiki/string.md)
/// The schemes of a URL read as one text: `https://pannous.com`
const URL_SCHEMES: [&str; 7] = ["http", "https", "ftp", "file", "data", "ws", "wss"];
const UNIT_LOOP_WORDS: [&str; 4] = ["chars", "characters", "codepoints", "bytes"];
const BYTES_WORD: &str = "bytes";
const IT_WORD: &str = "it";
const MAX_INTEGER_EXPONENT: i64 = 4096;
/// Literal suffixes of C, Java and C#, a tight conversion: `0.1f`, `0.1d` (double) → float; `0.1l` (long double) → exact, the default anyway
const LITERAL_SUFFIXES: [(char, &str); 6] = [('f', "float"), ('F', "float"), ('d', "float"), ('D', "float"), ('l', "exact"), ('L', "exact")];
/// `0.1:float`, `1.5:int`: a number literal directly typed with one of these binds tightly, unlike the loose `as`
const LITERAL_NUMBER_TYPES: [&str; 11] = ["int", "i64", "integer", "exact", "real", "float", "fast", "f64", "double", "f32", "i32"];
/// Words that may precede the name of a global besides a type word (`int`, `long` … see `analyzer::type_word_kind`)
const ORDINAL_SUFFIXES: [&str; 4] = ["st", "nd", "rd", "th"];

/// Type names that take type arguments in angle brackets besides the plural and user types: `list<int>`, `map<text, int>`
const GENERIC_TYPE_HEADS: [&str; 7] = ["list", "array", "set", "map", "option", "result", "tuple"];

const DECLARATION_MODIFIERS: [&str; 3] = ["export", "mutable", "mut"];

const SIGNED_OPERAND_TOPIC: &str = "signed-operand";
/// `xs .+ 4`: an arithmetic operator behind a dot applies to each element (D3)
const ELEMENT_WISE_OPERATORS: [(char, Op); 4] = [('+', Op::Add), ('-', Op::Sub), ('*', Op::Mul), ('/', Op::Div)];

/// Control words behind a statement, each lowering to `if`/`while`, negated for `unless`/`until`
/// Words that declare a type from a field block: `struct point{x:int y:int}`, `class contact {name email?}`
const TYPE_DECLARATION_WORDS: [&str; 2] = ["class", "struct"];
/// `record point{x:int y:int}` declares a type like `struct`, but `record` is also an everyday variable name:
/// it is a declaration only when a name and a field block follow
const RECORD_WORD: &str = "record";
/// `1 upto 10` excludes 10 (wiki/range.md), asked about because readers expect either
const UPTO: &str = "upto";
/// Word spellings of `≈` (wiki/operator.md): equal within the relative `tolerance`
const SIMILARITY_WORDS: [&str; 2] = ["circa", "approximately"];
const EXCLUSIVE_DOTS: &str = "..";
/// Topic of the Ask about `for i in 0..n-1`, which Kotlin reads inclusive
const KOTLIN_RANGE: &str = "kotlin-range";
const STATEMENT_MODIFIERS: [(&str, Op, bool); 4] = [("if", Op::If, false), ("unless", Op::If, true), ("while", Op::While, false), ("until", Op::While, true)];
/// Words that test a value for being ø or falsy, sugar for `not x`; `failed` tests for an Error value (is_error)
const TEST_WORDS: [&str; 6] = ["empty", "missing", "absent", "unknown", "undefined", FAILED_WORD];
const FAILED_WORD: &str = "failed";
/// The runtime test of `x failed` (wasm_emitter/text_builtins.rs)
const IS_ERROR_CALL: &str = "is_error";
const EMPTY_WORD: &str = "empty";
/// Operators that may follow a suffix `!` directly: `x!+1`, `x!*2` (`!=` is the inequality)
const INFIX_AFTER_BANG: [char; 9] = ['+', '-', '*', '/', '%', '^', '<', '>', ')'];
/// Words that may follow a test word and so end the condition
const CONDITION_FOLLOWERS: [&str; 5] = ["then", "else", "and", "or", "do"];
/// Python's `elif`, Perl's and Ruby's `elsif`, PHP's `elseif`: all `else if`
const ELSE_IF_WORDS: [&str; 3] = ["elif", "elsif", "elseif"];
const RETURN_KEYWORD: &str = "return";
/// `await job` waits for a task; its operand binds like the operand of a unary minus (Op::Neg)
const AWAIT_KEYWORD: &str = "await";
const AWAIT_OPERAND_BP: u8 = 155;
const PRINT_WORD: &str = "print";
/// `print a  print b`: statements separated by spaces only (user decision 2026-10-03: a loud error)
const TWO_STATEMENTS_ON_ONE_LINE: &str = "two statements on one line? separate them with `;` or a newline";
const IN_KEYWORD: &str = "in";
/// Ruby/Lua blocks: `while c do … end`, `if c then … else … end`
const END_KEYWORD: &str = "end";
const ELSE_KEYWORD: &str = "else";
const END_BLOCK_OPENERS: [&str; 2] = ["do", "then"];
const AMBIGUOUS_END: &str = "ambiguous `end`: it closes either the `then` or the `do`; as in Ruby and Lua every `then … end` and `do … end` needs its own: write `while c do … if x then … end end` or `while c { … if x { … } }`";
/// Keywords a `[` after never indexes: `in [1, 2]` and `return [x]` take a list
const UNINDEXABLE_KEYWORDS: [&str; 6] = ["in", "return", "yield", "then", "else", "do"];

fn is_print_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Symbol(word) if word == PRINT_WORD)
}

/// `print` or the call `print(…)`: a print statement starts here
fn starts_print(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.first().is_some_and(is_print_word),
		other => is_print_word(other),
	}
}

/// Words separated by spaces form one expression: `upper "a"` is one argument of print
fn one_expression(words: &[Node]) -> Node {
	match words {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	}
}

/// The call print(a, b, …): its arguments are separated by commas, like Python's
fn print_call(arguments: impl IntoIterator<Item = Node>) -> Node {
	Node::List([Symbol(PRINT_WORD.to_string())].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None)
}

/// The arguments of `print(…)`: words separated by spaces (`print(upper "a")`) are one, else each item is one
fn print_arguments(arguments: Node) -> Vec<Node> {
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) if items.len() > 1 => vec![arguments],
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		_ => vec![arguments],
	}
}

/// The arguments of a parsed print list: the call print(a, b) has several, `print first xs` the one expression `first xs`
pub(crate) fn print_arguments_of(call: &[Node], bracket: &Bracket) -> Vec<Node> {
	match bracket {
		Bracket::Round => call[1..].to_vec(),
		_ => vec![one_expression(&call[1..])],
	}
}

/// A list as grouped by its separators, with the braceless print forms made explicit:
/// `print first xs` prints the one expression `first xs`, and in `print a, b` the comma binds looser than the space,
/// so the comma list [[print a], b] becomes the call print(a, b)
fn grouped_list(items: Vec<Node>, bracket: Bracket, separator: Separator) -> Node {
	let items = if separator == Separator::Space { with_arrow_bodies(items) } else { items };
	if bracket != Bracket::None || !matches!(separator, Separator::Space | Separator::Colon) {
		return Node::List(items, bracket, separator);
	}
	match (&separator, items[0].drop_meta()) {
		(Separator::Space, head) if starts_print(head) && items.iter().skip(2).any(starts_print) => {
			let second = items.iter().skip(2).find(|item| starts_print(item)).expect("guarded");
			Diagnostic::at(second, TWO_STATEMENTS_ON_ONE_LINE).into_error()
		}
		(Separator::Space, head) if is_print_word(head) && items.len() > 2 => {
			Node::List(vec![items[0].clone(), one_expression(&items[1..])], bracket, separator)
		}
		(Separator::Colon, Node::List(head, Bracket::None, Separator::Space)) if is_print_word(&head[0]) => {
			print_call(head[1..].iter().chain(&items[1..]).cloned())
		}
		_ => Node::List(items, bracket, separator),
	}
}

/// `(x => print x)`: `=>` binds looser than the space, so the words after a lambda's arrow are its body (`print x`), not
/// further items; a run of entries `1 => "a" 2 => "b"` stays a list of pairs
fn with_arrow_bodies(mut items: Vec<Node>) -> Vec<Node> {
	let is_arrow = |node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::FatArrow, _));
	let Some(arrow) = items.iter().position(is_arrow) else { return items };
	if arrow + 1 == items.len() || items[arrow + 1..].iter().any(is_arrow) {
		return items;
	}
	let rest = items.split_off(arrow + 1);
	let Node::Key(parameters, op, body) = items.pop().expect("the arrow").drop_meta().clone() else { unreachable!("an arrow") };
	let body = one_expression(&[vec![*body], rest].concat());
	items.push(Node::Key(parameters, op, Box::new(body)));
	items
}

/// `for chars in text {…it…}` (also characters, codepoints, bytes): the variable `it` over `chars(text)`, or over the
/// bytes `(0..text.bytes).map(i => byte_at(text, i))`; None when the body reads the word itself (a variable named chars)
fn unit_iteration(variable: &Node, iterable: &Node, body: &Node) -> Option<(Node, Node)> {
	let Node::Symbol(word) = variable.drop_meta() else { return None };
	let mentions = |name: &str| {
		let mut found = false;
		body.visit(&mut |part| found |= matches!(part, Node::Symbol(symbol) if symbol == name));
		found
	};
	if !UNIT_LOOP_WORDS.contains(&word.as_str()) || mentions(word) {
		return None;
	}
	let call = |name: &str, arguments: Vec<Node>| Node::List([vec![Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None);
	let walked = if word == BYTES_WORD {
		let offset = Symbol("byte·offset".to_string());
		let count = Node::Key(Box::new(iterable.clone()), Op::Dot, Box::new(Symbol(BYTES_WORD.to_string())));
		let offsets = Node::Key(Box::new(Node::Number(Number::Int(0))), Op::Range, Box::new(count));
		let byte = Node::Key(Box::new(offset.clone()), Op::FatArrow, Box::new(call("byte_at", vec![iterable.clone(), offset])));
		Node::Key(Box::new(Node::List(vec![offsets], Bracket::Round, Separator::None)), Op::Dot, Box::new(call("map", vec![byte])))
	} else {
		call("chars", vec![iterable.clone()])
	};
	Some((Symbol(IT_WORD.to_string()), walked))
}

/// The infix operators continue_expr reads apart from the operator table
enum SpecialInfix {
	/// `a mod b`, `a rem b`
	Keyword(Op),
	/// `7//2`, `7 div 2`, `x//=2`
	FloorDivision { compound: bool },
	/// `xs |> f(a)`
	Pipeline,
	/// `xs .+ 4`
	ElementWise(Op),
	/// `x in xs`
	Membership,
}

impl SpecialInfix {
	fn applied(self, lhs: Node, operand: Node) -> Node {
		match self {
			SpecialInfix::Keyword(op) => Node::Key(Box::new(lhs), op, Box::new(operand)),
			SpecialInfix::FloorDivision { compound } => {
				let quotient = floor_division(lhs.clone(), operand);
				if compound { Node::Key(Box::new(lhs), Op::Assign, Box::new(quotient)) } else { quotient }
			}
			SpecialInfix::Pipeline => piped(lhs, operand),
			SpecialInfix::ElementWise(op) => crate::analyzer::element_wise(lhs, op, operand),
			SpecialInfix::Membership => Node::List(vec![lhs, Symbol(IN_KEYWORD.to_string()), operand], Bracket::None, Separator::Space),
		}
	}
}

fn is_unindexable_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if UNINDEXABLE_KEYWORDS.contains(&word.as_str()))
}
/// The marker calls the parser leaves for `try X else Y` and `assert C else X`, lowered in `library_words`
pub const TRY_MARKER: &str = "try·else";
pub const ASSERT_MARKER: &str = "assert·else";
const GUARD_MARKERS: [(&str, &str); 2] = [("try", TRY_MARKER), ("assert", ASSERT_MARKER)];
/// `nand` and its glyph pair, both `not (a and b)`
const NAND_SPELLINGS: [&str; 2] = ["nand", "¬&"];
const TO_WORD: &str = "to";
/// `a[start:end]` calls the library word `slice`
const SLICE_WORD: &str = "slice";
const TIMES_WORD: &str = "times";
/// The call `n times text` is parsed to: the text repeated n times
pub const TEXT_TIMES: &str = "times·text";
/// The acknowledge-once note that a spaced `//` after code is a comment, not Python's floor division
const SLASH_COMMENT_TOPIC: &str = "slash-comment";
/// Directive words after `#` that keep the line a comment (`#use lib`, `#include x`, `#import f from "m"`);
/// besides them only `# ` with a space, `#!` (shebang) and `##` (doc comment) start a comment, any other `#x` counts
const HASH_DIRECTIVES: [&str; 3] = ["use", "include", "import"];
const ELVIS_WORD: &str = "elvis";
/// `x is int` tests the type; `x == int` stays equality (user decision #30)
const IS_WORD: &str = "is";
const HEX_WORD: &str = "hex";
/// `N times` takes everything up to an assignment: `n+1 times {…}`
const TIMES_BP: u8 = 50;

/// Superscript digits ⁰…⁹ in digit order: `3⁴` is 3^4 and a run of them is one exponent (`2¹⁰` is 2^10)
const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
/// A vulgar fraction such as ⅓ decomposes (NFKD) into numerator, this slash and denominator: 1⁄3
const FRACTION_SLASH: char = '⁄';

/// Parser options for handling different file formats
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(Default)]
pub struct ParserOptions {
	/// XML mode: treat <tag> as XML tags, not C++ generics
	pub xml_mode: bool,
	/// Data mode (untrusted or foreign data, never evaluated): English words stay symbols
	/// (`NO` is Norway, not false) and numbers keep their source literal (`01234`, `1.10`)
	pub data_mode: bool,
	/// WIT syntax: `name<a, b>` applies a type (`tuple<s64, s64>`), `struct`/`type` are plain words
	pub wit_mode: bool,
}


impl ParserOptions {
	pub fn wit() -> Self {
		ParserOptions { wit_mode: true, ..Default::default() }
	}

	pub fn xml() -> Self {
		ParserOptions { xml_mode: true, ..Default::default() }
	}

	pub fn data() -> Self {
		ParserOptions { data_mode: true, ..Default::default() }
	}
}

/// A word that can take type arguments: a capitalised type, a built-in type word, a plural type word, `list`, `map` …
fn names_a_type(word: &str) -> bool {
	word.chars().next().is_some_and(char::is_uppercase)
		|| crate::analyzer::type_word_kind(&word.to_lowercase()).is_some()
		|| crate::analyzer::plural_element_type(word).is_some()
		|| GENERIC_TYPE_HEADS.contains(&word)
}

/// The type `name<arguments>` written in words: `list<list<int>>` is `list of list of int`
fn type_application_name(name: &str, arguments: &str) -> String {
	let arguments = arguments.replace('<', " of ").replace('>', "");
	format!("{name} of {}", arguments.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// The sign of a superscript exponent: ⁺ is +1, ⁻ is -1
fn superscript_sign(ch: char) -> Option<i64> {
	match ch {
		'⁺' => Some(1),
		'⁻' => Some(-1),
		_ => None,
	}
}

fn superscript_digit(ch: char) -> Option<i64> {
	SUPERSCRIPT_DIGITS.chars().position(|digit| digit == ch).map(|position| position as i64)
}

/// Numerator and denominator of a Unicode vulgar fraction character: ½ → (1, 2), ⅚ → (5, 6), ↉ → (0, 3)
fn vulgar_fraction(ch: char) -> Option<(i64, i64)> {
	let decomposed: String = std::iter::once(ch).nfkd().collect();
	let (numerator, denominator) = decomposed.split_once(FRACTION_SLASH)?;
	Some((numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Characters that are numeric for Unicode but written next to a number as operators or literals of their own
fn is_number_glyph(ch: char) -> bool {
	superscript_digit(ch).is_some() || vulgar_fraction(ch).is_some()
}

fn is_identifier_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

/// `floor_quotient(a, b)`: what `a // b` and `a div b` are, the Euclidean quotient that goes with `%`
/// (a == b*(a//b) + a%b): floor(a/b) for a positive divisor, ceil(a/b) for a negative one. Each operand is evaluated
/// once, and an exact quotient rounds exactly (wasm_emitter exact_euclid_div)
pub const FLOOR_QUOTIENT: &str = "floor_quotient";

/// `|>` binds below range and arithmetic, above `as` and comparisons
const PIPELINE_BINDING_POWER: (u8, u8) = (127, 128);

/// `value |> f(args)` → `f(value, args)`, `value |> f` → `f(value)`
fn piped(value: Node, stage: Node) -> Node {
	match stage.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Symbol(_))) => {
			let mut items = items.clone();
			items.insert(1, value);
			Node::List(items, Bracket::Round, Separator::None)
		}
		_ => Node::List(vec![stage, value], Bracket::Round, Separator::None),
	}
}

fn floor_division(dividend: Node, divisor: Node) -> Node {
	Node::List(vec![Symbol(FLOOR_QUOTIENT.to_string()), dividend, divisor], Bracket::Round, Separator::None)
}

/// Read and parse a WASP file
/// The body block at the end of an `if`/`while` header: `c {b}` is the pair [c {b}], and in `i < n {b}` the
/// block juxtaposes only with the rightmost operand `n`, so it is split off the right spine of the comparison
fn split_trailing_block(header: &Node, empty_is_block: bool) -> Option<(Node, Node)> {
	match header.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => match items[1].drop_meta() {
			Node::List(_, Bracket::Curly, _) => Some((items[0].clone(), items[1].clone())),
			Empty if empty_is_block => Some((items[0].clone(), items[1].clone())),
			_ => None,
		},
		// `if x in xs {…}`: the block after the collection is the body
		Node::List(items, bracket, separator) if items.len() == 3 && matches!(items[1].drop_meta(), Node::Symbol(word) if word == IN_KEYWORD) => {
			split_trailing_block(&items[2], empty_is_block)
				.map(|(collection, block)| (Node::List(vec![items[0].clone(), items[1].clone(), collection], bracket.clone(), separator.clone()), block))
		}
		Node::Key(left, op, right) if op.is_comparison() || op.is_arithmetic() || op.is_logical() || op.is_prefix() => split_trailing_block(right, empty_is_block)
			.map(|(operand, block)| (Node::Key(left.clone(), *op, Box::new(operand)), block)),
		_ => None,
	}
}

pub fn parse_file(path: &str) -> Node {
	let options = if path.ends_with(".wit") { ParserOptions::wit() } else { ParserOptions::default() };
	match read_to_string(path) {
		Ok(content) => WaspParser::parse_with_options(&content, options),
		_ => error(&format!("Failed to read {}", path)),
	}
}

pub fn parse(input: &str) -> Node {
	if input.ends_with(".wasp") {
		return parse_file(input);
	}
	WaspParser::parse(input)
}

/// The number a text spells if the whole text is one number literal, so `int "12a"` never guesses
pub fn number_in_text(text: &str) -> Option<Number> {
	let text = text.trim();
	let spells_number = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || "+-._eE".contains(c));
	if !spells_number {
		return None;
	}
	match WaspParser::parse(text).drop_meta() {
		Node::Number(number) => Some(*number),
		_ => None,
	}
}

/// `(col // 3) * 3`: a spaced `//` after code starts a comment, and when the rest of its line closes a bracket that is
/// open before it, the writer meant floor division. The line, column and the hidden closing bracket.
fn comment_hiding_a_closer(input: &str) -> Option<(usize, usize, char)> {
	for (line_index, line) in input.lines().enumerate() {
		let chars: Vec<char> = line.chars().collect();
		let (mut depth, mut quote): (i32, Option<char>) = (0, None);
		for (column, &c) in chars.iter().enumerate() {
			match (quote, c) {
				(Some(open), _) if c == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(c),
				(None, '(' | '[' | '{') => depth += 1,
				(None, ')' | ']' | '}') => depth -= 1,
				(None, '/') if chars.get(column + 1) == Some(&'/') => {
					let spaced = column > 0 && chars[column - 1] == ' ' && chars.get(column + 2) == Some(&' ');
					let rest = &chars[column + 2..];
					let closes = rest.iter().map(|c| match c { ')' | ']' | '}' => 1, '(' | '[' | '{' => -1, _ => 0 }).sum::<i32>();
					if spaced && depth > 0 && closes > 0 {
						let closer = rest.iter().find(|c| matches!(c, ')' | ']' | '}')).copied().unwrap_or(')');
						return Some((line_index + 1, column + 1, closer));
					}
					break;
				}
				_ => {}
			}
		}
	}
	None
}

/// Parse-only path for untrusted data: nothing is evaluated, no word is guessed to be a boolean
pub fn parse_data(input: &str) -> Node {
	WaspParser::parse_with_options(input, ParserOptions::data())
}

pub fn parse_xml(input: &str) -> Node {
	WaspParser::parse_with_options(input, ParserOptions::xml())
}

pub struct WaspParser {
	input: String,
	chars: Vec<char>,
	pos: usize,
	line_nr: usize,
	column: usize,
	char: char,
	pub current_line: String,
	base_indent: usize,
	/// Inside a block indented with spaces below a trailing `:` spaces count as indent too (elsewhere only tabs do)
	indent_counts_spaces: bool,
	options: ParserOptions,
	/// Inside an `if`/`while` condition `=` compares instead of assigning (wiki/Bad.md)
	equals_compares: bool,
	/// Inside the iterable of `for x in …` a block is the loop body, never an argument: `for i in 0..n {…}`
	in_for_header: bool,
	/// Where the innermost bracketed group opened (line, column): an unclosed one names it
	group_start: (usize, usize),
	/// While the `then` body of `if c: body else …` is parsed, `else` ends it instead of joining it
	stops_at_else: bool,
	/// The position of the sign in `1 -1` read as the list `[1 -1]`: no enclosing expression subtracts it either (`x=1 -1`)
	signed_list_element: Option<usize>,
	/// Inside `do … end`: the `end` keyword closes the statement list
	stops_at_end: bool,
	/// `N times` loops parsed so far, numbering their hidden counters
	times_loops: usize,
	/// The symbol parsed last was a function keyword (`def`, `function`): the next one is the function's name
	after_function_keyword: bool,
	/// `a ?: b` with a computed left side parsed so far, numbering their hidden variables
	elvis_operands: usize,
	/// Names defined with `:=` so far: a braceless call of one may take an identifier argument anywhere (`fac it-1`)
	functions: std::collections::HashSet<String>,
	/// Those of them declared with named parameters (`f x y := …`), the rest take the implicit `it`
	functions_with_parameters: std::collections::HashSet<String>,
	/// Operators the program declares (`suffix operator ‼ := it*2`), longest glyph first, found by a pre-scan of the source
	user_operators: Vec<UserOperator>,
	/// Types the program declares (`class point{…}`), found by a pre-scan of the source: `point{x:1}` constructs one (D4)
	declared_types: std::collections::HashSet<String>,
	/// The parameters of each `to` phrase defined so far, to catch a second phrase that differs only by untyped nouns
	phrase_definitions: std::collections::HashMap<String, Vec<Node>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum UserOperatorKind {
	Prefix,
	Suffix,
	Infix,
}

#[derive(Clone, Debug)]
struct UserOperator {
	glyph: Vec<char>,
	kind: UserOperatorKind,
	/// How tight it binds: a suffix's power, a prefix's right power, an infix's left power (its right one is one more)
	level: u8,
}

impl UserOperatorKind {
	fn default_level(self) -> u8 {
		match self {
			UserOperatorKind::Suffix => USER_SUFFIX_BP,
			UserOperatorKind::Prefix => USER_PREFIX_RIGHT_BP,
			UserOperatorKind::Infix => USER_INFIX_BP.0,
		}
	}
}

const OPERATOR_FUNCTION_PREFIX: &str = "operator_";
use crate::node::ATTRIBUTE_MARK;
const OPERATOR_KINDS: [(&str, UserOperatorKind); 3] = [("prefix", UserOperatorKind::Prefix), ("suffix", UserOperatorKind::Suffix), ("infix", UserOperatorKind::Infix)];
/// A user operator binds like the built-ins it resembles: suffix like the superscripts, prefix like unary minus, infix like `+`
const USER_SUFFIX_BP: u8 = 200;
const USER_PREFIX_RIGHT_BP: u8 = 155;
const USER_INFIX_BP: (u8, u8) = (140, 141);
/// `operator ⊕ has precedence above *`: ⊕ binds this much tighter than `*`; the built-in levels are at least 5 apart
const PRECEDENCE_STEP: u8 = 2;
const PRECEDENCE_DIRECTIONS: [(&str, bool); 2] = [("above", true), ("below", false)];

/// A glyph a program may declare as an operator: symbols with a non-ASCII character (`‼`, `⊕`), never a letter or digit.
/// ASSUMPTION P48: superscript digits and signs too (`suffix operator ³`, `prefix operator ⁻`, wiki/operator.md)
fn is_operator_glyph(glyph: &str) -> bool {
	let is_superscript = |c: char| superscript_digit(c).is_some() || superscript_sign(c).is_some();
	!glyph.is_empty() && glyph.chars().all(|c| is_superscript(c) || (!c.is_alphanumeric() && !c.is_whitespace())) && !glyph.is_ascii()
}

/// `suffix operator ⁰ := …` and, ASSUMPTION P48, the short `suffix ⁰ := …`: the glyph and the words after `:=`
fn declared_glyph<'a>(words: &'a [&'a str]) -> Option<&'a str> {
	match words {
		["operator", glyph, ":=", ..] | [glyph, ":=", ..] => Some(glyph),
		_ => None,
	}
}

fn is_plain_name(word: &str) -> bool {
	!word.is_empty() && word.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The type names a source declares: the word after `class`, `struct`, `record` or `type` (not the call `type(x)`),
/// outside comments and texts (`// the type end` declares nothing)
fn scan_declared_types(source: &str) -> std::collections::HashSet<String> {
	let source = code_only(source);
	let words: Vec<&str> = source.split(|c: char| !is_identifier_char(c) && c != '(').flat_map(|word| word.split_inclusive('(')).filter(|word| !word.is_empty()).collect();
	words.windows(2)
		.filter(|pair| TYPE_DECLARATION_WORDS.contains(&pair[0]) || pair[0] == RECORD_WORD || pair[0] == "type")
		.map(|pair| pair[1])
		.filter(|name| is_plain_name(name))
		.map(str::to_string)
		.collect()
}

/// The source with its comments (`// …`, `/* … */`) and texts (`"…"`, `'…'`) blanked out
fn code_only(source: &str) -> String {
	let chars: Vec<char> = source.chars().collect();
	let mut code = String::with_capacity(source.len());
	let mut at = 0;
	while at < chars.len() {
		match text_or_comment_end(&chars, at) {
			Some(end) => {
				code.push(' ');
				at = end;
			}
			None => {
				code.push(chars[at]);
				at += 1;
			}
		}
	}
	code
}

/// The end of the comment (`// …`, `/* … */`) or text (`"…"`, `'…'`) starting at `chars[at]`, if one starts there
pub(crate) fn text_or_comment_end(chars: &[char], at: usize) -> Option<usize> {
	let (c, next) = (chars[at], chars.get(at + 1).copied());
	let skip_until = |from: usize, end: &[char]| (from..chars.len()).find(|&i| chars[i..].starts_with(end)).map_or(chars.len(), |i| i + end.len());
	let after = match (c, next) {
		('/', Some('/')) if at == 0 || chars[at - 1] != ':' => (at..chars.len()).find(|&i| chars[i] == '\n').unwrap_or(chars.len()),
		('/', Some('*')) => skip_until(at + 2, &['*', '/']),
		('"' | '\'', _) => skip_until(at + 1, &[c]),
		_ => return None,
	};
	Some(after.max(at + 1))
}

/// The operators a source declares: `prefix|suffix|infix operator ⊕ := body` and the pattern `a ⊕ b := body` (infix)
fn scan_user_operators(source: &str) -> Vec<UserOperator> {
	let mut found: Vec<UserOperator> = Vec::new();
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		let declared = match words.as_slice() {
			[kind, rest @ ..] if declared_glyph(rest).is_some() => {
				OPERATOR_KINDS.iter().find(|(word, _)| word == kind).and_then(|(_, kind)| Some((declared_glyph(rest)?, *kind)))
			}
			[left, glyph, right, ":=", ..] if is_plain_name(left) && is_plain_name(right) => Some((*glyph, UserOperatorKind::Infix)),
			_ => None,
		};
		if let Some((glyph, kind)) = declared.filter(|(glyph, _)| is_operator_glyph(glyph)) {
			found.push(UserOperator { glyph: glyph.chars().collect(), kind, level: kind.default_level() });
		}
	}
	// `operator ⊕ has precedence above *`, wherever it stands, once every operator is known
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		if let Ok(Some((glyph, level))) = precedence_declaration(&words, &found) {
			found.iter_mut().filter(|operator| operator.glyph == glyph.chars().collect::<Vec<char>>()).for_each(|operator| operator.level = level);
		}
	}
	found.sort_by_key(|operator| std::cmp::Reverse(operator.glyph.len()));
	found
}

/// `operator ⊕ has precedence above|below [operator] Y`: the glyph ⊕ and its new level; Ok(None) for any other statement,
/// Err for a precedence declaration that cannot be followed (a built-in ⊕, an unknown Y)
fn precedence_declaration<'a>(words: &[&'a str], declared: &[UserOperator]) -> Result<Option<(&'a str, u8)>, String> {
	let ["operator", glyph, "has", "precedence", direction, rest @ ..] = words else { return Ok(None) };
	let Some((_, above)) = PRECEDENCE_DIRECTIONS.iter().find(|(word, _)| word == direction) else { return Ok(None) };
	let other = match rest {
		["operator", other] | [other] => *other,
		_ => return Ok(None),
	};
	if !declared.iter().any(|operator| operator.glyph.iter().collect::<String>() == *glyph) {
		return Err(format!("operator {glyph} is built in: only a declared operator gets a precedence (`infix operator {glyph} := …` first)"));
	}
	let reference = match declared.iter().find(|operator| operator.glyph.iter().collect::<String>() == other) {
		Some(operator) => operator.level,
		None => built_in_level(other).ok_or_else(|| format!("unknown operator {other} in `operator {glyph} has precedence {direction} {other}`"))?,
	};
	let level = if *above { reference.saturating_add(PRECEDENCE_STEP) } else { reference.saturating_sub(PRECEDENCE_STEP) };
	Ok(Some((glyph, level)))
}

/// The left binding power of a built-in infix operator written as `text` (`*`, `+`, `and`)
fn built_in_level(text: &str) -> Option<u8> {
	let parser = WaspParser::new_with_options(text.to_string(), ParserOptions { data_mode: true, ..ParserOptions::default() });
	match parser.peek_operator() {
		Some((op, length)) if length == text.chars().count() => Some(op.binding_power().0),
		_ => None,
	}
}

/// The single argument `(int x)` of `f(int x)` is the typed parameter `x:int`, so that flattening the call `f(T, y)` into
/// `(f T y)` never reads a parameter named like a type (`offset_of(text, byte, start)`) as the type of the next one
fn typed_parameter(arguments: Node) -> Node {
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) if matches!(items.as_slice(), [type_word, name]
			if matches!(type_word.drop_meta(), Symbol(word) if crate::analyzer::type_word_kind(word).is_some()) && matches!(name.drop_meta(), Symbol(_))) => {
			Node::Key(Box::new(items[1].clone()), Op::Colon, Box::new(items[0].clone()))
		}
		_ => arguments,
	}
}

/// Every argument of `f(int x, float y)` as its typed parameter
fn typed_parameters(arguments: Node) -> Node {
	match arguments.drop_meta() {
		Node::List(items, bracket, Separator::Colon) => {
			Node::List(items.iter().cloned().map(typed_parameter).collect(), bracket.clone(), Separator::Colon)
		}
		_ => typed_parameter(arguments),
	}
}

fn operator_call(glyph: &[char], arguments: Vec<Node>) -> Node {
	let name = format!("{OPERATOR_FUNCTION_PREFIX}{}", glyph.iter().collect::<String>());
	Node::List([vec![Symbol(name)], arguments].concat(), Bracket::Round, Separator::None)
}

enum ElseParseMode {
	Atom,
	Expr,
}

impl WaspParser {
	pub fn new(input: String) -> Self {
		Self::new_with_options(input, ParserOptions::default())
	}

	/// Source text is normalized to NFC, so equal-looking text and identifiers are equal
	pub fn new_with_options(input: String, options: ParserOptions) -> Self {
		let input: String = input.nfc().collect();
		let input = if options.data_mode { input } else { crate::uniscript_entities::expand_entities(&input) };
		let current_line = input.lines().next().unwrap_or("").to_string();
		let chars: Vec<char> = input.chars().collect();
		let user_operators = if options == ParserOptions::default() { scan_user_operators(&input) } else { Vec::new() };
		let declared_types = if options == ParserOptions::default() { scan_declared_types(&input) } else { Default::default() };
		WaspParser {
			user_operators,
			declared_types,
			phrase_definitions: Default::default(),
			input,
			chars,
			pos: 0,
			line_nr: 1,
			column: 1,
			char: '\0',
			current_line,
			base_indent: 0,
			indent_counts_spaces: false,
			options,
			equals_compares: false,
			in_for_header: false,
			group_start: (0, 0),
			stops_at_else: false,
			signed_list_element: None,
			stops_at_end: false,
			times_loops: 0,
			after_function_keyword: false,
			elvis_operands: 0,
			functions: Default::default(),
			functions_with_parameters: Default::default(),
		}
	}

	/// Set the hint position to current parser position
	fn set_hint_pos(&self) {
		set_hint_position(self.line_nr, self.column);
	}

	/// The declared operator written at the cursor, of one of the kinds
	fn user_operator_at(&self, kinds: &[UserOperatorKind]) -> Option<UserOperator> {
		self.user_operators.iter().find(|operator| kinds.contains(&operator.kind) && self.chars[self.pos..].starts_with(&operator.glyph)).cloned()
	}

	/// `3‼`: the call of the suffix operator on the left operand
	fn try_parse_user_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Suffix])?;
		if min_bp > operator.level {
			return None;
		}
		self.advance_by(operator.glyph.len());
		Some(operator_call(&operator.glyph, vec![lhs.clone()]))
	}

	/// `2 ⊕ 3`: the call of the infix operator on both operands
	fn try_parse_user_infix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Infix])?;
		let (left_bp, right_bp) = (operator.level, operator.level + 1);
		if left_bp < min_bp {
			return None;
		}
		self.advance_by(operator.glyph.len());
		self.skip_whitespace();
		let rhs = self.parse_expr(right_bp);
		Some(operator_call(&operator.glyph, vec![lhs.clone(), rhs]))
	}

	/// `∆3`: the call of the prefix operator on the operand after it
	fn try_parse_user_prefix(&mut self) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Prefix])?;
		self.advance_by(operator.glyph.len());
		self.skip_spaces();
		let operand = self.parse_expr(operator.level);
		Some(operator_call(&operator.glyph, vec![operand]))
	}

	/// `prefix|suffix|infix operator ⊕ := body` defines the function `operator_⊕`: its parameter is `it` (prefix, suffix) or `a b` (infix).
	/// `operator ⊕ has precedence above +` is not supported.
	fn try_parse_operator_declaration(&mut self, word: &str) -> Option<Node> {
		if self.options != ParserOptions::default() {
			return None;
		}
		let rest: String = self.chars[self.pos..].iter().take_while(|c| **c != '\n' && **c != ';').collect();
		let words: Vec<&str> = rest.split_whitespace().collect();
		if word == "operator" && words.get(1) == Some(&"has") && words.get(2) == Some(&"precedence") {
			while !matches!(self.current_char(), '\n' | ';' | '\0') {
				self.advance();
			}
			// the pre-scan applied it (scan_user_operators); here only its error is reported
			let statement: Vec<&str> = std::iter::once("operator").chain(words.iter().copied()).collect();
			return Some(match precedence_declaration(&statement, &self.user_operators) {
				Ok(Some(_)) => Empty,
				Ok(None) => error(&format!("an operator precedence is declared `operator ⊕ has precedence above|below Y`, not `operator {}`", words.join(" "))),
				Err(message) => error(&message),
			});
		}
		let (_, kind) = OPERATOR_KINDS.iter().find(|(keyword, _)| *keyword == word)?;
		let glyph = declared_glyph(&words).filter(|glyph| is_operator_glyph(glyph))?;
		let has_operator_word = words.first() == Some(&"operator");
		let glyph: Vec<char> = glyph.chars().collect();
		self.skip_spaces();
		if has_operator_word {
			self.advance_by("operator".chars().count());
			self.skip_spaces();
		}
		self.advance_by(glyph.len());
		self.skip_spaces();
		self.advance_by(":=".len());
		let body = self.parse_expr(0);
		let parameters = match kind {
			UserOperatorKind::Infix => vec![Symbol("a".to_string()), Symbol("b".to_string())],
			_ => vec![Symbol("it".to_string())],
		};
		let head = operator_call(&glyph, parameters);
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// Hint when the operator written at the cursor is not the canonical one; call once when the operator is consumed
	/// The operator as written, after hinting at its preferred spelling
	fn hint_operator(&self, chars: usize, is_prefix: bool) -> String {
		let written: String = (0..chars).map(|offset| self.peek_char(offset)).collect();
		self.set_hint_pos();
		norm::operator(&written, is_prefix);
		written
	}

	/// Ranges whose end readers expect either way are Asks, by default read as wasp does (exclusive):
	/// `a upto b` (wiki/range.md: excludes b) and the loop bound `for i in 0..n-1` (Kotlin's `..` includes n-1)
	fn range_reading(&self, op: Op, written: &str, end: &Node, line: usize, column: usize) -> Result<Op, Node> {
		let end_text = crate::normalize::operand_text(end);
		let question = match written {
			UPTO => format!("does `upto {end_text}` include {end_text}? (`..<` or `..` exclude it, `to` or `...` include it)"),
			EXCLUSIVE_DOTS if self.in_for_header && is_minus_one(end) => {
				format!("does the loop bound `..{end_text}` include {end_text}? (wasp's `..` excludes it, Kotlin's includes it)")
			}
			_ => return Ok(op),
		};
		let topic = if written == UPTO { UPTO } else { KOTLIN_RANGE };
		let readings = vec![reading("exclusive", "..<"), reading("inclusive", "...")];
		let chosen = ask(&Ask::new(topic, question, readings, Fallback::Warning).written(written).at(line, column))?;
		Ok(if chosen == 0 { Op::Range } else { Op::To })
	}

	pub fn parse(input: &str) -> Node {
		Self::parse_with_options(input, ParserOptions::default())
	}

	pub fn parse_with_options(input: &str, options: ParserOptions) -> Node {
		if let Some((line, column, closer)) = comment_hiding_a_closer(input).filter(|_| !options.data_mode) {
			return error(&format!("`//` at {line}:{column} starts a comment that hides the closing `{closer}`: floor division is written glued, a//b"));
		}
		let mut parser = WaspParser::new_with_options(input.to_string(), options);
		let program = parser.parse_list_with_separators(None, Bracket::None);
		if options == ParserOptions::default() {
			crate::normalize::check_style(&program);
		}
		if program.is_nothing() { Empty } else { program }
	}

	fn end_of_input(&self) -> bool {
		self.pos >= self.chars.len()
	}

	fn current_char(&self) -> char {
		*self.chars.get(self.pos).unwrap_or(&'\0')
	}

	fn is_identifier_start(&self, offset: usize) -> bool {
		let letter = self.peek_char(offset);
		letter.is_alphabetic() || letter == '_'
	}

	fn peek_char(&self, offset: usize) -> char {
		*self.chars.get(self.pos + offset).unwrap_or(&'\0')
	}

	/// Do blanks, an identifier, optional blanks and a `{` follow the cursor: `record point {…}`
	fn name_and_block_follow(&self) -> bool {
		let is_blank = |ch: char| matches!(ch, ' ' | '\t');
		let mut offset = 0;
		while is_blank(self.peek_char(offset)) {
			offset += 1;
		}
		let name_start = offset;
		while is_identifier_char(self.peek_char(offset)) {
			offset += 1;
		}
		let has_name = offset > name_start;
		while is_blank(self.peek_char(offset)) {
			offset += 1;
		}
		has_name && self.peek_char(offset) == '{'
	}

	fn advance(&mut self) {
		let ch = self.current_char();
		self.char = ch; // debug
		if ch == '\n' {
			self.line_nr += 1;
			self.column = 1;
			// Update current_line to the new line
			let lines: Vec<&str> = self.input.lines().collect();
			if self.line_nr > 0 && self.line_nr <= lines.len() {
				self.current_line = lines[self.line_nr - 1].to_string();
			} else {
				self.current_line = String::new();
			}
		} else {
			self.column += 1;
		}
		self.pos += 1;
	}

	fn get_position(&self) -> (usize, usize) {
		(self.line_nr, self.column)
	}

	fn prev_char(&self) -> char {
		if self.pos == 0 { '\0' } else { *self.chars.get(self.pos - 1).unwrap_or(&'\0') }
	}

	/// Check if input at current position matches a keyword (followed by non-alphanumeric)
	fn matches_keyword(&self, keyword: &str) -> bool {
		keyword.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !is_identifier_char(self.peek_char(keyword.len()))
	}

	/// `#` followed by a space (`# note`), a shebang `#!`, a doc comment `##` or a directive (`#use lib`) starts a comment;
	/// `#` directly followed by anything else counts (`#s`, `#a-1`, `#f(x)`)
	fn at_hash_comment(&self) -> bool {
		if matches!(self.peek_char(1), ' ' | '\t' | '\n' | '\r' | '\0' | '!' | '#') {
			return true;
		}
		let word: String = (1..).map(|offset| self.peek_char(offset)).take_while(|&c| is_identifier_char(c)).collect();
		HASH_DIRECTIVES.contains(&word.as_str())
	}

	/// `//` right behind an operand (`7//2`, `x//=2`, `f(x)//2`) divides; after a space or `:` (URLs) it starts a comment
	fn at_floor_division(&self) -> bool {
		let previous = self.prev_char();
		self.current_char() == '/' && self.peek_char(1) == '/' && self.pos > 0 && (is_identifier_char(previous) || matches!(previous, ')' | ']'))
	}

	/// `.+ .- .* ./` at the cursor: the element-wise form of the arithmetic operator
	fn element_wise_operator(&self) -> Option<Op> {
		if self.current_char() != '.' {
			return None;
		}
		ELEMENT_WISE_OPERATORS.iter().find(|(glyph, _)| *glyph == self.peek_char(1)).map(|(_, op)| *op)
	}

	/// `1 -1`, `[1 +1]`: a number, a space, then a sign glued to a number (D13). Asked: the list `[1 -1]` (the default,
	/// a warning when unanswered) or the arithmetic `1 - 1`. Variables, calls and words keep subtracting: `x -1`
	fn signed_number_starts_a_list(&mut self, lhs: &Node) -> Result<bool, Node> {
		if self.signed_list_element == Some(self.pos) {
			return Ok(true); // answered for the operand inside
		}
		if !matches!(lhs.drop_meta(), Node::Number(_)) || !self.prev_char().is_whitespace() || !self.number_starts_at(1) {
			return Ok(false);
		}
		let signed: String = self.chars[self.pos..].iter().take_while(|ch| !ch.is_whitespace() && !matches!(ch, ']' | ')' | '}' | ',' | ';')).collect();
		let (first, sign, number) = (lhs.serialize(), self.current_char(), &signed[1..]);
		let written = format!("{first} {signed}");
		let readings = vec![
			crate::diagnostic::reading("the list", &format!("[{written}]")),
			crate::diagnostic::reading("arithmetic", &format!("{first} {sign} {number}")),
		];
		let question = crate::diagnostic::Ask::new(SIGNED_OPERAND_TOPIC, format!("is `{written}` a list or arithmetic?"), readings, crate::diagnostic::Fallback::Warning)
			.written(&written).at(self.line_nr, self.column);
		let is_list = crate::diagnostic::ask(&question)? == 0;
		if is_list {
			self.signed_list_element = Some(self.pos);
		}
		Ok(is_list)
	}

	/// Code stands before the cursor on its line (`x = 7 // note`, not a `// note` line of its own)
	fn follows_code_on_its_line(&self) -> bool {
		self.chars[..self.pos].iter().rev().take_while(|&&c| c != '\n').any(|c| !c.is_whitespace())
	}

	fn is_at_line_start(&self) -> bool {
		// Check if we're at the very beginning or right after whitespace/newline
		self.pos == 0 || self.prev_char().is_whitespace()
	}

	/// Skip whitespace, return (had_newline, current_line_indent)
	/// Only tabs count as semantic indent (not spaces)
	fn skip_whitespace(&mut self) -> (bool, usize) {
		let mut had_newline = false;
		let mut line_indent = 0;
		loop {
			let ch = self.current_char();
			if !ch.is_whitespace() {
				break;
			}
			if ch == '\n' {
				had_newline = true;
				line_indent = 0; // reset for new line
			} else if ch == '\t' || (ch == ' ' && self.indent_counts_spaces) {
				line_indent += 1; // tabs count as indent, spaces only inside a space-indented block
			}
			self.advance();
		}
		(had_newline, line_indent)
	}

	/// Blanks and line continuations: a `\` at the end of a line joins the next line to the statement
	fn skip_spaces(&mut self) {
		loop {
			if self.current_char() == ' ' || self.current_char() == '\t' {
				self.advance();
			} else if let Some(continuation_length) = self.line_continuation_length() {
				self.advance_by(continuation_length);
			} else {
				return;
			}
		}
	}

	/// Is the keyword at the cursor followed (after blanks) by `{`: `div {…}` is a tag, not the operator
	fn word_opens_block(&self, word: &str) -> bool {
		let after = word.chars().count();
		let gap = (after..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		self.peek_char(after + gap) == '{'
	}

	/// The identifier starting `offset` characters ahead
	fn word_at(&self, offset: usize) -> String {
		(offset..).map(|at| self.peek_char(at)).take_while(|c| is_identifier_char(*c)).collect()
	}

	/// The identifier that ends right before the cursor and the blanks before it, on this line
	fn word_before(&self) -> &str {
		let line = &self.input_before_cursor();
		let trimmed = line.trim_end_matches([' ', '\t', '\r']);
		let start = trimmed.rfind(|c: char| !is_identifier_char(c)).map_or(0, |at| at + trimmed[at..].chars().next().map_or(1, char::len_utf8));
		match &trimmed[start..] {
			"then" => "then",
			ELSE_KEYWORD => ELSE_KEYWORD,
			_ => "",
		}
	}

	/// The source line up to the cursor
	fn input_before_cursor(&self) -> String {
		let before: String = self.chars[..self.pos].iter().rev().take_while(|c| **c != '\n').collect();
		before.chars().rev().collect()
	}

	/// Length of `\` plus trailing blanks and the newline, when the backslash ends its line; or of the line break and
	/// indentation before a method call that starts the next line (`numbers\n    .map(square)`, as in JS, Kotlin, Swift)
	fn line_continuation_length(&self) -> Option<usize> {
		if matches!(self.current_char(), '\n' | '\r') && !self.options.data_mode {
			let mut length = 1;
			while matches!(self.peek_char(length), ' ' | '\t' | '\r' | '\n') {
				length += 1;
			}
			let starts_method = self.peek_char(length) == '.' && (self.peek_char(length + 1).is_alphabetic() || self.peek_char(length + 1) == '_');
			let starts_pipeline = self.peek_char(length) == '|' && self.peek_char(length + 1) == '>';
			// `if c then⏎ a⏎ else⏎ b`: a line ending in then/else, or one starting with else, continues the if
			let starts_else = self.word_at(length) == ELSE_KEYWORD;
			let ends_in_branch_word = matches!(self.word_before(), "then" | ELSE_KEYWORD);
			return (starts_method || starts_pipeline || starts_else || ends_in_branch_word).then_some(length);
		}
		if self.current_char() != '\\' {
			return None;
		}
		let mut length = 1;
		while matches!(self.peek_char(length), ' ' | '\t' | '\r') {
			length += 1;
		}
		(self.peek_char(length) == '\n').then_some(length + 1)
	}

	fn consume_rest_of_line(&mut self) -> String {
		let mut line_comment = String::new();
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				break;
			}
			if ch == '\n' {
				self.advance();
				break;
			}
			line_comment.push(ch);
			self.advance();
		}
		line_comment.trim().to_string()
	}

	/// `/* … */` and `/# … #/` open a block comment
	fn at_block_comment_start(&self) -> bool {
		self.current_char() == '/' && matches!(self.peek_char(1), '*' | '#')
	}

	/// The text of a `/* … */` or `/# … #/` comment, the parser standing on its opening; comments nest:
	/// `/* outer /* inner */ still outer */`
	fn consume_block_comment(&mut self) -> String {
		let mark = if self.peek_char(1) == '#' { '#' } else { '*' };
		self.advance_by(2);
		let mut block = String::new();
		let mut depth = 1;
		while self.current_char() != '\0' {
			let pair = (self.current_char(), self.peek_char(1));
			if pair == (mark, '/') || pair == ('/', mark) {
				depth += if pair.0 == '/' { 1 } else { -1 };
				if depth == 0 {
					self.advance_by(2);
					break;
				}
				block.extend([pair.0, pair.1]);
				self.advance_by(2);
				continue;
			}
			block.push(self.current_char());
			self.advance();
		}
		block
	}

	/// Spaces and the comments that may sit inside a line: `1 /* inline */ + 1` and `a b # note`; the newline stays a separator
	fn skip_spaces_and_inline_comments(&mut self) {
		loop {
			self.skip_spaces();
			match (self.current_char(), self.peek_char(1)) {
				_ if self.at_block_comment_start() => { self.consume_block_comment(); }
				('#', ' ' | '\t') => {
					while !matches!(self.current_char(), '\n' | '\0') {
						self.advance();
					}
				}
				_ => return,
			}
		}
	}

	fn skip_whitespace_and_comments(&mut self) -> (bool, usize, Option<String>) {
		let mut had_newline = false;
		let mut line_indent = 0;
		let mut comments = Vec::new();
		loop {
			let (newline, indent) = self.skip_whitespace();
			had_newline |= newline;
			if newline { line_indent = indent; }

			let (c1, c2) = (self.current_char(), self.peek_char(1));

			// # line comment (shell-style) at line start: `# note`, `#!`, `##`, `#use …`; `#x`, `#(…)`, `#a-1` count
			if c1 == '#' && self.is_at_line_start() && self.at_hash_comment() {
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// // line comment (but not :// URL scheme)
			if c1 == '/' && c2 == '/' && self.prev_char() != ':' && !self.at_floor_division() {
				if self.follows_code_on_its_line() {
					self.set_hint_pos();
					crate::diagnostic::educate_once(SLASH_COMMENT_TOPIC, "a // b", "a//b", "`// …` after code is a comment; floor division is written glued: a//b");
				}
				self.advance_by(2);
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// /* block comment */ and /# block comment #/
			if self.at_block_comment_start() {
				let block = self.consume_block_comment();
				had_newline |= block.contains('\n');
				let trimmed = block.trim();
				if !trimmed.is_empty() { comments.push(trimmed.to_string()); }
				continue;
			}
			break;
		}
		let comment = if comments.is_empty() { None } else { Some(comments.join("\n")) };
		(had_newline, line_indent, comment)
	}

	/// Skip characters until the target character is found
	fn skip_until(&mut self, target: char) {
		while !self.end_of_input() && self.current_char() != target {
			self.advance();
		}
	}

	/// Parse XML text content (everything until '<' or end of input)
	fn parse_xml_text_content(&mut self) -> String {
		let mut text = String::new();
		while !self.end_of_input() && self.current_char() != '<' {
			text.push(self.current_char());
			self.advance();
		}
		text.trim().to_string()
	}

	/// Skip XML processing instruction: <?xml ... ?>
	fn skip_processing_instruction(&mut self) -> Node {
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
	fn skip_xml_comment(&mut self) -> Node {
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
	fn skip_doctype(&mut self) -> Node {
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
	fn parse_cdata(&mut self) -> Node {
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

	fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == '\n' || self.pos >= self.input.len()
	}

	/// A `;` that ends its line is a statement terminator of the same strength as the newline after it
	fn only_blanks_before_newline(&self) -> bool {
		self.chars[self.pos..].iter().find(|ch| !matches!(ch, ' ' | '\t' | '\r')) == Some(&'\n')
	}

	/// Check if current character can start an atom (for implicit application)
	fn can_start_atom(&self) -> bool {
		let ch = self.current_char();
		ch.is_alphanumeric() || ch == '_' || ch == '"' || ch == '\'' || ch == '(' || ch == '[' || ch == '{'
			|| self.number_starts_at(0) || self.starts_function_reference()
	}

	/// `&name`: a reference to the function `name`, an `&` glued to the word after it and not to a word before it (`a &b`, `f(&g)`)
	fn starts_function_reference(&self) -> bool {
		self.current_char() == '&' && self.peek_char(1).is_alphabetic() && !is_identifier_char(self.prev_char())
	}

	/// A digit, or a leading-dot decimal like `.5`
	fn number_starts_at(&self, offset: usize) -> bool {
		let ch = self.peek_char(offset);
		ch.is_ascii_digit() || (ch == '.' && self.peek_char(offset + 1).is_ascii_digit())
	}

	/// An operand may start after `offset` characters and following blanks: not a closing bracket, separator or the end
	fn operand_follows(&self, offset: usize) -> bool {
		let mut position = offset;
		while matches!(self.peek_char(position), ' ' | '\t') {
			position += 1;
		}
		!matches!(self.peek_char(position), '\0' | '\n' | '\r' | '>' | ')' | ']' | '}' | ',' | ';' | '=')
	}

	/// Check if character terminates a URL
	fn is_url_terminator(&self, ch: char) -> bool {
		ch == '\0' || ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r'
			|| ch == ';' || ch == ')' || ch == ']' || ch == '}' || ch == '>'
			|| ch == '"' || ch == '\'' || ch == ',' || ch == '«'
	}

	/// Peek ahead for an infix operator, returns (Op, chars_to_consume) if found
	/// Checks longer operators first (greedy matching)
	fn peek_operator(&self) -> Option<(Op, usize)> {
		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		if (c1, c2) == ('?', ':') {
			return None; // the elvis `?:` is no ternary, `try_parse_elvis` takes it
		}

		if let Some(word) = SIMILARITY_WORDS.iter().find(|word| self.matches_keyword(word)) {
			return Some((Op::Similar, word.len()));
		}
		// Keywords (4-char)
		if self.matches_keyword("then") { return Some((Op::Then, 4)); }
		if self.matches_keyword("else") { return Some((Op::Else, 4)); }

		// 3-char operators
		match (c1, c2, c3) {
			('.', '.', '.') => return Some((Op::To, 3)),
			('.', '.', '<') => return Some((Op::Range, 3)), // Swift-style exclusive range
			('&', '&', '=') => return Some((Op::AndAssign, 3)),
			('|', '|', '=') => return Some((Op::OrAssign, 3)),
			('^', '^', '=') => return Some((Op::XorAssign, 3)),
			('*', '*', '=') => return Some((Op::PowAssign, 3)),
			_ => {}
		}
		// Keywords (3-char)
		if self.matches_keyword("and") { return Some((Op::And, 3)); }
		if self.matches_keyword("xor") { return Some((Op::Xor, 3)); }
		if self.matches_keyword("not") { return Some((Op::Not, 3)); }

		// 2-char operators
		match (c1, c2) {
			('a', 's') if !c3.is_alphanumeric() => return Some((Op::As, 2)),
			('?', '.') if c3.is_alphabetic() || c3 == '_' => return Some((Op::SafeDot, 2)), // `x?.name`; `x ?.5 : 1` is a ternary
			(':', '=') => return Some((Op::Define, 2)),
			('~', '~') => return Some((Op::Similar, 2)),
			(':', ':') => return Some((Op::Scope, 2)),
			('-', '>') => return Some((Op::Arrow, 2)),
			('=', '>') => return Some((Op::FatArrow, 2)),
			('*', '*') => return Some((Op::Pow, 2)),
			('+', '=') => return Some((Op::AddAssign, 2)),
			('-', '=') => return Some((Op::SubAssign, 2)),
			('*', '=') => return Some((Op::MulAssign, 2)),
			('/', '=') => return Some((Op::DivAssign, 2)),
			('%', '=') => return Some((Op::ModAssign, 2)),
			('^', '=') => return Some((Op::PowAssign, 2)),
			('<', '<') if !self.options.wit_mode => return Some((Op::Shl, 2)),
			// `list<list<int>>` closes two generics, a shift has an operand behind it
			('>', '>') if !self.options.wit_mode && self.operand_follows(2) => return Some((Op::Shr, 2)),
			('<', '=') => return Some((Op::Le, 2)),
			('>', '=') => return Some((Op::Ge, 2)),
			('=', '=') => return Some((Op::Eq, 2)),
			('!', '=') => return Some((Op::Ne, 2)),
			('+', '+') => return Some((Op::Inc, 2)),
			('-', '-') => return Some((Op::Dec, 2)),
			('.', '.') => return Some((Op::Range, 2)),
			('+', '-') if c3.is_whitespace() && self.prev_char().is_whitespace() => return Some((Op::PlusMinus, 2)),
			('&', '&') => return Some((Op::And, 2)),
			('|', '|') => return Some((Op::Or, 2)),
			_ => {}
		}
		// Keywords (2-char)
		if self.matches_keyword("or") { return Some((Op::Or, 2)); }
		if self.matches_keyword("is") { return Some((Op::Eq, 2)); } // wiki/equality.md: `is` compares by value like ==
		if self.matches_keyword("be") { return Some((Op::Define, 2)); } // wiki/be.md
		if self.matches_keyword("if") { return Some((Op::If, 2)); }
		if self.matches_keyword("do") { return Some((Op::Do, 2)); }
		if self.matches_keyword("to") { return Some((Op::To, 2)); }
		if self.matches_keyword("upto") { return Some((Op::Range, 4)); } // wiki/range.md: `1 upto 10` excludes 10
		// Kotlin's `for i in 0 until n`; elsewhere `until` guards a statement: `i++ until c`
		if self.in_for_header && self.matches_keyword("until") { return Some((Op::Range, 5)); }

		// 1-char operators
		match c1 {
			':' => Some((Op::Colon, 1)),
			'=' if self.equals_compares => Some((Op::Eq, 1)),
			'=' => Some((Op::Assign, 1)),
			// `a .5` is a list of two values, `a.5` a member access
			'.' if !(self.prev_char().is_whitespace() && self.number_starts_at(0)) => Some((Op::Dot, 1)),
			'+' => Some((Op::Add, 1)),
			'±' => Some((Op::PlusMinus, 1)),
			'-' => Some((Op::Sub, 1)),
			'*' => Some((Op::Mul, 1)),
			'/' if c2 != '/' => Some((Op::Div, 1)), // Don't treat // as division - it's a comment
			'%' => Some((Op::Mod, 1)),
			'^' => Some((Op::Pow, 1)),
			'×' | '⋅' => Some((Op::Mul, 1)),
			'÷' => Some((Op::Div, 1)),
			'<' | '>' if self.options.wit_mode => None, // angle brackets only delimit type arguments
			'<' => Some((Op::Lt, 1)),
			'>' => Some((Op::Gt, 1)),
			'≤' => Some((Op::Le, 1)),
			'≥' => Some((Op::Ge, 1)),
			'≠' => Some((Op::Ne, 1)),
			'≈' | '⋍' | '~' => Some((Op::Similar, 1)),
			'!' => Some((Op::Not, 1)),
			'¬' => Some((Op::Not, 1)),
			'&' if self.starts_function_reference() && self.prev_char().is_whitespace() => None, // `map &square xs`
			'&' => Some((Op::And, 1)),
			'|' => Some((Op::Or, 1)),
			glyph if glyph_operator(glyph).is_some() => glyph_operator(glyph).map(|(op, _)| (op, 1)),
			'#' => Some((Op::Hash, 1)),
			'?' => Some((Op::Question, 1)),
			'…' => Some((Op::To, 1)),
			_ => None,
		}
	}

	/// `unless`/`until` in front of a statement: the `if`/`while` it reads as, with the condition negated afterwards
	fn peek_negated_control_word(&self) -> Option<(Op, usize)> {
		if self.options.data_mode {
			return None;
		}
		STATEMENT_MODIFIERS.iter()
			.find(|(word, _, negated)| *negated && self.matches_keyword(word))
			.map(|(word, op, _)| (*op, word.len()))
	}

	/// `try` or `assert` followed by an operand: the words that guard a statement
	fn peek_guard_word(&self) -> Option<&'static str> {
		if self.options.data_mode {
			return None;
		}
		GUARD_MARKERS.iter().map(|(word, marker)| (*word, *marker))
			.find(|(word, _)| self.matches_keyword(word) && matches!(self.peek_char(word.len()), ' ' | '\t'))
			.map(|(_, marker)| marker)
	}

	/// `try X else Y` and `assert C else X`, as the marker call `marker(X, Y)` that `library_words` lowers.
	/// X runs to the `else` (it may be an assignment); `assert C` alone has ø for the message.
	fn parse_guard(&mut self, marker: &'static str) -> Node {
		let word_length = GUARD_MARKERS.iter().find(|(_, known)| *known == marker).map_or(0, |(word, _)| word.len());
		self.advance_by(word_length);
		self.skip_spaces();
		let outer = std::mem::replace(&mut self.stops_at_else, true);
		let guarded = self.with_equals_comparing(marker == ASSERT_MARKER, |parser| parser.parse_guarded_phrase());
		self.stops_at_else = outer;
		self.skip_spaces();
		let fallback = if self.matches_keyword("else") {
			self.advance_by("else".len());
			self.skip_spaces();
			self.parse_expr(0)
		} else if marker == TRY_MARKER {
			return error("`try` needs an `else`: `try X else Y`");
		} else {
			Empty
		};
		Node::List(vec![Symbol(marker.to_string()), guarded, fallback], Bracket::Round, Separator::None)
	}

	/// The guarded part of `try X else Y`: one expression, or a braceless call of several (`try raise "boom" else 3`)
	fn parse_guarded_phrase(&mut self) -> Node {
		let mut items = vec![self.parse_expr(0)];
		loop {
			self.skip_spaces();
			if self.matches_keyword("else") || matches!(self.current_char(), '\0' | '\n' | '\r' | ';' | ')' | ']' | '}' | ',') {
				break;
			}
			let position = self.pos;
			let item = self.parse_expr(0);
			if self.pos == position {
				break;
			}
			items.push(item);
		}
		if items.len() == 1 { items.remove(0) } else { Node::List(items, Bracket::None, Separator::Space) }
	}

	/// Peek for prefix operators (unary operators that bind to right operand)
	fn peek_prefix_operator(&self) -> Option<(Op, usize)> {
		if self.matches_keyword("while") { return Some((Op::While, 5)); }
		if self.matches_keyword("sqrt") { return Some((Op::Sqrt, 4)); }
		if self.matches_keyword("cbrt") { return Some((Op::Cbrt, 4)); }
		if self.matches_keyword("not") { return Some((Op::Not, 3)); }
		if self.matches_keyword("abs") { return Some((Op::Abs, 3)); }
		if self.matches_keyword("if") { return Some((Op::If, 2)); }

		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let variable_follows = c3.is_alphabetic() || c3 == '_';
		match c1 {
			'+' if c2 == '+' && variable_follows => Some((Op::Inc, 2)),
			'-' if c2 == '-' && variable_follows => Some((Op::Dec, 2)),
			'-' => Some((Op::Neg, 1)),
			dash if matches!(glyph_operator(dash), Some((Op::Sub, _))) => Some((Op::Neg, 1)),
			'!' | '¬' => Some((Op::Not, 1)),
			'√' => Some((Op::Sqrt, 1)),
			'∛' => Some((Op::Cbrt, 1)),
			'‖' => Some((Op::Abs, 1)),
			'#' => Some((Op::Hash, 1)), // prefix # means count/length
			_ => None,
		}
	}

	/// Peek for suffix operators (unary operators that bind to left operand)
	fn peek_suffix_operator(&self) -> Option<(Op, usize)> {
		match (self.current_char(), self.peek_char(1)) {
			('+', '+') => Some((Op::Inc, 2)),
			('-', '-') => Some((Op::Dec, 2)),
			_ => None,
		}
	}

	/// `$main`, `$ii_i`: names keep their sigil (WAT identifiers, DOM selectors)
	fn parse_dollar_name(&mut self) -> Node {
		self.advance(); // skip '$'
		match self.parse_symbol() {
			Ok(name) => Symbol(format!("${name}")),
			Err(message) => error(&message),
		}
	}

	fn unwrap_single(group: Node) -> Node {
		match group {
			Node::List(mut items, _, _) if items.len() == 1 => items.remove(0),
			other => other,
		}
	}

	/// `@name` or `@name(value)` annotates the atom that follows: `@version(2) @draft tee{a:1}`.
	/// `@name:value` (inside a literal: `point{x:1 @source:"gps"}`) is the meta entry `@name`, never a field;
	/// after a dot, `p.@name` names the meta key itself.
	fn parse_attribute(&mut self) -> Node {
		let after_dot = self.pos > 0 && self.chars[self.pos - 1] == '.';
		self.advance(); // skip '@'
		let name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		if after_dot {
			return Symbol(format!("{ATTRIBUTE_MARK}{name}"));
		}
		if self.current_char() == ':' && self.peek_char(1) != '=' {
			self.advance();
			self.skip_spaces();
			return Node::Key(Box::new(Symbol(format!("{ATTRIBUTE_MARK}{name}"))), Op::Colon, Box::new(self.parse_atom()));
		}
		let value = if self.current_char() == '(' { Self::unwrap_single(self.parse_bracketed('(')) } else { Node::True };
		self.parse_atom().with_attribute(&name, value)
	}

	/// Parse an atomic expression (no infix operators)
	/// Handles: numbers, strings, brackets, symbols with named blocks
	fn parse_atom(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let (line_nr, column) = self.get_position();

		if self.is_at_line_end() {
			return Empty;
		}

		let node = match self.current_char() {
			// `&name` is the function `name` itself: a name is already a function value where a function is expected
			'&' if self.starts_function_reference() => {
				self.advance();
				match self.parse_symbol() {
					Ok(name) => Symbol(name),
					Err(message) => error(&message),
				}
			}
			'"' | '\'' | '«' => self.parse_string(),
			// `a, *rest = xs`: the starred name takes the items the other names leave (src/lowering/tuples.rs)
			'*' if self.is_identifier_start(1) => {
				self.advance();
				match self.parse_symbol() {
					Ok(name) => Symbol(format!("{}{name}", crate::tuples::STARRED)),
					Err(message) => error(&message),
				}
			}
			'(' | '[' | '{' => self.parse_bracketed(self.current_char()),
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			'<' => self.parse_bracketed('<'),
			';' | '>' | '}' | ')' | ']' => Empty, // Closing brackets/terminators handled by caller
			'ø' => { self.advance(); return Empty }
			'∞' => { self.advance(); return Node::Number(Number::Inf) } // the float infinity (P56)
			// $n parameter reference (e.g., $0 = first param)
			'@' if self.peek_char(1).is_alphabetic() => self.parse_attribute(),
			'$' if self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_' => self.parse_dollar_name(),
			'$' if self.peek_char(1).is_numeric() => {
				self.advance(); // skip '$'
				let mut num_str = String::new();
				while self.current_char().is_numeric() {
					num_str.push(self.current_char());
					self.advance();
				}
				Node::Symbol(format!("${}", num_str))
			}
			ch if ch.is_numeric() || self.number_starts_at(0) || (ch == '-' && self.number_starts_at(1)) => {
				self.parse_number()
			}
			ch if ch.is_alphabetic() || ch == '_' => self.parse_symbol_with_suffix(),
			'\\' if let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos) => {
				(0..length).for_each(|_| self.advance());
				error(&crate::uniscript_entities::unknown_entity(&name))
			}
			'\\' if let Some(name) = crate::uniscript_entities::bare_entity_name_at(&self.chars, self.pos) => {
				(0..=name.len()).for_each(|_| self.advance());
				error(&format!("a uniscript entity is written \\:{name}, not \\{name}"))
			}
			ch => {
				warn!(
					"Unexpected character '{}' at line {}, column {}",
					ch, line_nr, column
				);
				self.advance();
				error(&format!("Unexpected character '{}'", ch))
			}
		};

		if node == Empty {
			return node;
		}

		// Attach metadata
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			#[cfg(debug_assertions)]
			line: self.current_line.clone(),
		});
		if let Some(c) = comment {
			node.with_comment(c)
		} else {
			node
		}
	}

	/// The word of `length` characters just parsed is the first thing of its statement: nothing, a newline, `;` or `{` before it
	fn word_starts_statement(&self, length: usize) -> bool {
		let word_start = self.pos.saturating_sub(length);
		let before = self.chars[..word_start].iter().rev().find(|ch| !matches!(ch, ' ' | '\t' | '\r'));
		matches!(before, None | Some('\n' | ';' | '{'))
	}

	/// `to name params: body`, after the word `to`: the function `name(params) := body`.
	/// The body is the rest of the statement, a `{…}` block or an indented block. Parameters are plain names or, with a
	/// known type word, typed slots (`to square a number:`, type_name_matching::parameter_slots). Anything else is no definition.
	fn try_parse_to_definition(&mut self) -> Option<Node> {
		let before_header = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let restore = |parser: &mut Self| (parser.pos, parser.line_nr, parser.column, parser.current_line) = before_header.clone();
		self.skip_spaces();
		let Some(name) = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() else {
			restore(self);
			return None;
		};
		let mut parameters = Vec::new();
		loop {
			self.skip_spaces();
			// `to square a number: …` or `to square a number { … }`
			if (self.current_char() == ':' && self.peek_char(1) != '=') || self.current_char() == '{' {
				break;
			}
			match self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() {
				Some(parameter) => parameters.push(parameter),
				None => {
					restore(self);
					return None;
				}
			}
		}
		if self.current_char() == ':' {
			self.advance();
			self.skip_spaces();
		}
		let body = self.parse_definition_body();
		self.functions.insert(name.clone());
		let words: Vec<&str> = parameters.iter().map(String::as_str).collect();
		let declared_types = &self.declared_types;
		let is_known_type = |word: &str| {
			crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some() || declared_types.contains(word)
		};
		let parameters = match crate::type_name_matching::parameter_slots(&words, &body, &is_known_type) {
			Ok(parameters) => parameters,
			Err(message) => return Some(error(&message)),
		};
		if let Some(clash) = self.phrase_redefinition(&name, &parameters) {
			return Some(clash);
		}
		let head = if parameters.is_empty() {
			Symbol(name)
		} else {
			self.functions_with_parameters.insert(name.clone());
			self.phrase_definitions.insert(name.clone(), parameters.clone());
			Node::List([vec![Symbol(name)], parameters].concat(), Bracket::Round, Separator::None)
		};
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// `to kill a person: …` after `to kill a dog: …`: the same phrase with other untyped nouns can only be told apart by types
	fn phrase_redefinition(&self, name: &str, parameters: &[Node]) -> Option<Node> {
		let earlier = self.phrase_definitions.get(name).filter(|earlier| earlier.len() == parameters.len())?;
		let untyped = |parameter: &Node| matches!(parameter.drop_meta(), Symbol(_));
		let differing: Vec<(&Node, &Node)> = earlier.iter().zip(parameters).filter(|(before, now)| before != now).collect();
		if differing.is_empty() || !differing.iter().all(|(before, now)| untyped(before) && untyped(now)) {
			return None;
		}
		let classes: Vec<String> = differing.iter().flat_map(|(before, now)| [before.name(), now.name()]).map(|noun| format!("class {noun}")).collect();
		Some(error(&format!("{name} is defined twice; declare {} to dispatch on them", classes.join(" and "))))
	}

	/// Move back to the end of the last thing before the whitespace run behind the cursor
	fn seek_back_over_whitespace(&mut self) {
		let mut position = self.pos.min(self.chars.len());
		while position > 0 && self.chars[position - 1].is_whitespace() {
			position -= 1;
		}
		let line_start = self.chars[..position].iter().rposition(|ch| *ch == '\n').map_or(0, |newline| newline + 1);
		self.pos = position;
		self.line_nr = 1 + self.chars[..position].iter().filter(|ch| **ch == '\n').count();
		self.column = position - line_start + 1;
		self.current_line = self.chars[line_start..].iter().take_while(|ch| **ch != '\n').collect();
	}

	/// The rest of the line, or the indented lines below it, or a `{…}` block
	fn parse_definition_body(&mut self) -> Node {
		if self.current_char() == '{' {
			return self.parse_atom();
		}
		if !self.only_blanks_before_newline() {
			return self.parse_expr(0);
		}
		self.parse_indented_block().unwrap_or_else(|| error("a definition needs a body: `to name params: body`"))
	}

	/// Offside rule: the lines indented (by tabs or spaces) below a line ending in `:` are its `{…}` block;
	/// None, with nothing consumed, when the next line is not indented deeper
	fn parse_indented_block(&mut self) -> Option<Node> {
		let next_line = self.chars[self.pos..].iter().skip_while(|ch| **ch != '\n').skip(1);
		let indented_by_spaces = next_line.take_while(|ch| matches!(ch, ' ' | '\t')).any(|ch| *ch == ' ');
		let before_block = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let outer_counts_spaces = self.indent_counts_spaces;
		self.indent_counts_spaces |= indented_by_spaces;
		let (_, indent) = self.skip_whitespace();
		if indent <= self.base_indent {
			self.indent_counts_spaces = outer_counts_spaces;
			(self.pos, self.line_nr, self.column, self.current_line) = before_block;
			return None;
		}
		let outer_indent = std::mem::replace(&mut self.base_indent, indent);
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.indent_counts_spaces = outer_counts_spaces;
		self.seek_back_over_whitespace(); // the newline after the block still separates the next statement
		Some(curly_block(block))
	}

	/// Ruby/Lua `do … end`, `then … end`, `else … end`: an `end` closes the statements opened here,
	/// counting the `do`/`then` openers in between; an `end` that closes nothing leaves the one-statement body
	fn closing_end_follows(&self, openers: &[&str]) -> bool {
		let mut open = 1;
		let mut position = self.pos;
		while position < self.chars.len() {
			let ch = self.chars[position];
			if ch == '"' || ch == '\'' {
				position += 2 + self.chars[position + 1..].iter().position(|quoted| *quoted == ch).unwrap_or(self.chars.len());
				continue;
			}
			if !is_identifier_char(ch) {
				position += 1;
				continue;
			}
			let word_start = position;
			while position < self.chars.len() && is_identifier_char(self.chars[position]) {
				position += 1;
			}
			if word_start > 0 && self.chars[word_start - 1] == '.' {
				continue; // `range.end` is a property
			}
			let word: String = self.chars[word_start..position].iter().collect();
			match word.as_str() {
				END_KEYWORD => open -= 1,
				opener if openers.contains(&opener) => open += 1,
				_ => {}
			}
			if open == 0 {
				return true;
			}
		}
		false
	}

	/// The statements up to the closing `end` (consumed) as a `{…}` block; a `then` block also ends at its `else`
	fn parse_end_block(&mut self, stops_at_else: bool) -> Node {
		let outer_else = std::mem::replace(&mut self.stops_at_else, stops_at_else);
		let outer_end = std::mem::replace(&mut self.stops_at_end, true);
		let outer_indent = self.base_indent;
		let before_first_line = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let (_, indent) = self.skip_whitespace();
		(self.pos, self.line_nr, self.column, self.current_line) = before_first_line;
		self.base_indent = self.base_indent.max(indent); // its lines may be indented, the `end` line not
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.stops_at_else = outer_else;
		self.stops_at_end = outer_end;
		self.skip_whitespace();
		if self.matches_keyword(END_KEYWORD) {
			self.advance_by(END_KEYWORD.len());
		}
		curly_block(block)
	}

	/// The `end` (or, in a `then` block, the `else`) that closes the block being parsed
	fn at_block_close(&self) -> bool {
		self.stops_at_end && (self.matches_keyword(END_KEYWORD) || (self.stops_at_else && self.matches_keyword(ELSE_KEYWORD)))
	}

	fn parameters_follow_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == '('
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::versions::VERSION_KEYWORD || self.options.data_mode {
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

	fn parse_symbol_with_suffix(&mut self) -> Node {
		let version_len = crate::versions::tagged_literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + version_len].iter().collect();
			self.advance_by(version_len);
			return Node::Symbol(literal);
		}
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return error(&e),
		};

		if let Some(version) = self.version_operand(&symbol) {
			return version;
		}

		// `def square (n) {…}` names its function like `def square(n) {…}`
		let names_function = std::mem::replace(&mut self.after_function_keyword, is_function_keyword(&symbol) && !self.options.wit_mode && !self.options.data_mode);
		if names_function && self.parameters_follow_after_blanks() {
			self.skip_spaces();
		}

		if self.url_follows(&symbol) {
			return self.parse_url(symbol);
		}

		if let Some(constant) = check_constants(&symbol, self.options.data_mode) {
			return constant; // if true {} fall through :?
		}

		if let Some(declaration) = self.try_parse_operator_declaration(&symbol) {
			return declaration;
		}

		// Possessive: `p's name` is the field `p.name`
		if !self.options.data_mode && self.current_char() == '\'' && self.peek_char(1) == 's' && self.peek_char(2) == ' ' && self.is_identifier_start(3) {
			self.advance_by(3);
			return match self.parse_symbol() {
				Ok(field) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(field))),
				Err(message) => error(&message),
			};
		}

		// Optional type: `x:int?=ø`, `f(x:int?)`, the field `right? }` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?' && (self.ends_optional_type(self.peek_char(1), self.peek_char(2)) || self.closes_after_blanks(1)) {
			self.advance();
			return Symbol(format!("{symbol}?"));
		}

		if symbol == HEX_WORD && !self.options.data_mode {
			if let Some(number) = self.try_parse_hex_word() {
				return number;
			}
		}

		if symbol == TO_WORD && !self.options.data_mode && !self.options.wit_mode && self.word_starts_statement(symbol.len()) {
			if let Some(definition) = self.try_parse_to_definition() {
				return definition;
			}
		}

		// Handle "global" keyword: global name = value
		if symbol == "for" {
			if let Some(loop_node) = self.try_parse_for_in() {
				return loop_node;
			}
		}

		if symbol == "global" || symbol == "export" {
			if let Some(declaration) = self.try_parse_global_declaration(&symbol) {
				return declaration;
			}
		}

		// Handle "class"/"struct"/"type" keyword: class Name { fields }
		// But NOT type(x) which is a function call for type introspection
		if self.options.wit_mode && self.current_char() == '<' {
			return self.parse_type_application(symbol);
		}
		if !self.options.wit_mode && self.declares_type(&symbol) {
			return self.parse_type_declaration();
		}

		// `point {x:1}` with blanks constructs a declared type like the glued `point{x:1}` (open decision 41)
		if self.declared_types.contains(&symbol) && self.block_after_blanks() {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			let block = self.parse_bracketed('{');
			return Node::Key(Box::new(Symbol(symbol)), Op::None, Box::new(block));
		}

		self.parse_glued_suffix(symbol)
	}

	/// `http://…`, `file://…`: the scheme of a URL, the rest of which reads as one text
	fn url_follows(&self, symbol: &str) -> bool {
		URL_SCHEMES.contains(&symbol) && self.current_char() == ':' && self.peek_char(1) == '/' && self.peek_char(2) == '/'
	}

	fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}

	/// `class Name {…}`, `struct`, `type Name {…}`, `record Name {…}`; `class:"btn"`, `type:email` (html attributes),
	/// `x.class` and `type(x)` are no declarations
	fn declares_type(&self, symbol: &str) -> bool {
		let is_key = self.current_char() == ':' && self.peek_char(1) != '=';
		let start = self.pos.saturating_sub(symbol.chars().count());
		let is_field = start > 0 && self.chars[start - 1] == '.';
		!is_key && !is_field && (TYPE_DECLARATION_WORDS.contains(&symbol)
			|| (symbol == RECORD_WORD && self.name_and_block_follow())
			|| (symbol == "type" && self.current_char() != '('))
	}

	/// The name and the `{fields}` of a type declaration, after its keyword
	fn parse_type_declaration(&mut self) -> Node {
		self.skip_whitespace();
		let type_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		self.skip_whitespace();
		let body = if self.current_char() == '{' { Self::transform_fields_to_types(self.parse_bracketed('{')) } else { Empty };
		Node::Type { name: Box::new(Symbol(type_name)), body: Box::new(body) }
	}

	/// What is glued to a word: `name{…}`, `List<int>`, `p@unit`, `f(args)`, `f(params) {body}`; else the word itself
	fn parse_glued_suffix(&mut self, symbol: String) -> Node {
		let ch = self.current_char();
		match ch {
			'{' => {
				let block = self.parse_bracketed('{');
				// `point{x:1}` of a declared type constructs a point, `point:{x:1}` and any other `name{…}` stay data (D4)
				let op = if self.declared_types.contains(&symbol) { Op::None } else { Op::Colon };
				Node::Key(Box::new(Symbol(symbol)), op, Box::new(block))
			}
			'<' if !self.options.xml_mode && !self.options.data_mode && self.type_application_length().is_some() => {
				let length = self.type_application_length().expect("guarded");
				let arguments: String = (1..length - 1).map(|offset| self.peek_char(offset)).collect();
				if symbol == "list" && !arguments.contains(',') {
					let element_words: Vec<&str> = arguments.split(|c: char| c == '<' || c == '>' || c.is_whitespace()).filter(|word| !word.is_empty()).collect();
					set_hint_position(self.line_nr, self.column.saturating_sub(symbol.chars().count()));
					norm::list_type(ListTypeStyle::Generic, &element_words);
				}
				self.advance_by(length);
				Symbol(type_application_name(&symbol, &arguments))
			}
			'<' if !self.options.xml_mode && !self.peek_char(1).is_numeric() && self.peek_char(1) != '<' && names_a_type(&symbol) => {
				// Only treat as generic if immediately after a type name (no space)
				// and NOT followed by a number (that would be comparison: i<9); `it<k` and `a<b` compare
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			// `info@pannous.com` in data is one word; `p@unit` in code reads the meta key: `p.@unit`
			'@' if self.peek_char(1).is_alphabetic() && self.options.data_mode => {
				let rest: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c) || matches!(c, '@' | '.' | '-')).collect();
				let rest = rest.trim_end_matches(['.', '-']).to_string();
				self.advance_by(rest.chars().count());
				Symbol(format!("{symbol}{rest}"))
			}
			'@' if self.peek_char(1).is_alphabetic() => {
				self.advance();
				match self.parse_symbol() {
					Ok(key) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(format!("{ATTRIBUTE_MARK}{key}")))),
					Err(message) => error(&message),
				}
			}
			'(' => {
				// Parse arguments as a proper Node
				let args_node = self.parse_bracketed('(');
				self.skip_spaces(); // Only spaces, preserve newlines as statement separators

				// in a condition `if f(1, 2) {…}` the block is the body of the `if`, not of a definition of f
				if self.current_char() == '{' && !self.equals_compares && !self.in_for_header {
					// Function with body: name(params) { body }
					let body = self.parse_bracketed('{');
					let signature = Node::List(
						vec![Symbol(symbol), typed_parameters(args_node)],
						Bracket::Round,
						Separator::None,
					);
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else if symbol == PRINT_WORD {
					print_call(print_arguments(args_node))
				} else {
					// Function call: name(params) -> List([symbol, args...])
					let mut items = vec![Symbol(symbol)];
					match typed_parameters(args_node) {
						Node::List(args, _, _) => items.extend(args),
						Node::Empty => {}
						other => items.push(other),
					}
					Node::List(items, Bracket::Round, Separator::None)
				}
			}
			_ => Node::symbol(&symbol),
		}
	}

	/// Helper to advance by N characters
	fn advance_by(&mut self, n: usize) {
		for _ in 0..n {
			self.advance();
		}
	}

	/// Pratt parser: parse expression with given minimum binding power
	/// Handles prefix, infix, and suffix operators
	fn parse_expr(&mut self, min_bp: u8) -> Node {
		self.skip_spaces();

		// Step 1: Prefix (nud)
		let lhs = if let Some((op, chars)) = self.peek_negated_control_word() {
			self.advance_by(chars);
			self.skip_spaces();
			let rhs = self.parse_prefix_operand(op);
			negate_condition(self.finish_prefix(op, rhs))
		} else if let Some(marker) = self.peek_guard_word() {
			self.parse_guard(marker)
		} else if let Some(call) = self.try_parse_user_prefix() {
			call
		} else if let Some(statement) = self.try_parse_return() {
			statement
		} else if let Some(awaited) = self.try_parse_await() {
			awaited
		} else if let Some((op, chars)) = self.peek_prefix_operator() {
			let (prefix_line, prefix_column) = self.get_position();
			self.hint_operator(chars, true);
			self.advance_by(chars);
			self.skip_spaces();
			if chars == 1 && op == Op::Abs {
				self.parse_norm_bars()
			} else {
				let rhs = self.parse_prefix_operand(op);
				if op == Op::Hash {
					set_hint_position(prefix_line, prefix_column);
					norm::length_operator(&crate::normalize::operand_text(&rhs), false);
				}
				self.finish_prefix(op, rhs)
			}
		} else {
			self.parse_atom()
		};

		self.continue_expr(lhs, min_bp)
	}

	/// `await a + await b`: `await` takes its operand like a unary minus, so each task is awaited before the sum.
	/// `await(x)` (a call or a definition of a function `await`) and `await = …` stay as they are
	fn try_parse_await(&mut self) -> Option<Node> {
		if !self.matches_keyword(AWAIT_KEYWORD) {
			return None;
		}
		let before_keyword = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance_by(AWAIT_KEYWORD.len());
		let called = self.current_char() == '(';
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		if called || names_a_variable || matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0' | ')') {
			(self.pos, self.line_nr, self.column, self.current_line) = before_keyword;
			return None;
		}
		let operand = self.parse_expr(AWAIT_OPERAND_BP);
		Some(Node::List(vec![Symbol(AWAIT_KEYWORD.to_string()), operand], Bracket::None, Separator::Space))
	}

	/// `return` takes the whole expression after it: `return -1` is no subtraction from `return`
	fn try_parse_return(&mut self) -> Option<Node> {
		if !self.matches_keyword(RETURN_KEYWORD) {
			return None;
		}
		let before_keyword = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance_by(RETURN_KEYWORD.len());
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		// a bare `return` at the end of a statement returns ø: `if t == ø { return }`
		let value = if matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0') && !names_a_variable {
			Node::Empty
		} else if !self.at_body_start() || names_a_variable {
			(self.pos, self.line_nr, self.column, self.current_line) = before_keyword; // `return := …` as a name
			return None;
		} else {
			self.parse_expr(0)
		};
		Some(Node::List(vec![Symbol(RETURN_KEYWORD.to_string()), value], Bracket::None, Separator::Space))
	}

	/// The operator after `lhs` that the infix table does not hold, its width in characters and its binding power
	fn special_infix(&self, lhs: &Node) -> Option<(SpecialInfix, usize, (u8, u8))> {
		// `a mod b` is `a % b` (Euclidean, 0 ≤ r < |b|), `a rem b` the truncated remainder (sign of the dividend, as C)
		for (word, op) in [("mod", Op::Mod), ("rem", Op::Rem)] {
			if self.matches_keyword(word) {
				return Some((SpecialInfix::Keyword(op), word.len(), op.binding_power()));
			}
		}
		// Python floor division glued to its operand, `7//2` and `x//=2` (`x // note` stays a comment), and `7 div 2`;
		// `div{…}` and `div {…}` are the html tag, and data never computes
		let word_division = self.matches_keyword("div") && !self.options.data_mode && !self.word_opens_block("div");
		if word_division || self.at_floor_division() {
			let compound = !word_division && self.peek_char(2) == '=';
			let op = if compound { Op::Assign } else { Op::Div };
			return Some((SpecialInfix::FloorDivision { compound }, if compound || word_division { 3 } else { 2 }, op.binding_power()));
		}
		// the pipeline `xs |> f(a)` is the call `f(xs, a)`, `xs |> f` is `f(xs)` (F#, Elixir); it binds below arithmetic
		// and above comparison: `xs |> sum > 3` is `sum(xs) > 3`
		if self.current_char() == '|' && self.peek_char(1) == '>' {
			return Some((SpecialInfix::Pipeline, 2, PIPELINE_BINDING_POWER));
		}
		// element-wise arithmetic `xs .+ 4` maps the operator over the list (D3: `xs + 4` asks, `xs + [4]` joins)
		if let Some(op) = self.element_wise_operator() {
			return Some((SpecialInfix::ElementWise(op), 2, op.binding_power()));
		}
		// membership `x in xs` binds like `==`, kept as the list [x in xs]: `if "Z" in v {…}`, `found = x in xs`;
		// a unit word stays in the flat counting phrase: `number of chars in t`
		let counts_units = matches!(lhs.drop_meta(), Node::Symbol(word) if crate::analyzer::text_unit(word).is_some());
		if self.matches_keyword(IN_KEYWORD) && !counts_units {
			return Some((SpecialInfix::Membership, IN_KEYWORD.len(), Op::Eq.binding_power()));
		}
		None
	}

	/// The right operand of a special infix operator, None when it binds looser than `min_bp` (nothing consumed)
	fn special_operand(&mut self, width: usize, (left, right): (u8, u8), min_bp: u8) -> Option<Node> {
		if left < min_bp {
			return None;
		}
		self.advance_by(width);
		self.skip_whitespace();
		Some(self.parse_expr(right))
	}

	/// The infix and suffix operators after an already parsed left operand, binding tighter than `min_bp`
	fn continue_expr(&mut self, mut lhs: Node, min_bp: u8) -> Node {
		const ARGUMENT_BP: u8 = 140; // a braceless argument takes arithmetic, stops at ranges and comparisons: f 3-1 > 5
		const MAX_BP_FOR_APPLICATION: u8 = 151; // operand of + - * / takes a braceless call: 1 + f 3
		const SUBSCRIPT_BP: u8 = 170; // Matches Op::Hash
		// Right operand of the last comparison, to chain a<b<c into a<b and b<c
		let mut previous_comparand: Option<Node> = None;
		loop {
			self.skip_spaces_and_inline_comments(); // not newlines: they are separators

			// Step 2: Suffix (led)
			// ASSUMPTION P48: a declared suffix operator wins over the built-in one of the same glyph (`suffix operator ³`)
			if let Some(updated) = self.try_parse_user_suffix(&lhs, min_bp).or_else(|| self.try_parse_suffix(&lhs, min_bp)) {
				lhs = updated;
				continue;
			}
			if let Some(updated) = self.try_parse_user_infix(&lhs, min_bp) {
				lhs = updated;
				continue;
			}

			// Step 2b: Subscript (tight, like Op::Hash)
			if let Some(updated) = self.try_parse_subscript(&lhs, min_bp, SUBSCRIPT_BP) {
				lhs = updated;
				continue;
			}

			if let Some(updated) = self.try_parse_evaluate_bang(&lhs)
				.or_else(|| self.try_parse_control_suffix(&lhs, min_bp))
				.or_else(|| self.try_parse_test_word(&lhs, min_bp))
				.or_else(|| self.try_parse_nand(&lhs, min_bp))
				.or_else(|| self.try_parse_elvis(&lhs, min_bp)) {
				lhs = updated;
				continue;
			}

			// Step 3a–e: the operators the infix table does not hold (`mod`, `//`, `|>`, `.+`, `in`, see SpecialInfix)
			if let Some((infix, width, binding)) = self.special_infix(&lhs) {
				let Some(operand) = self.special_operand(width, binding, min_bp) else { break };
				lhs = infix.applied(lhs, operand);
				previous_comparand = None;
				continue;
			}

			// Step 3: Check for infix operator
			let (op, chars) = match self.peek_operator() {
				Some(pair) => pair,
				None if self.stops_at_else && self.at_else_if_word().is_some() => break, // `if c: x elif d: y`
				None if self.at_block_close() => break, // `do x end`: the `end` is no argument of x
				None => {
					// Step 3b: Check for implicit function application (space between atoms)
					// This makes `x = f y` parse as `x = (f y)` and `252 > f y` as `252 > (f y)`
					// Only apply when:
					// - min_bp > 0 (not at top level)
					// - min_bp <= 130 (inside assignment/comparison/arithmetic)
					// - lhs is a Symbol or a function call (List with Bracket::None)
					// - NOT parenthesized expressions (List with Bracket::Round)
					// - Argument conditions vary by context:
					//   - In assignment (min_bp <= 60): allow ANY argument (including identifiers)
					//   - In colon/comparison (min_bp > 60): only non-identifier args
					//   This allows `fetch url` but prevents `a: b c d` from chaining
					if let Some(updated) = self.try_parse_implicit_application(
						&lhs,
						min_bp,
						ARGUMENT_BP,
						MAX_BP_FOR_APPLICATION,
					) {
						lhs = updated;
						continue;
					}
					break;
				}
			};

			let (l_bp, r_bp) = op.binding_power();

			// Stop if operator binds less tightly than our minimum
			if l_bp < min_bp || (op == Op::Else && self.stops_at_else) {
				break;
			}
			if matches!(op, Op::Add | Op::Sub) {
				match self.signed_number_starts_a_list(&lhs) {
					Ok(false) => {}
					Ok(true) => {
						if op == Op::Add {
							self.advance_by(1); // `+1` is the element 1
						}
						break; // `1 -1`: the signed number is the next element
					}
					Err(strict) => {
						lhs = strict;
						break;
					}
				}
			}

			// Consume the operator
			let written = self.hint_operator(chars, false);
			let (op_line, op_column) = self.get_position();
			let bare_symbol = Some(self.current_char()).filter(|symbol| chars == 1 && matches!(symbol, '&' | '|'));
			self.advance_by(chars);
			let block_body = match op {
				Op::Colon if self.only_blanks_before_newline() => {
					self.with_equals_comparing(false, |parser| parser.parse_indented_block()) // the block of `if c:` assigns
				}
				Op::Do | Op::Then | Op::Else if self.closing_end_follows(&END_BLOCK_OPENERS) => Some(self.parse_end_block(op == Op::Then)),
				Op::Do if self.closing_end_follows(&["do"]) => Some(error(AMBIGUOUS_END)), // `do a; if c then b end`
				_ => None,
			};
			if block_body.is_none() {
				self.skip_whitespace();
			}
			if op == Op::Define {
				if let Some(name) = defined_function_name(&lhs) {
					if matches!(lhs.drop_meta(), Node::List(..)) {
						self.functions_with_parameters.insert(name.clone());
					}
					self.functions.insert(name);
				}
			}

			// Parse right-hand side with appropriate binding power
			if op == Op::Colon {
				self.equals_compares = false; // in `if c: x=1` the colon ends the condition, the rest is the body
			}
			let rhs = block_body.unwrap_or_else(|| self.parse_expr(r_bp));

			if op == Op::Define && !matches!(lhs.drop_meta(), Node::List(..)) && !mentions(&rhs, "it") {
				self.functions.remove(&lhs.name()); // `x := 5` defines a value, not a function
			}

			if let Some(symbol) = bare_symbol {
				if let Some(diagnostic) = logic_mixed_with_comparison(&lhs, symbol, &rhs) {
					lhs = Diagnostic { line: op_line, column: op_column, ..diagnostic }.into_error();
					previous_comparand = None;
					continue;
				}
			}

			let op = match self.range_reading(op, &written, &rhs, op_line, op_column) {
				Ok(op) => op,
				Err(unanswered) => {
					lhs = unanswered;
					previous_comparand = None;
					continue;
				}
			};

			if op == Op::Hash {
				crate::normalize::set_position_of(&lhs);
				norm::index_operator(&crate::normalize::operand_text(&lhs), &crate::normalize::operand_text(&rhs), false);
			}

			if op.is_equality() && ungrouped_equality(&lhs) {
				lhs = Diagnostic { line: op_line, column: op_column, ..chained_equality(&lhs, op, &rhs) }.into_error();
				previous_comparand = None;
				continue;
			}

			let rhs = if op == Op::Eq && written != IS_WORD { crate::type_tests::equality_operand(rhs) } else { rhs };
			let op = if op == Op::Assign && is_function_block(&lhs, &rhs) {
				self.functions.insert(lhs.name());
				Op::Define
			} else {
				op
			};
			lhs = match previous_comparand.take() {
				Some(middle) if op.is_ordering() => {
					let next_comparison = Node::Key(Box::new(middle), op, Box::new(rhs.clone()));
					Node::Key(Box::new(lhs), Op::And, Box::new(next_comparison))
				}
				_ => Node::Key(Box::new(lhs), op, Box::new(rhs.clone())),
			};
			if op.is_ordering() {
				previous_comparand = Some(rhs);
			}
		}

		lhs
	}

	/// `‖x‖` brackets a whole expression like parentheses: `‖3-5‖*2` → 4
	fn parse_norm_bars(&mut self) -> Node {
		let inner = self.parse_expr(0);
		self.skip_spaces();
		if self.current_char() != '‖' {
			return error("Missing closing ‖");
		}
		self.advance();
		self.finish_prefix(Op::Abs, inner)
	}

	/// Operand of a prefix operator; `++i` binds like `i++`, conditions compare with `=`
	fn parse_prefix_operand(&mut self, op: Op) -> Node {
		let (left_bp, right_bp) = op.binding_power();
		match op {
			Op::Inc | Op::Dec => self.parse_expr(left_bp),
			Op::If | Op::While => self.with_equals_comparing(true, |parser| parser.parse_expr(right_bp)),
			// `#m#1` counts `m#1`: indexing a count is never meant
			Op::Hash => self.parse_expr(left_bp - 1),
			_ => self.parse_expr(right_bp),
		}
	}

	fn with_equals_comparing<T>(&mut self, compares: bool, parse: impl FnOnce(&mut Self) -> T) -> T {
		let outer = std::mem::replace(&mut self.equals_compares, compares);
		let node = parse(self);
		self.equals_compares = outer;
		node
	}

	fn finish_prefix(&mut self, op: Op, rhs: Node) -> Node {
		match (op, rhs.drop_meta()) {
			(Op::If, _) => self.finish_if_prefix(rhs),
			(Op::While, _) => self.finish_while_prefix(rhs),
			(Op::Neg, Node::Number(number)) => Node::Number(-*number),
			(Op::Inc | Op::Dec, _) => Node::Key(Box::new(rhs), op, Box::new(Empty)), // ++i is i++: increment is immediate
			_ => Node::Key(Box::new(Empty), op, Box::new(rhs)),
		}
	}

	fn finish_if_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let then_block = self.parse_atom(); // parse { block }
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		if let Node::Key(cond, Op::Colon, then_expr) = &rhs {
			let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
			// the body runs to the end of the statement, `if c: x+=1`, but not into the `else`
			let outer = std::mem::replace(&mut self.stops_at_else, true);
			let then_expr = self.continue_expr(then_expr.as_ref().clone(), 0);
			self.stops_at_else = outer;
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		if let Some((condition, then_block)) = split_trailing_block(&rhs, false) {
			self.skip_spaces();
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(condition));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		// `if (x < 0) return -1`: C's parenthesized condition, then the statement on the same line
		let is_grouped = matches!(rhs.drop_meta(), Node::List(_, Bracket::Round, _));
		if is_grouped && self.at_body_start() && !self.matches_keyword("then") && !self.matches_keyword("else") {
			let outer = std::mem::replace(&mut self.stops_at_else, true);
			let then_expr = self.parse_expr(0);
			self.stops_at_else = outer;
			let if_then = Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(rhs))), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		Node::Key(Box::new(Empty), Op::If, Box::new(rhs))
	}

	/// A statement body follows: not a separator, closing bracket, end of input or the `do` keyword
	fn at_body_start(&self) -> bool {
		!matches!(self.current_char(), '\0' | ';' | ',' | '\n' | '}' | ')' | ']') && !self.matches_keyword("do")
	}

	/// `global [modifiers] name[=value]` and the same after `export`: a variable declared at module level.
	/// `export` without a name after it, and `export f:=it*2`, a function, stay ordinary code.
	fn try_parse_global_declaration(&mut self, keyword: &str) -> Option<Node> {
		let before_declaration = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.skip_whitespace();
		self.skip_declaration_modifiers();
		if keyword == "export" && !self.at_identifier_start() {
			(self.pos, self.line_nr, self.column, self.current_line) = before_declaration;
			return None;
		}
		let declaration = self.parse_expr(0); // name, name=value or name:=value
		let defines_function = matches!(declaration.drop_meta(), Node::Key(name, Op::Define, _) if self.functions.contains(&name.name()));
		if keyword == "export" && defines_function {
			return Some(declaration);
		}
		Some(Node::Key(Box::new(Symbol("global".to_string())), Op::Colon, Box::new(declaration)))
	}

	fn at_identifier_start(&self) -> bool {
		self.current_char().is_alphabetic() || self.current_char() == '_'
	}

	/// Modifier and type words between the keyword and the name (`global const int k=7`): the global holds the value, the words are dropped
	fn skip_declaration_modifiers(&mut self) {
		loop {
			let before_word = (self.pos, self.line_nr, self.column, self.current_line.clone());
			let word = self.parse_symbol().unwrap_or_default();
			let is_modifier = DECLARATION_MODIFIERS.contains(&word.as_str()) || crate::analyzer::CONSTANT_KEYWORDS.contains(&word.as_str()) || crate::analyzer::type_word_kind(&word).is_some();
			self.skip_spaces();
			if !is_modifier || !self.at_identifier_start() {
				(self.pos, self.line_nr, self.column, self.current_line) = before_word; // `global int` names the variable int
				return;
			}
		}
	}

	/// The name of `for x in …`, or the names each element destructures into: `for (r, c) in …`, `for k, v in …`
	fn parse_loop_variable(&mut self) -> Option<Node> {
		let bracket = if self.current_char() == '(' { self.advance(); Bracket::Round } else { Bracket::None };
		let mut names = vec![];
		loop {
			self.skip_spaces();
			names.push(Symbol(self.parse_symbol().ok()?));
			self.skip_spaces();
			if self.current_char() != ',' {
				break;
			}
			self.advance();
		}
		if bracket == Bracket::Round {
			if self.current_char() != ')' {
				return None;
			}
			self.advance();
		}
		Some(if names.len() == 1 { names.remove(0) } else { Node::List(names, bracket, Separator::Colon) })
	}

	/// `for x in iterable: body` and `for x in iterable {body}`, after the word `for`; the body of a colon runs to the end of
	/// the statement. `for 1..10 : print it` names no variable (wiki/for.md): `for iterable {body}`, whose items are `it`
	fn try_parse_for_in(&mut self) -> Option<Node> {
		let before_header = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.skip_spaces();
		let variable = self.parse_loop_variable();
		self.skip_spaces();
		let named = variable.filter(|_| self.matches_keyword("in"));
		if named.is_some() {
			self.advance_by("in".len());
		} else {
			(self.pos, self.line_nr, self.column, self.current_line) = before_header.clone();
			self.skip_spaces();
			if self.current_char() == '(' { // `for(i=0;i<n;i++)`
				(self.pos, self.line_nr, self.column, self.current_line) = before_header;
				return None;
			}
		}
		let outer_header = std::mem::replace(&mut self.in_for_header, true);
		let iterable = self.with_equals_comparing(false, |parser| parser.parse_expr(Op::Colon.binding_power().0 + 1));
		self.in_for_header = outer_header;
		self.skip_spaces();
		let body_word = if self.current_char() == ':' { Some(":") } else { Some("do").filter(|word| self.matches_keyword(word)) };
		let Some(variable) = named else {
			// only the colon and do forms: `for 1..3 {…}` is read where for loops are lowered
			let Some(word) = body_word else {
				(self.pos, self.line_nr, self.column, self.current_line) = before_header;
				return None;
			};
			let body = self.colon_body(word);
			let block = Node::List(vec![body], Bracket::Curly, Separator::Semicolon);
			return Some(Node::List(vec![Symbol("for".to_string()), iterable, block], Bracket::None, Separator::Space));
		};
		let body = match body_word {
			Some(word) => self.colon_body(word), // `for i in 0..n: body`, `for i in 0..n do body`
			None => self.parse_atom(),
		};
		// `for chars in text: print it`: a unit word walks the text by that unit, the item is `it`
		if let Some((unit, iterable)) = unit_iteration(&variable, &iterable, &body) {
			return Some(Node::List(vec![Symbol("for".to_string()), unit, Symbol("in".to_string()), iterable, body], Bracket::None, Separator::Space));
		}
		Some(Node::List(vec![Symbol("for".to_string()), variable, Symbol("in".to_string()), iterable, body], Bracket::None, Separator::Space))
	}

	/// The body after `:` or `do`: an indented block under a colon at the end of the line, else the rest of the line
	fn colon_body(&mut self, word: &str) -> Node {
		self.advance_by(word.len());
		let indented_block = if word == ":" && self.only_blanks_before_newline() { self.parse_indented_block() } else { None };
		indented_block.unwrap_or_else(|| self.rest_of_statement())
	}

	/// The words up to the end of the line, one expression: the colon body `print i` of `for i in 1..3 : print i`
	fn rest_of_statement(&mut self) -> Node {
		let mut words = vec![self.with_equals_comparing(false, |parser| parser.parse_expr(0))];
		loop {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			if !self.can_start_atom() {
				return one_expression(&words);
			}
			words.push(self.with_equals_comparing(false, |parser| parser.parse_expr(0)));
		}
	}

	fn finish_while_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let body_block = self.parse_atom(); // parse { block }
			return while_do(rhs, body_block);
		}

		if let Node::Key(condition, Op::Colon, body) = &rhs {
			let body = self.continue_expr(body.as_ref().clone(), 0); // `while c: i+=2`
			return while_do(condition.as_ref().clone(), body);
		}

		if matches!(rhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1) && self.at_body_start() {
			let body = self.parse_expr(0); // `while (i<9) i++`
			return while_do(rhs, body);
		}

		if let Some((condition, body)) = split_trailing_block(&rhs, true) { // `{}` parses as ø
			return while_do(condition, body);
		}

		Node::Key(Box::new(Empty), Op::While, Box::new(rhs))
	}

	/// `else body`, and `else if` / `elif` / `elsif` / `elseif` chaining a whole new `if … {…} else …`
	fn parse_optional_else(&mut self, if_then: Node, mode: ElseParseMode) -> Node {
		self.skip_spaces();
		// `}` then `else …` on the next line belongs to this if
		if self.else_on_a_later_line() {
			while matches!(self.current_char(), ' ' | '\t' | '\n' | '\r') {
				self.advance();
			}
		}
		let else_expr = if let Some(word) = self.at_else_if_word() {
			self.advance_by(word.len());
			self.parse_else_if()
		} else if self.matches_keyword("else") {
			self.advance_by("else".len());
			self.skip_spaces();
			match mode {
				_ if self.matches_keyword("if") => {
					self.advance_by("if".len());
					self.parse_else_if()
				}
				_ if self.current_char() == ':' => {
					self.advance(); // Python's `else: body`
					self.parse_expr(0)
				}
				ElseParseMode::Atom => self.parse_atom(),
				ElseParseMode::Expr => self.parse_expr(0),
			}
		} else {
			return if_then;
		};
		Node::Key(Box::new(if_then), Op::Else, Box::new(else_expr))
	}

	/// Do line breaks and blanks, then the word `else`, follow the cursor
	fn else_on_a_later_line(&self) -> bool {
		let gap = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t' | '\n' | '\r')).count();
		let crosses_line = (0..gap).any(|at| self.peek_char(at) == '\n');
		let word = ELSE_KEYWORD.chars().enumerate().all(|(i, c)| self.peek_char(gap + i) == c) && !is_identifier_char(self.peek_char(gap + ELSE_KEYWORD.len()));
		crosses_line && word
	}

	fn at_else_if_word(&self) -> Option<&'static str> {
		ELSE_IF_WORDS.into_iter().find(|word| self.matches_keyword(word))
	}

	fn parse_else_if(&mut self) -> Node {
		let condition = self.parse_prefix_operand(Op::If);
		self.finish_if_prefix(condition)
	}

	/// The exponent written in superscript digits and signs at the cursor, its length in characters and whether it has a sign:
	/// ⁴ → (4, 1), ¹² → (12, 2), ⁻¹ → (-1, 2), ²⁺³ → (5, 3)
	fn superscript_exponent(&self) -> Option<(i64, usize, bool)> {
		let (mut exponent, mut length, mut signed) = (0i64, 0usize, false);
		loop {
			let sign = match self.chars.get(self.pos + length).copied().and_then(superscript_sign) {
				Some(sign) => sign,
				None if length == 0 => 1,
				None => break,
			};
			let sign_length = self.chars.get(self.pos + length).copied().and_then(superscript_sign).map_or(0, |_| 1);
			let digits: Vec<i64> = self.chars.iter().skip(self.pos + length + sign_length).map_while(|ch| superscript_digit(*ch)).collect();
			if digits.is_empty() {
				break;
			}
			let run = digits.iter().fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit));
			exponent = exponent.saturating_add(sign * run);
			signed |= sign_length > 0;
			length += sign_length + digits.len();
		}
		(length > 0).then_some((exponent, length, signed))
	}

	/// `x⁴` is x^4, `x⁻¹` is 1/x; the single digits ² and ³ keep their dedicated square and cube operators
	fn try_parse_superscript_power(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let (exponent, length, signed) = self.superscript_exponent()?;
		let (op, right) = match (exponent, length, signed) {
			(2, 1, false) => (Op::Square, Empty),
			(3, 1, false) => (Op::Cube, Empty),
			_ => (Op::Pow, Node::int(exponent.abs())),
		};
		if op.binding_power().0 < min_bp {
			return None;
		}
		self.advance_by(length);
		let power = Node::Key(Box::new(lhs.clone()), op, Box::new(right));
		Some(if exponent < 0 { Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power)) } else { power })
	}

	fn try_parse_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some(power) = self.try_parse_superscript_power(lhs, min_bp) {
			return Some(power);
		}
		let (op, chars) = self.peek_suffix_operator()?;
		let (l_bp, _) = op.binding_power();
		if l_bp < min_bp {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(lhs.clone()), op, Box::new(Empty)))
	}

	/// `{a*a}!` and `f!` at the end of a statement evaluate the block or name, which a block does on the spot anyway.
	/// After a function or method the `!` mutates in place (user decision D2, by position): `x.upper!` and `upper(x)!`
	/// assign the result back to x; in `upper x!` the name is marked for crate::mutation to do the same.
	fn try_parse_evaluate_bang(&mut self, lhs: &Node) -> Option<Node> {
		let mutated = crate::mutation::mutated_variable(lhs).cloned();
		let is_evaluable = matches!(lhs.drop_meta(), Node::Symbol(_) | Node::List(_, Bracket::Curly, _));
		// `name! email?`: glued to its name and followed by a space, the `!` is a suffix even when an operand follows
		// `x!+1`: glued to its name and followed by an infix operator, the `!` is a suffix too
		let glued_suffix = !self.prev_char().is_whitespace() && (matches!(self.peek_char(1), ' ' | '\t') || INFIX_AFTER_BANG.contains(&self.peek_char(1)));
		if self.current_char() != '!' || self.peek_char(1) == '=' || (self.operand_follows(1) && !glued_suffix) || !(is_evaluable || mutated.is_some()) {
			return None;
		}
		self.advance();
		Some(match (mutated, lhs.drop_meta()) {
			(Some(variable), _) => Node::Key(Box::new(variable), Op::Assign, Box::new(lhs.clone())),
			(None, Node::Symbol(_)) => crate::mutation::marked(lhs.clone()),
			(None, _) => lhs.clone(),
		})
	}

	/// `x empty`, `x missing`, `x is absent` … at the end of a condition are `not x` (wiki/null.md); `x failed` is `is_error(x)`.
	/// The word must end the condition: a block, colon, `then`/`else`/`and`/`or` or the end of the statement follows.
	/// `x is empty` stays the comparison with ø.
	fn try_parse_test_word(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if self.options.data_mode || min_bp > Op::Not.binding_power().1 || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		let is_length = "is".len();
		let after_is = self.matches_keyword("is");
		let word_start = if after_is { is_length + (is_length..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count() } else { 0 };
		let word = TEST_WORDS.iter().find(|word| word.chars().enumerate().all(|(index, letter)| self.peek_char(word_start + index) == letter) && !is_identifier_char(self.peek_char(word_start + word.len())))?;
		if after_is && *word == EMPTY_WORD {
			return None;
		}
		let end = word_start + word.len();
		let blanks = (end..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let next = self.peek_char(end + blanks);
		let word_ends_condition = !self.operand_follows(end) || matches!(next, '{' | ':') || CONDITION_FOLLOWERS.iter().any(|follower| {
			follower.chars().enumerate().all(|(index, letter)| self.peek_char(end + blanks + index) == letter) && !is_identifier_char(self.peek_char(end + blanks + follower.len()))
		});
		if !word_ends_condition {
			return None;
		}
		self.advance_by(end);
		if *word == FAILED_WORD {
			return Some(Node::List(vec![Symbol(IS_ERROR_CALL.to_string()), lhs.clone()], Bracket::Round, Separator::None));
		}
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(lhs.clone())))
	}

	/// A control word behind a statement: `x++ while c`, `a = 2 if c`, `i++ until c`, `a = 2 unless c`, `3 times {body}`
	fn try_parse_control_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if self.options.data_mode || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		if (min_bp <= TIMES_BP || self.times_fills_list()) && self.matches_keyword(TIMES_WORD) {
			self.advance_by(TIMES_WORD.len());
			return Some(self.parse_times_loop(lhs.clone()));
		}
		let (word, guard, negated) = STATEMENT_MODIFIERS.iter().copied().find(|(word, _, _)| self.matches_keyword(word))?;
		if min_bp > 0 {
			return None;
		}
		self.advance_by(word.len());
		self.skip_spaces();
		let condition = self.with_equals_comparing(true, |parser| parser.parse_expr(Op::If.binding_power().1));
		if guard == Op::While {
			// a trailing `while` tests before the first round like the leading one; it is no do-while (user decision #22)
			let (statement, test) = (crate::normalize::operand_text(lhs), crate::normalize::operand_text(&condition));
			crate::normalize::hint(&format!("{statement} {word} {test}"), &format!("{word} {test} {{ {statement} }}"),
				"a trailing loop word tests before the first round, the statement may never run");
		}
		let condition = if negated { Node::Key(Box::new(Empty), Op::Not, Box::new(condition)) } else { condition };
		Some(match guard {
			Op::While => while_do(condition, lhs.clone()),
			_ => Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(condition))), Op::Then, Box::new(lhs.clone())),
		})
	}

	/// `a nand b` and `a ¬& b` are `not (a and b)`
	fn try_parse_nand(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let spelling = NAND_SPELLINGS.iter().find(|spelling| self.matches_operator_word(spelling))?;
		let (and_bp, right_bp) = Op::And.binding_power();
		if and_bp < min_bp || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		self.set_hint_pos();
		norm::operator(spelling, false);
		self.advance_by(spelling.chars().count());
		self.skip_spaces();
		let rhs = self.parse_expr(right_bp);
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(Node::Key(Box::new(lhs.clone()), Op::And, Box::new(rhs)))))
	}

	/// The word or glyph pair `spelling` at the cursor; a word must not run into an identifier
	fn matches_operator_word(&self, spelling: &str) -> bool {
		let matches = spelling.chars().enumerate().all(|(index, letter)| self.peek_char(index) == letter);
		matches && (!spelling.chars().all(char::is_alphabetic) || !is_identifier_char(self.peek_char(spelling.chars().count())))
	}

	/// `a ?: b` is `a ? a : b`; a left side that is not a plain name is evaluated once, into a hidden variable
	fn try_parse_elvis(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let (question_bp, _) = Op::Question.binding_power();
		if self.current_char() != '?' || self.peek_char(1) != ':' || question_bp < min_bp {
			return None;
		}
		self.advance_by(2);
		self.skip_whitespace();
		let alternative = self.parse_expr(question_bp + 1); // left-assoc: `a ?: b ?: c` is `(a ?: b) ?: c`, the same value
		let elvis = |value: Node| Node::Key(Box::new(value.clone()), Op::Question, Box::new(Node::Key(Box::new(value), Op::Colon, Box::new(alternative))));
		if matches!(lhs.drop_meta(), Node::Symbol(_)) {
			return Some(elvis(lhs.clone()));
		}
		self.elvis_operands += 1;
		let operand = Symbol(format!("{ELVIS_WORD}·{}", self.elvis_operands));
		let store = Node::Key(Box::new(operand.clone()), Op::Assign, Box::new(lhs.clone()));
		Some(Node::List(vec![store, elvis(operand)], Bracket::Round, Separator::Semicolon))
	}

	/// `hex 1010` is `0x1010`; the digits start with a number, `hex ff` stays a call of `hex` on `ff`
	fn try_parse_hex_word(&mut self) -> Option<Node> {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		if blanks == 0 || !self.peek_char(blanks).is_ascii_digit() {
			return None;
		}
		let end = (blanks..).find(|&offset| !self.peek_char(offset).is_ascii_hexdigit())?;
		if is_identifier_char(self.peek_char(end)) {
			return None;
		}
		let digits: String = (blanks..end).map(|offset| self.peek_char(offset)).collect();
		self.advance_by(end);
		Some(i64::from_str_radix(&digits, 16).map(Node::int).unwrap_or_else(|_| error(&format!("Invalid hex: hex {digits}"))))
	}

	/// `times [x]` fills a list, binding to the count right before it: `xs = 100 times [0]`
	fn times_fills_list(&self) -> bool {
		let after_word = TIMES_WORD.len();
		let blanks = (after_word..).take_while(|&offset| self.peek_char(offset) == ' ').count();
		self.peek_char(after_word + blanks) == '['
	}

	/// `N times {body}` and `N times: body` count with a hidden variable of its own, so nested loops do not meet
	fn parse_times_loop(&mut self, count: Node) -> Node {
		self.skip_spaces();
		let body = match self.current_char() {
			'{' => self.parse_atom(),
			':' => {
				self.advance();
				self.with_equals_comparing(false, |parser| parser.parse_expr(0))
			}
			'[' => {
				let list = self.parse_atom();
				return crate::analyzer::filled_list(count, &list).unwrap_or_else(|| error("`n times [x]` repeats one element: `3 times [0]`"));
			}
			// `3 times "ab"`, `3 times greeting`: the text repeated (a non-text is an error where its kind is known)
			quote_or_letter if matches!(quote_or_letter, '"' | '\'') || self.is_identifier_start(0) => return Node::List(vec![Symbol(TEXT_TIMES.to_string()), count, self.parse_atom()], Bracket::Round, Separator::None),
			_ => return error("`times` needs a body: `3 times {…}`"),
		};
		self.times_loops += 1;
		let counter = Symbol(format!("{TIMES_WORD}·{}", self.times_loops));
		// the count is evaluated once, before the first round (user decision #22): the body may change what it reads
		let (count_binding, count) = match count.drop_meta() {
			Node::Number(_) => (None, count),
			_ => {
				let held = Symbol(format!("{TIMES_WORD}·count·{}", self.times_loops));
				(Some(Node::Key(Box::new(held.clone()), Op::Assign, Box::new(count))), held)
			}
		};
		let zero_to_count = Node::Key(Box::new(Node::Number(Number::Int(0))), Op::Range, Box::new(count));
		let rounds = Node::List(vec![Symbol("for".to_string()), counter, Symbol("in".to_string()), zero_to_count, body], Bracket::None, Separator::Space);
		match count_binding {
			Some(binding) => Node::List(vec![binding, rounds], Bracket::Round, Separator::Semicolon),
			None => rounds,
		}
	}

	fn try_parse_subscript(&mut self, lhs: &Node, min_bp: u8, subscript_bp: u8) -> Option<Node> {
		// a space before the bracket makes it a list, `f [1, 2]`; only `xs[1]` indexes
		if self.current_char() != '[' || min_bp > subscript_bp || self.prev_char().is_whitespace() || is_unindexable_keyword(lhs) {
			return None;
		}
		if let Some(list_type) = self.try_parse_array_type_suffix(lhs) {
			return Some(list_type);
		}
		let before_bracket = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance(); // skip '['
		self.skip_whitespace();

		let mut indices = vec![self.parse_slice_from_start().unwrap_or_else(|| self.parse_expr(0))];
		self.skip_whitespace();

		while self.current_char() == ',' {
			self.advance(); // skip ','
			self.skip_whitespace();
			indices.push(self.parse_expr(0));
			self.skip_whitespace();
		}

		if self.current_char() != ']' {
			(self.pos, self.line_nr, self.column, self.current_line) = before_bracket; // `foo [1 2 3]`: a list argument, not an index
			return None;
		}
		self.advance(); // skip ']'
		// `int[n]` is n zeros of the type unless int is a variable (analyzer lower_declarations): no indexing hint
		let names_a_type = matches!(lhs.drop_meta(), Node::Symbol(word) if crate::analyzer::zero_list(Empty, word).is_some());
		if slice_bounds(&indices[0]).is_none() && !names_a_type {
			crate::normalize::set_position_of(lhs);
			norm::index_operator(&crate::normalize::operand_text(lhs), &crate::normalize::operand_text(&indices[0]), true);
		}

		Some(indices.into_iter().fold(lhs.clone(), subscript))
	}

	/// `[:end]` and `[:]`, a slice from the start: `ø:end`
	fn parse_slice_from_start(&mut self) -> Option<Node> {
		if self.current_char() != ':' {
			return None;
		}
		self.advance(); // skip ':'
		self.skip_whitespace();
		let end = if self.current_char() == ']' { Empty } else { self.parse_expr(0) };
		Some(Node::Key(Box::new(Empty), Op::Colon, Box::new(end)))
	}

	/// The Java/C array type `int[]` is the list type `[int]`
	fn try_parse_array_type_suffix(&mut self, lhs: &Node) -> Option<Node> {
		let Node::Symbol(word) = lhs.drop_meta() else { return None };
		if !is_identifier_char(self.prev_char()) || self.peek_char(1) != ']' || !names_a_type(word) {
			return None;
		}
		self.advance_by(2);
		Some(Node::List(vec![lhs.clone()], Bracket::Square, Separator::Space))
	}

	fn try_parse_implicit_application(
		&mut self,
		lhs: &Node,
		min_bp: u8,
		argument_bp: u8,
		max_bp_for_application: u8,
	) -> Option<Node> {
		let lhs_is_callable = match lhs.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(_, Bracket::None, _) => true, // Function call from implicit application
			_ => false,
		};
		let lhs_is_defined_function = matches!(lhs.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		let ch = self.current_char();
		let in_assignment_context = min_bp <= 60; // Assignment r_bp is 59
		let arg_is_non_identifier = ch.is_numeric()
			|| self.number_starts_at(0)
			|| ch == '"'
			|| ch == '\''
			|| ch == '('
			|| ch == '['
			|| ch == '{'
			|| ch == '-'
			|| self.starts_function_reference();
		let should_apply = in_assignment_context || arg_is_non_identifier || lhs_is_defined_function;

		// At statement level a list `f a b` is a call with all its items; a function of the implicit `it` takes one argument,
		// so `f 3-1 > 15` compares `f(3-1)` just like the operand `1 + f 3-1 > 15` does
		let takes_one_argument = lhs_is_defined_function && !self.functions_with_parameters.contains(&lhs.name());
		if (min_bp == 0 && !takes_one_argument) || min_bp > max_bp_for_application {
			return None;
		}
		if !lhs_is_callable || !self.can_start_atom() || !should_apply || (ch == '{' && self.in_for_header) {
			return None;
		}

		let arg = self.parse_expr(argument_bp);
		if arg == Empty {
			return None;
		}

		Some(Node::List(
			vec![lhs.clone(), arg],
			Bracket::None,
			Separator::Space,
		))
	}

	/// Parse a complete value/expression - calls parse_expr(0) for operator chaining
	fn parse_value(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();

		// Capture position before parsing
		let (line_nr, column) = self.get_position();
		if self.is_at_line_end() {
			return Empty;
		};

		let ch = self.current_char();

		// Handle special non-expression cases first
		let node = match ch {
			';' => return Empty, // Semicolons handled by main parse loop
			'>' => return Empty, // Closing bracket handled by parse_bracketed
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			// Everything else goes through parse_expr for operator chaining
			_ => self.parse_expr(0),
		};

		if node == Empty {
			return node;
		}
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			#[cfg(debug_assertions)]
			line: self.current_line.s(),
		});
		// Attach preceding comment as metadata
		if let Some(comment_text) = comment {
			node.with_comment(comment_text)
		} else {
			node
		}
	}

	fn parse_string(&mut self) -> Node {
		let quote = self.current_char();
		let (quote_line, quote_column) = self.get_position();
		self.advance(); // skip opening quote

		let interpolates = quote == '"' && !self.options.data_mode && !self.options.xml_mode && !self.options.wit_mode;
		let mut s = String::new();
		// the same literal in injection::parts syntax (holes `${expr}`, literal dollars `$$`), kept while it has a hole
		let mut template = String::new();
		let mut is_template = false;
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				return error(&format!("Unterminated string: the text opened at {quote_line}:{quote_column} has no closing `{quote}`"));
			}
			if ch == quote {
				self.advance(); // skip closing quote
				let s: String = s.nfc().collect(); // an escape (`e\u{301}`) may leave it unnormalized
				if is_template {
					return crate::interpolation::template_text(&template);
				}
				// a one-character string is a Codepoint whichever quote is used, so only longer strings have a canonical quote
				if s.chars().count() > 1 {
					set_hint_position(quote_line, quote_column);
					norm::quotes(quote, &s);
				}
				// quotes with exactly one character become Codepoint
				let mut chars = s.chars();
				if let Some(c) = chars.next() {
					if chars.next().is_none() {
						if let Some(number_type) = self.number_cast_follows().filter(|_| interpolates && !c.is_ascii_digit()) {
							let message = format!("\"{c}\" as {number_type}: a text is no number (user decision #35); only a single-quoted character converts to its code point");
							return Diagnostic { message, line: quote_line, column: quote_column, fix: Some(format!("'{c}' as {number_type}")) }.into_error();
						}
						return Node::codepoint(c);
					}
				}
				return Node::text(&s);
			}
			let hole = match ch {
				'$' if interpolates => self.parse_dollar_hole(),
				'\\' if interpolates && self.peek_char(1) == '(' => self.parse_swift_hole().map(Some),
				_ => Ok(None),
			};
			match hole {
				Err(message) => return error(&message),
				Ok(Some(expression)) => {
					template.push_str(&format!("${{{expression}}}"));
					is_template = true;
					continue;
				}
				Ok(None) => {}
			}
			// a bare `$name` is text here but stays a hole for sql/sh templates (injection::parts), so it is not escaped
			let bare_dollar_name = ch == '$' && (self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_');
			let literal = if ch == '\\' {
				self.advance();
				let escaped = self.current_char();
				is_template |= interpolates && escaped == '$'; // an escaped dollar is never a hole, also in sql/sh
				match escaped {
					':' if interpolates && self.peek_char(1).is_ascii_alphabetic() => {
						let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos - 1) else { unreachable!("a letter follows") };
						let Some(character) = crate::uniscript_entities::entity(&name) else { return error(&crate::uniscript_entities::unknown_entity(&name)) };
						(0..length - 2).for_each(|_| self.advance());
						character
					}
					'n' => '\n',
					't' => '\t',
					'r' => '\r',
					'u' if self.peek_char(1) == '{' => match self.unicode_escape() {
						Ok(c) => c,
						Err(message) => return error(&message),
					},
					c => c,
				}
			} else {
				ch
			};
			self.advance();
			s.push(literal);
			match literal {
				'$' if !bare_dollar_name => template.push_str("$$"),
				c => template.push(c),
			}
		}
	}

	/// `\u{e9}` after the backslash: the code point of the hex digits; the closing `}` is left for the caller to skip
	fn unicode_escape(&mut self) -> Result<char, String> {
		self.advance(); // skip `u`, now at `{`
		let mut hex = String::new();
		while self.peek_char(1).is_ascii_hexdigit() {
			self.advance();
			hex.push(self.current_char());
		}
		if self.peek_char(1) != '}' {
			return Err(format!("\\u{{{hex}…: a unicode escape is hex digits in braces, like \\u{{e9}}"));
		}
		self.advance(); // now at `}`
		u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).ok_or_else(|| format!("\\u{{{hex}}} is no unicode code point"))
	}

	/// The number type of a following `as int` / `as float` …
	fn number_cast_follows(&self) -> Option<String> {
		let rest: String = self.chars[self.pos..].iter().take(32).collect();
		let words: Vec<&str> = rest.split_whitespace().take(2).collect();
		let number_type = words.get(1)?.trim_end_matches(|c: char| !is_identifier_char(c));
		use crate::type_kinds::Kind::{Float, Int};
		let is_number = matches!(crate::analyzer::builtin_type_kind(number_type), Some(Int | Float)) && !number_type.starts_with("bool");
		(words.first() == Some(&"as") && is_number).then(|| number_type.to_string())
	}

	/// `${expr}` inside interpolated text: the hole's expression, `None` for a plain dollar (`$5`, `$x`, `$ `).
	/// User decision D1: "only the one with the curly braces must interpolate the other is text like dollar money".
	fn parse_dollar_hole(&mut self) -> Result<Option<String>, String> {
		let (line, column) = self.get_position();
		let next = self.peek_char(1);
		let expression = if next == '{' {
			self.advance_by(2);
			let expression = self.text_until_closing('{', '}')?;
			set_hint_position(line, column);
			norm::interpolation(&format!("${{{expression}}}"), &expression);
			expression
		} else {
			return Ok(None);
		};
		Ok(Some(expression))
	}

	/// `\(expr)` inside interpolated text, the canonical hole: its expression
	fn parse_swift_hole(&mut self) -> Result<String, String> {
		self.advance_by(2);
		self.text_until_closing('(', ')')
	}

	/// The source up to the bracket closing an already opened `open`, skipping quoted text; consumes the closing bracket
	fn text_until_closing(&mut self, open: char, close: char) -> Result<String, String> {
		let start = self.pos;
		let mut depth = 1;
		let mut quote: Option<char> = None;
		loop {
			let ch = self.current_char();
			match (ch, quote) {
				('\0', _) => return Err(format!("unterminated interpolation hole: missing `{close}`")),
				('\\', Some(_)) => self.advance(),
				(c, Some(q)) if c == q => quote = None,
				('"' | '\'', None) => quote = Some(ch),
				(c, None) if c == open => depth += 1,
				(c, None) if c == close => {
					depth -= 1;
					if depth == 0 {
						let expression: String = self.chars[start..self.pos].iter().collect();
						self.advance();
						return if expression.trim().is_empty() { Err("empty interpolation hole".into()) } else { Ok(expression) };
					}
				}
				_ => {}
			}
			self.advance();
		}
	}
	fn parse_number(&mut self) -> Node {
		let start = self.pos;
		let number = self.parse_number_value();
		let literal: String = self.chars[start..self.pos].iter().collect();
		let number = self.keep_source_literal(number, &literal);
		match self.literal_type_suffix() {
			Some(number_type) => Node::Key(Box::new(number), Op::As, Box::new(Node::Symbol(number_type))),
			None => self.with_juxtaposed_factor(number),
		}
	}

	/// `2x` is 2*x, `3(4)` is 3*(4), `3x²` is 3*(x²): a number directly followed by a symbol or `(` multiplies.
	/// Code only: data keeps `size: 3px`. Ordinals `1st` `2nd` `3rd` `4th` are no products.
	fn with_juxtaposed_factor(&mut self, number: Node) -> Node {
		if self.options.data_mode || matches!(number, Node::Error(_)) {
			return number;
		}
		let next = self.current_char();
		let tight = (next.is_alphabetic() || next == '_' || next == '(') && !self.at_ordinal_suffix();
		if !tight && !self.at_spaced_unit() {
			return number;
		}
		self.skip_spaces();
		let factor = self.parse_atom();
		let factor = self.try_parse_superscript_power(&factor, 0).unwrap_or(factor);
		Node::Key(Box::new(number), Op::Mul, Box::new(factor))
	}

	/// `2 km` multiplies only when `km` is a known unit; any other spaced word keeps the list `[2 foo]`, and so does a
	/// unit word that starts the next entry or assignment: `{w:2 h:3}`
	fn at_spaced_unit(&self) -> bool {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let word: String = rest[spaces..].iter().take_while(|c| is_identifier_char(**c)).collect();
		let after_word: String = rest[spaces + word.chars().count()..].iter().skip_while(|c| **c == ' ').take(2).collect();
		let starts_entry = after_word.starts_with(':') || (after_word.starts_with('=') && after_word != "==");
		// `2 m²`: the power is no part of the unit's name
		let unit = word.trim_end_matches(|c: char| superscript_digit(c).is_some());
		spaces > 0 && crate::units::is_unit(unit) && !starts_entry
	}

	fn at_ordinal_suffix(&self) -> bool {
		let word: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c)).collect();
		ORDINAL_SUFFIXES.contains(&word.as_str())
	}

	/// Tight conversion written on the literal: `0.1f` → float, `0.1l` → exact, `0.1:float` / `1.5:int` → that type.
	/// Code only: in data `version: 1.10` and `n:int` stay key/value.
	fn literal_type_suffix(&mut self) -> Option<String> {
		if self.options.data_mode {
			return None;
		}
		let suffix = LITERAL_SUFFIXES.iter().find(|(letter, _)| *letter == self.current_char());
		if let Some((_, number_type)) = suffix.filter(|_| !is_identifier_char(self.peek_char(1))) {
			self.advance();
			return Some(number_type.to_string());
		}
		if self.current_char() != ':' {
			return None;
		}
		let name: String = self.chars[self.pos + 1..].iter().take_while(|c| is_identifier_char(**c)).collect();
		if !LITERAL_NUMBER_TYPES.contains(&name.as_str()) {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		Some(name)
	}

	/// Data round-trips its literals: `zip: 01234` stays `01234`, `version: 1.10` stays `1.10`
	fn keep_source_literal(&self, number: Node, literal: &str) -> Node {
		let is_lossy = matches!(number, Node::Number(_)) && number.serialize() != literal;
		if self.options.data_mode && is_lossy {
			number.with_source_literal(literal)
		} else {
			number
		}
	}

	fn parse_number_value(&mut self) -> Node {
		// RFC 3339 / RFC 9557 literal: 2024-01-31, 2024-01-31T10:00Z, 2024-01-31T10:00[Europe/Berlin]
		// Not in data mode yet: the data path round-trips source text and has no date serialization
		let literal_len = if self.options.data_mode { 0 } else { crate::time::literal_len(&self.chars[self.pos..]) };
		if literal_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + literal_len].iter().collect();
			self.advance_by(literal_len);
			return Node::data(crate::time::TimeLiteral(literal));
		}
		let version_len = crate::versions::literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + version_len].iter().collect();
			self.advance_by(version_len);
			return Node::Symbol(literal);
		}
		let mut num_str = String::new();

		// todo edge case: leading plus
		if self.current_char() == '-' {
			num_str.push('-');
			self.advance();
		}

		// Check for hexadecimal: 0x or 0X
		if self.current_char() == '0' {
			let next_ch = self.peek_char(1);
			if next_ch == 'x' || next_ch == 'X' {
				// Parse hexadecimal
				self.advance(); // skip '0'
				self.advance(); // skip 'x'
				let mut hex_str = String::new();
				loop {
					let ch = self.current_char();
					if !ch.is_ascii_hexdigit() {
						break;
					}
					hex_str.push(ch);
					self.advance();
				}
				return i64::from_str_radix(&hex_str, 16)
					.map(Node::int)
					.unwrap_or_else(|_| error(&format!("Invalid hex: 0x{}", hex_str)));
			}
		}

		if let Some(fraction) = self.vulgar_fraction_literal() {
			return fraction;
		}

		self.push_digits(&mut num_str);
		let mut is_float = false;
		// Only consume . as decimal if NOT followed by another . (range operator) or a word (`4.square` calls square)
		let after_point = self.peek_char(1);
		if self.current_char() == '.' && after_point != '.' && !(after_point.is_alphabetic() || after_point == '_') {
			is_float = true;
			num_str.push('.');
			self.advance();
			self.push_digits(&mut num_str);
		}
		let exponent = match self.parse_exponent() {
			Ok(exponent) => exponent,
			Err(e) => return error(&e),
		};

		match exponent {
			// 1e3 is the exact integer 1000, like the literal it abbreviates
			Some(exp) if !is_float && exp >= 0 => {
				if exp > MAX_INTEGER_EXPONENT {
					return error(&format!("Exponent too large: {}e{}", num_str, exp));
				}
				num_str.push_str(&"0".repeat(exp as usize));
				self.integer_node(&num_str)
			}
			Some(exp) => format!("{}e{}", num_str, exp)
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}e{}", num_str, exp))),
			None if is_float => num_str
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}", num_str))),
			None => {
				let number = self.integer_node(&num_str);
				self.with_time_unit(number)
			}
		}
	}

	/// `⅓` is the exact ratio 1/3; a number written directly after it multiplies: `⅓9` is 3
	fn vulgar_fraction_literal(&mut self) -> Option<Node> {
		let (numerator, denominator) = vulgar_fraction(self.current_char())?;
		self.advance();
		let ratio = Number::ratio(Number::Int(numerator), Number::Int(denominator));
		let fraction = Node::Number(ratio);
		if !self.current_char().is_ascii_digit() {
			return Some(fraction);
		}
		let factor = self.parse_number_value();
		Some(Node::Key(Box::new(fraction), Op::Mul, Box::new(factor)))
	}

	/// `1 month`, `24 hours`: an integer followed by a time unit word is a duration
	fn with_time_unit(&mut self, number: Node) -> Node {
		let Node::Number(Number::Int(count)) = &number else {
			return number;
		};
		if self.options.data_mode {
			return number;
		}
		let Some((unit, length)) = crate::time::unit_after(&self.chars[self.pos..]) else {
			return number;
		};
		self.advance_by(length);
		Node::data(unit.times(*count))
	}

	fn integer_node(&self, digits: &str) -> Node {
		crate::extensions::numbers::Number::parse_integer(digits)
			.map(Node::Number)
			.unwrap_or_else(|| error(&format!("Invalid int: {}", digits)))
	}

	/// Digits with `_` separators between them: 1_000_000
	fn push_digits(&mut self, num_str: &mut String) {
		loop {
			let ch = self.current_char();
			// '三'.is_numeric() is true but not ASCII
			// Exclude superscripts and vulgar fractions: suffix operators and literals of their own
			if ch.is_numeric() && !is_number_glyph(ch) {
				num_str.push(ch);
				self.advance();
			} else if ch == '_' && self.prev_char().is_ascii_digit() && self.peek_char(1).is_ascii_digit() {
				self.advance();
			} else {
				break;
			}
		}
	}

	/// Scientific notation suffix: e3, E-3, e+03 (only when digits follow, so `2em` stays a unit)
	fn parse_exponent(&mut self) -> Result<Option<i64>, String> {
		let (e, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let signed = (c2 == '+' || c2 == '-') && c3.is_ascii_digit();
		if !(e == 'e' || e == 'E') || !(c2.is_ascii_digit() || signed) {
			return Ok(None);
		}
		self.advance();
		let mut exponent = String::new();
		if signed {
			exponent.push(self.current_char());
			self.advance();
		}
		self.push_digits(&mut exponent);
		exponent.parse::<i64>().map(Some).map_err(|_| format!("Invalid exponent: e{}", exponent))
	}

	/// `?` directly after a type name, then `=` (not `==`) or a closing bracket or separator
	fn ends_optional_type(&self, next: char, after: char) -> bool {
		matches!(next, ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0') || (next == '=' && after != '=')
	}

	/// Do blanks and then a block of fields follow the cursor: `{}` or `{ name: …`, never a statement block like `{ out += x }`
	fn block_after_blanks(&self) -> bool {
		let blanks_from = |start: usize| (start..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let blanks = blanks_from(0);
		if blanks == 0 || self.peek_char(blanks) != '{' {
			return false;
		}
		let first = blanks + 1 + blanks_from(blanks + 1);
		let name = (first..).take_while(|at| self.peek_char(*at).is_alphanumeric() || self.peek_char(*at) == '_').count();
		let after_name = first + name + blanks_from(first + name);
		self.peek_char(first) == '}' || (name > 0 && self.peek_char(after_name) == ':' && self.peek_char(after_name + 1) != '=')
	}

	/// Do blanks and then a closing bracket, `,`, `;` or the line end follow `offset`: nothing a ternary could be followed by
	fn closes_after_blanks(&self, offset: usize) -> bool {
		let blanks = (offset..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		blanks > 0 && matches!(self.peek_char(offset + blanks), ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0')
	}

	fn parse_symbol(&mut self) -> Result<String, String> {
		let mut symbol = String::new();
		loop {
			let ch = self.current_char();
			// Include alphanumeric, underscore, and hyphen (for kebab-case)
			// BUT: a hyphen joins only a following word, so `i--` and `x-1` end the symbol at the hyphen
			let next = self.peek_char(1);
			let is_hyphen_in_symbol = ch == '-' && (next.is_alphabetic() || next == '_');
			if (ch.is_alphanumeric() && !is_number_glyph(ch)) || ch == '_' || is_hyphen_in_symbol {
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

	/// `tuple<s64, list<node>>` → `tuple` followed by the angle-bracketed argument list
	/// Length of the `<int>`, `<list<int>>`, `<text, int>` directly at the cursor when everything inside is a type word;
	/// `a<b`, `a<<b` and `a<b>c` are not type applications
	fn type_application_length(&self) -> Option<usize> {
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
				',' | ' ' => {}
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

	fn parse_type_application(&mut self, type_name: String) -> Node {
		self.advance(); // skip '<'
		let arguments = self.parse_list_with_separators(Some('>'), Bracket::Less);
		Node::List(vec![Symbol(type_name), arguments], Bracket::None, Separator::None)
	}

	fn parse_bracketed(&mut self, open: char) -> Node {
		let (close, bracket_type) = match open {
			'(' => (')', Bracket::Round),
			'[' => (']', Bracket::Square),
			'{' => ('}', Bracket::Curly),
			'<' => ('>', Bracket::Round),
			_ => panic!("Invalid bracket: {}", open),
		};
		let start = self.get_position();
		let outer_start = std::mem::replace(&mut self.group_start, start);
		self.advance(); // skip opening bracket
		let compares = self.equals_compares && bracket_type != Bracket::Curly; // a block is not the condition
		// `for i in (0 until n)` and `(0..n-1)` are still the header; a block or a list inside it is not
		let inner_header = self.in_for_header && bracket_type == Bracket::Round;
		let outer_header = std::mem::replace(&mut self.in_for_header, inner_header);
		let list = self.with_equals_comparing(compares, |parser| parser.parse_list_with_separators(Some(close), bracket_type));
		self.in_for_header = outer_header;
		self.group_start = outer_start;
		list
	}

	/// Parse XML tag: <tag attr="value">content</tag> or <tag />
	fn parse_xml_tag(&mut self) -> Node {
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

		// Parse tag name
		let tag_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(e) => return error(&e),
		};

		// Parse attributes
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

	fn parse_list_with_separators(&mut self, close: Option<char>, bracket: Bracket) -> Node {
		// Collect all items with their following separators
		let mut items_with_seps: Vec<(Node, Separator)> = Vec::new();

		loop {
			// Skip whitespace but track indent for dedent detection
			let (had_newline, line_indent) = self.skip_whitespace();

			// Check for dedent - exit this block if we're back to lower indent
			if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				break;
			}

			// Check for end condition (also check for end-of-input to avoid infinite loop)
			let ch = self.current_char();
			let at_end = match close {
				Some(c) => ch == c || ch == '\0',
				None => self.end_of_input() || self.at_block_close(),
			};
			if at_end {
				if let Some(closer) = close.filter(|_| ch == '\0') {
					let (line, column) = self.group_start;
					return error(&format!("`{closer}` is missing: the group opened at {line}:{column} runs to the end of the input"));
				}
				if close.is_some() {
					self.advance(); // consume closing bracket
				}
				break;
			}

			let pos_before = self.pos;
			let item = self.parse_value();

			let consumed_input = self.pos != pos_before;
			// `==` is loose (false equals ø): only a real ø is skipped
			if matches!(item, Empty) && !(consumed_input && close.is_some()) {
				if !consumed_input {
					self.advance();
				}
				continue;
			}

			let (had_newline, line_indent, _) = self.skip_whitespace_and_comments();

			// Handle indentation-based blocks
			let item = if had_newline && line_indent > self.base_indent && bracket == Bracket::None
			{
				// Indented block follows - parse it as body of current item
				let old_indent = self.base_indent;
				self.base_indent = line_indent;
				let body = self.parse_list_with_separators(None, Bracket::None);
				self.base_indent = old_indent;
				// Combine item with indented body as Key
				Node::Key(Box::new(Symbol(item.name())), Op::Colon, Box::new(body))
			} else if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				// Dedent - push item and exit this level
				items_with_seps.push((item, Separator::None));
				break;
			} else {
				item
			};

			// Determine separator after this item
			let ch = self.current_char();
			let at_end = match close {
				Some(c) => ch == c || ch == '\0',
				None => self.end_of_input() || self.at_block_close(),
			};
			let sep = if at_end {
				Separator::None
			} else if ch == ',' {
				self.advance();
				Separator::Colon
			} else if ch == ';' {
				self.advance();
				if self.only_blanks_before_newline() { Separator::Newline } else { Separator::Semicolon }
			} else if had_newline {
				Separator::Newline
			} else {
				Separator::Space
			};

			items_with_seps.push((item, sep));

			if self.pos == pos_before {
				self.advance();
			}
		}

		let list = self.group_by_separators(items_with_seps, bracket);
		match list.duplicate_key() {
			Some(key) => error(&format!("duplicate key '{}'", key)),
			None => list,
		}
	}
	fn group_by_separators(
		&self,
		items_with_seps: Vec<(Node, Separator)>,
		bracket: Bracket,
	) -> Node {
		if items_with_seps.is_empty() {
			return if bracket == Bracket::Curly { Node::List(Vec::new(), bracket, Separator::None) } else { Empty }; // an empty block is a value
		}

		if items_with_seps.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings
			return items_with_seps[0].0.clone();
		}

		// Collect all unique separator precedences (excluding None)
		let mut precedences: Vec<u8> = items_with_seps
			.iter()
			.map(|(_, sep)| sep.precedence())
			.filter(|&p| p < 255)
			.collect();
		precedences.sort();
		precedences.dedup();

		if precedences.is_empty() {
			// All items have None separator - return as space-separated list
			let items: Vec<Node> = items_with_seps.into_iter().map(|(node, _)| node).collect();
			if items.len() == 1 && bracket == Bracket::None {
				// Only unwrap single items for implicit groupings
				return items[0].clone();
			}
			return grouped_list(items, bracket, Separator::Space);
		}

		// Start with the loosest (highest precedence value) separator
		let max_prec = *precedences.last().unwrap();
		let split_sep = items_with_seps
			.iter()
			.find(|(_, sep)| sep.precedence() == max_prec)
			.map(|(_, sep)| sep.clone())
			.unwrap_or(Separator::Space);

		// Split items into groups by this separator
		let mut groups: Vec<Vec<(Node, Separator)>> = Vec::new();
		let mut current_group = Vec::new();

		for (item, sep) in items_with_seps {
			if sep.precedence() == max_prec {
				// Found a split point - add item and close group
				current_group.push((item, Separator::None));
				if !current_group.is_empty() {
					groups.push(current_group);
					current_group = Vec::new();
				}
			} else {
				// Keep this separator for processing in sub-groups
				current_group.push((item, sep));
			}
		}

		// Add final group
		if !current_group.is_empty() {
			groups.push(current_group);
		}

		// Filter empty groups
		groups.retain(|g| !g.is_empty());

		if groups.is_empty() {
			return Empty;
		}

		// Recursively process each group for tighter separators
		let grouped_nodes: Vec<Node> = groups
			.into_iter()
			.map(|group| {
				if group.len() == 1 && group[0].1 == Separator::None {
					// Single item with no further separators
					group[0].0.clone()
				} else {
					// Has multiple items or tighter separators - recurse
					// Inner groups use Bracket::None to avoid extra braces in serialization
					self.group_by_separators(group, Bracket::None)
				}
			})
			.collect();

		// Return result
		if grouped_nodes.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings (Bracket::None)
			// Explicit brackets like {x} or [x] should preserve the wrapper
			grouped_nodes[0].clone()
		} else {
			grouped_list(grouped_nodes, bracket, split_sep)
		}
	}

	/// Transform field definitions: Key(name, op, Symbol) -> Key(name, op, Type)
	/// Used for class/struct definitions to convert type names to Type nodes
	fn transform_fields_to_types(node: Node) -> Node {
		match node {
			Node::List(items, bracket, sep) => {
				let transformed: Vec<Node> = items.into_iter().map(Self::transform_fields_to_types).collect();
				Node::List(transformed, bracket, sep)
			}
			Node::Key(name, op, value) => {
				let type_node = Self::symbol_to_type(*value);
				Node::Key(name, op, Box::new(type_node))
			}
			Node::Meta { node, data } => {
				Node::Meta { node: Box::new(Self::transform_fields_to_types(*node)), data }
			}
			other => other,
		}
	}

	/// Convert a Symbol to a Type node (for type references)
	fn symbol_to_type(node: Node) -> Node {
		match node {
			Node::Symbol(s) => Node::Type {
				name: Box::new(Node::Symbol(s)),
				body: Box::new(Empty),
			},
			Node::Meta { node, data } => {
				Node::Meta { node: Box::new(Self::symbol_to_type(*node)), data }
			}
			other => other,
		}
	}
}

/// In data only JSON's words are literals; aliases like `yes`, `no`, `none`, `pi` stay symbols.
const DATA_WORD_LITERALS: [&str; 3] = ["true", "false", "null"];

fn real(exact: Exact) -> Node {
	Node::Number(Number::real(Real::Exact(exact)))
}

fn check_constants(s: &str, data_mode: bool) -> Option<Node> {
	let is_word = s.chars().all(|c| c.is_ascii_alphabetic());
	if data_mode && is_word && !DATA_WORD_LITERALS.contains(&s) {
		return None;
	}
	match s.to_lowercase().as_str() {
		"⊤" | "true" | "yes" | "✓" | "🗸" | "✔" | "✓️" | "🗹" | "☑" | "✅" | "⊨" => Some(Node::True),
		"⊥" | "false" | "no" | "⊭" | "❌" | "" => Some(Node::False),
		"ø" | "null" | "nul" | "none" | "nil" | "nill" | "nix" | "nada" | "nothing" | "empty" | "void" => Some(Empty),
		// exact generators (extensions/reals.rs); a bare `e` or `i` stays a free name
		"π" | "pi" => Some(real(Exact::pi())),
		"τ" | "tau" => Some(real(Exact::pi().scale(&Rational::integer(2)))),
		"euler" | "ℯ" => Some(real(Exact::euler())),
		"ⅈ" => Some(real(Exact::imaginary())),
		// hyperreals (wiki/hyperreals.md): the glyphs only, `epsilon` stays a free name like `e`
		"ε" => Some(real(Exact::epsilon())),
		"ω" => Some(real(Exact::omega())),
		"⚠️" | "⚡" | "⚡️" => Some(error(s)),
		_ => None,
	}
}
// Tests moved to tests/parser/test_parser.rs

/// `f := …`, `f x := …`, `f(x) := …` define f
fn defined_function_name(target: &Node) -> Option<String> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// `n-1`, the end a Kotlin writer gives an inclusive `..`
fn is_minus_one(end: &Node) -> bool {
	match end.drop_meta() {
		Node::Key(_, Op::Sub, one) => matches!(one.drop_meta(), Node::Number(Number::Int(1))),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_minus_one(&items[0]),
		_ => false,
	}
}

pub(crate) fn mentions(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(left, _, right) => mentions(left, name) || mentions(right, name),
		Node::List(items, _, _) => items.iter().any(|item| mentions(item, name)),
		_ => false,
	}
}

/// `a==b` built in the same expression, not grouped by parentheses (a group is parsed as its own node)
fn ungrouped_equality(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if op.is_equality())
}

/// `1==1==1` chains in Python (true) but is `(1==1)==1` in C: ambiguous, like in Rust
fn chained_equality(lhs: &Node, op: Op, rhs: &Node) -> Diagnostic {
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let middle = match lhs {
		Node::Key(_, _, middle) => middle.serialize(),
		_ => left.clone(),
	};
	let symbol = op.as_str();
	Diagnostic {
		message: format!("ambiguous: equality does not chain in {left} {symbol} {right}"),
		line: 0,
		column: 0,
		fix: Some(format!("{left} and {middle} {symbol} {right} or ({left}) {symbol} {right}")),
	}
}

/// `3 & 4 == 4` is `(3&4)==4` in Python but `3 & (4==4)` in C: `&`/`|` next to an ungrouped comparison is ambiguous.
/// The word forms `and`/`or` read unambiguously and are not affected: `x==1 and y==2`.
fn logic_mixed_with_comparison(lhs: &Node, symbol: char, rhs: &Node) -> Option<Diagnostic> {
	let comparison = |node: &Node| match node.drop_meta() {
		Node::Key(left, op, right) if op.is_comparison() => Some((left.serialize(), op.as_str(), right.serialize())),
		_ => None,
	};
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let fix = match (comparison(lhs), comparison(rhs)) {
		(None, Some((a, op, b))) => format!("{left} {symbol} ({a} {op} {b}) or ({left} {symbol} {a}) {op} {b}"),
		(Some((a, op, b)), None) => format!("({a} {op} {b}) {symbol} {right} or {a} {op} ({b} {symbol} {right})"),
		(Some(_), Some(_)) => format!("({left}) {symbol} ({right})"),
		(None, None) => return None,
	};
	let word = if symbol == '&' { "and" } else { "or" };
	Some(Diagnostic {
		message: format!("ambiguous: `{symbol}` mixed with a comparison in {left} {symbol} {right}; group it or write `{word}`"),
		line: 0,
		column: 0,
		fix: Some(fix),
	})
}


/// The number a subscript was written with, `1` of `d[1]`, kept on its shifted index `d#2`: a map keyed by numbers
/// reads it (number_keys.rs, P34)
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenIndex(pub Number);

/// `target[index]` is the 1-based `target#(index+1)`; a numeric index is shifted at parse time.
/// A slice `target[start:end]`, `target[start..end]`, `target[start...last]` is the call `slice(target, start, end)`.
/// A negative index is an error that names the way to the last element: wasp never wraps around (Footguns.md).
pub fn subscript(target: Node, index: Node) -> Node {
	let positions = match index.drop_meta() {
		Node::Key(start, _, end) if slice_bounds(&index).is_some() => vec![start.as_ref(), end.as_ref()],
		_ => vec![&index],
	};
	if let Some(negative) = positions.into_iter().find(|position| counts_from_the_end(position)) {
		let message = format!("index out of range: negative index {} does not wrap around, the last element is last({})", negative.serialize(), target.serialize());
		return Diagnostic::at(negative, message).into_error();
	}
	if let Some((start, end)) = slice_bounds(&index) {
		return Node::List(vec![Symbol(SLICE_WORD.to_string()), target, start, end], Bracket::Round, Separator::None);
	}
	let one = Node::Number(Number::Int(1));
	let one_based = match index.drop_meta() {
		Node::Number(n) => Node::meta(Node::Number(*n + Number::Int(1)), Node::data(WrittenIndex(*n))),
		_ => Node::Key(Box::new(index), Op::Add, Box::new(one)),
	};
	Node::Key(Box::new(target), Op::Hash, Box::new(one_based))
}

/// A negative number or a negated expression: `-1`, `-k`
fn counts_from_the_end(index: &Node) -> bool {
	match index.drop_meta() {
		Node::Number(n) => f64::from(*n) < 0.0,
		Node::Key(empty, Op::Sub, _) => matches!(empty.drop_meta(), Empty),
		_ => false,
	}
}

/// The 0-based start and exclusive end of a slice index, ø where omitted (Python's `a[1:]`, `a[:2]`);
/// an inclusive range `start...last` ends after last
fn slice_bounds(index: &Node) -> Option<(Node, Node)> {
	let Node::Key(start, op, end) = index.drop_meta() else { return None };
	let end = match op {
		Op::Colon | Op::Range => end.as_ref().clone(),
		Op::To => Node::Key(end.clone(), Op::Add, Box::new(Node::Number(Number::Int(1)))),
		_ => return None,
	};
	Some((start.as_ref().clone(), end))
}

/// The written index of a subscript's 1-based index `index+1`, when it was not a number (inverse of `subscript`)
pub fn subscript_key(one_based_index: &Node) -> Option<&Node> {
	match one_based_index.drop_meta() {
		Node::Key(key, Op::Add, one) if matches!(one.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(1))) => Some(key),
		_ => None,
	}
}

/// `while condition body`: the loop head `ø while condition` applied `do` to its body
/// Statements as a `{…}` block body
fn curly_block(statements: Node) -> Node {
	match statements {
		Node::List(items, Bracket::None, separator) => Node::List(items, Bracket::Curly, separator),
		single => Node::List(vec![single], Bracket::Curly, Separator::None),
	}
}

pub(crate) fn while_do(condition: Node, body: Node) -> Node {
	let head = Node::Key(Box::new(Empty), Op::While, Box::new(condition));
	Node::Key(Box::new(head), Op::Do, Box::new(body))
}

/// The condition of the `if`/`while` head inside a parsed conditional or loop, negated: `unless c {…}` is `if not c {…}`
fn negate_condition(conditional: Node) -> Node {
	match conditional {
		Node::Key(head, op @ (Op::Then | Op::Else | Op::Do), body) => Node::Key(Box::new(negate_condition(*head)), op, body),
		Node::Key(empty, op @ (Op::If | Op::While), condition) if matches!(empty.drop_meta(), Empty) => {
			Node::Key(empty, op, Box::new(Node::Key(Box::new(Empty), Op::Not, condition)))
		}
		other => other,
	}
}

/// `f={it*2}` defines a function like `f:={it*2}`: a name assigned a block that uses `it`; other blocks are data
fn is_function_block(name: &Node, block: &Node) -> bool {
	matches!(name.drop_meta(), Node::Symbol(_)) && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) && mentions(block, "it")
}
