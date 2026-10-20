use crate::compiler::meta::LineInfo;
use crate::errors::diagnostic::{ask, reading, Ask, Diagnostic, Fallback, Level};
use crate::extensions::numbers::Number;
use crate::extensions::reals::{Exact, Rational, Real};
use crate::node::call;
use crate::node::Node::{Empty, Symbol};
use crate::node::{error, key_ops, Bracket, Node, Separator};
use crate::normalize::{hints as norm, set_hint_position, ListTypeStyle};
use crate::operators::{glyph_operator, is_function_keyword, Op};
use crate::words::DO_WORD;
use crate::words::EACH_WORD;
use crate::words::IS_WORD;
use crate::words::MAP_WORD;
use crate::words::OF_WORD;
use crate::words::PRINT_WORD;
use log::warn;
use std::fs::read_to_string;

use crate::node::ATTRIBUTE_MARK;
mod declaration_scan;
pub(crate) use declaration_scan::*;
mod collections;
mod context;
pub use collections::*;
use context::Context;

/// The value of a `Result<_, String>`; an error returns from the parse function as an error node
macro_rules! or_return_error {
	($result:expr) => {
		match $result {
			Ok(value) => value,
			Err(message) => return error(&message),
		}
	};
}

mod lookahead;
mod markup;
mod scanning;
pub use lookahead::PREFIX_OPERATOR_WORDS;
mod atoms;
pub(crate) use atoms::hyphen_joins_word;
mod blocks;
mod commands;
mod comments;
mod pipes;
use blocks::*;
pub(crate) use commands::*;
use comments::*;
pub(crate) use control::*;
pub(crate) use definitions::*;
pub(crate) use pipes::*;
mod control;
mod definitions;
mod expressions;
mod literals;
pub(crate) use expressions::*;
pub(crate) use literals::*;
pub use literals::{shader_blocks, without_shader_texts, PathLiteral};
use lookahead::*;
use scanning::*;
mod annotations;
mod math_notation;
mod rust_macros;
mod suffixes;
mod sugar;
mod types;
pub(crate) use math_notation::*;
pub(crate) use types::*;
use unicode_normalization::UnicodeNormalization;

/// The item of the map a Julia dot call `f.(xs)` lowers to
const BROADCAST_ITEM: &str = "broadcast_item";

/// `nonlocal y` in a nested function: it reads the enclosing function's current y (lowering/definitions/late_binding.rs)
const NONLOCAL_WORD: &str = "nonlocal";

/// Control words behind a statement, each lowering to `if`/`while`, negated for `unless`/`until`
/// Words that declare a type from a field block: `struct point{x:int y:int}`, `class contact {name email?}`
pub(crate) const TYPE_DECLARATION_WORDS: [&str; 2] = ["class", "struct"];
/// The keywords of a field: Kotlin's primary constructor `class Point(val x: Int, var y: Int)`, Swift's `var count = 0`
pub(crate) const FIELD_KEYWORDS: [&str; 3] = ["val", "var", "let"];
/// Java's and TypeScript's `class Square implements Shape {…}`
pub const IMPLEMENTS_WORD: &str = "implements";
/// `enum Color {red, green}` (declarations::enum_object), Kotlin's `enum class`
const ENUM_WORD: &str = "enum";
/// Go's `type Shape interface {…}` declares the trait Shape
const GO_INTERFACE_WORD: &str = "interface";
/// Go's `type Point struct {…}` declares the class Point
const GO_STRUCT_WORD: &str = "struct";
/// `type Node: gc struct {…}` (WebAssembly's GC structs): every warp class is one, the word adds nothing
const GC_WORD: &str = "gc";
/// C++'s and C#'s `operator +(o)`: the method of `+` named by its glyph
const OPERATOR_WORD: &str = "operator";
/// Words before a member of a class body that change nothing in warp: Swift's `mutating func`, visibility, `override`
pub const MEMBER_MODIFIERS: [&str; 11] = [
	"mutating",
	"override",
	"public",
	"private",
	"protected",
	"internal",
	"fileprivate",
	"open",
	"final",
	"async",
	"operator",
];
/// `record point{x:int y:int}` declares a type like `struct`, but `record` is also an everyday variable name:
/// it is a declaration only when a name and a field block follow
const RECORD_WORD: &str = "record";
pub(crate) const REVERSE_WORD: &str = "reverse";
const FAILED_WORD: &str = "failed";
const CLASS_KEYWORD: &str = "class";
const IF_WORD: &str = "if";
/// Ruby/Lua blocks: `while c do … end`, `if c then … else … end`; `fi` is an `end` alias
const END_KEYWORD: &str = "end";
const FI_KEYWORD: &str = "fi";
const ELSE_KEYWORD: &str = "else";
const END_BLOCK_OPENERS: [&str; 2] = ["do", "then"];
/// The words a Ruby `end` closes in a class body: `def … end`, `do … end`
const RUBY_END_OPENERS: [&str; 3] = ["def", "do", "class"];

/// The marker calls the parser leaves for `try X else Y` and `assert C else X`, lowered in `library_words`
pub const TRY_MARKER: &str = "try·else";
/// `after C return V` (wiki/thread.md) as the marker call `after·return(C, V)`, lowered by go_blocks into a waiting task
pub const AFTER_MARKER: &str = "after·return";
/// `class dog extends animal {…}`: the class named on the right is the parent (P117)
pub const EXTENDS_KEYWORD: &str = "extends";
/// `mixin Walker{…}` declares fields and methods classes take in: `class Duck with Walker, Swimmer {…}`
pub const MIXIN_WORD: &str = "mixin";
pub const WITH_KEYWORD: &str = "with";
/// The constructor of a class body, `init{…}` or `init(name){…}` (wiki/constructor.md, P162)
pub const CONSTRUCTOR_WORD: &str = "init";
/// The accessors of a class property, `get age() {…}`, `set age(v) {…}` (wiki/property.md)
pub const ACCESSOR_WORDS: [&str; 2] = ["get", "set"];
/// `static k = 3` in a class body: a member of the class, not of each instance (P122); kept as the annotation `@static`
pub const STATIC_KEYWORD: &str = "static";
/// `sleep 1s and print "x"`: an `and` between two statements runs them one after the other
const AND_KEYWORD: &str = "and";

pub const ASSERT_MARKER: &str = "assert·else";
pub const ASSERTION_FAILED: &str = "assertion failed";

const TO_WORD: &str = "to";
/// `a[start:end]` calls the library word `slice`
const SLICE_WORD: &str = crate::words::SLICE;
pub(crate) const TIMES_WORD: &str = "times";
/// The call `n times text` is parsed to: the text repeated n times
pub const TEXT_TIMES: &str = "times·text";
/// The severe warning on a `//` comment meant as floor division, `7//2` (decision floor-division)
pub const FLOOR_DIVISION_TOPIC: &str = "floor-division-comment";
const JULIA_FLOOR_DIVISION_TOPIC: &str = "julia-floor-division";
const FLOOR_DIVISION: &str = "/!";

/// Superscript digits ⁰…⁹ in digit order: `3⁴` is 3^4 and a run of them is one exponent (`2¹⁰` is 2^10)
const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";

/// Parser options for handling different file formats
#[derive(Clone, Copy, Debug, PartialEq, Default)]
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

/// `floor_quotient(a, b)`: what `a // b` and `a div b` are, the Euclidean quotient that goes with `%`
/// (a == b*(a//b) + a%b): floor(a/b) for a positive divisor, ceil(a/b) for a negative one. Each operand is evaluated
/// once, and an exact quotient rounds exactly (wasm_emitter exact_euclid_div)
pub const FLOOR_QUOTIENT: &str = "floor_quotient";

/// Read and parse a WARP file
pub fn parse_file(path: &str) -> Node {
	let options = if path.ends_with(".wit") { ParserOptions::wit() } else { ParserOptions::default() };
	match read_to_string(path) {
		Ok(content) => WarpParser::parse_with_options(&content, options),
		_ => error(&format!("Failed to read {}", path)),
	}
}

pub fn parse(input: &str) -> Node {
	if input.ends_with(".warp") {
		return parse_file(input);
	}
	WarpParser::parse(input)
}

/// Code a lowering pass writes, placed where `at` was written: an error inside it names that position, not the
/// template's own column (card template-positions: `sum 1` said 'not a list at 1:78', now 'at 1:5', the 1 it walks)
pub fn parse_template(template: &str, at: &Node) -> Node {
	generated(template, crate::errors::diagnostic::line_info(at).as_ref())
}

/// Code a lowering pass writes, without positions: an error inside it names the nearest written position around it
pub fn parse_generated(code: &str) -> Node {
	generated(code, None)
}

/// Generated code joins the tree after the sugar passes: `{ return ø }` is given the phrase they would give it
fn generated(code: &str, position: Option<&LineInfo>) -> Node {
	crate::lowering::welcome_forms::lower_bare_returns(placed(parse(code), position))
}

/// Every position in `node` replaced by `position`, or dropped without one
fn placed(node: Node, position: Option<&LineInfo>) -> Node {
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<LineInfo>().is_some()) =>
		{
			let inner = placed(*node, position);
			match position {
				Some(position) => inner.with_meta_data(position.clone()),
				None => inner,
			}
		}
		other => crate::node::children_rewritten(other, |child| placed(child, position)),
	}
}

/// Parse-only path for untrusted data: nothing is evaluated, no word is guessed to be a boolean
pub fn parse_data(input: &str) -> Node {
	WarpParser::parse_with_options(input, ParserOptions::data())
}

pub fn parse_xml(input: &str) -> Node {
	WarpParser::parse_with_options(input, ParserOptions::xml())
}

pub struct WarpParser {
	input: String,
	chars: Vec<char>,
	pos: usize,
	line_nr: usize,
	column: usize,
	char: char,
	pub current_line: String,
	/// The input's lines, for current_line at each line break (splitting the input at every one was quadratic)
	lines: Vec<String>,
	options: ParserOptions,
	/// What the enclosing constructs say about the text being parsed (context.rs), set for a stretch by within()
	context: Context,
	/// The variables of the enclosing `for k in keys(m)` loops: `m[k]` looks a key up, so no indexing hint
	key_variables: Vec<String>,
	/// Names assigned a text or typed text (`l = "en"`, `k:text`): `m[l]` looks a key up too (card text-key)
	text_variables: std::collections::HashSet<String>,
	/// Names assigned so far (`data = […]`): such a name is read as the variable, not as a prefix word
	variables: std::collections::HashSet<String>,
	/// Where the last path literal ended: a ` /name` after it is the next path (`cp /a /b`)
	path_end: Option<usize>,
	/// The functions the source defines anywhere, found by a pre-scan: ` /name` after one is a path (card name-after)
	program_functions: std::collections::HashSet<String>,
	/// Whether the source uses `end` as a name (`end = 3`), found by a pre-scan: then no `end` closes a block
	names_end: bool,
	/// The corresponding name check for the `fi` block-closing alias
	names_fi: bool,
	/// The `end`s and openers of the source per set of openers, indexed once on the first search for a closing `end`,
	/// so each `do`/`def` costs a lookup instead of a scan of the rest of the source
	closing_ends: std::cell::RefCell<std::collections::HashMap<String, blocks::ClosingEnds>>,
	/// The parameters of the definitions around: ` /name` after one divides, as after a variable; each definition's
	/// until the statement holding it ends (card path-parameter-scope)
	parameters_in_scope: Vec<String>,
	/// Inside `class Name {…}`: its fields named like a constant (`pi = 3`, `pi:int`), which its methods read instead
	/// of the constant; None outside a type body
	type_fields: Option<std::collections::HashSet<String>>,
	/// The position of the sign in `1 -1` read as the list `[1 -1]`: no enclosing expression subtracts it either (`x=1 -1`)
	signed_list_element: Option<usize>,
	/// The type parameters skipped after a function's name (`fn id<T>`), marked on the atom (P157)
	generic_names: Option<(String, Vec<String>)>,
	/// The next `{…}` is the block of a named tag (`ul{…}`, `ul {…}`): a repeated `key: value` there is a child, as
	/// repeated elements in XML/HTML (`ul{ li: "a" li: "b" }`), no duplicate key; a plain `{…}` is a map that reports
	/// one (user decision P176, card g-_bGo). Taken by the list it opens, so blocks inside it are not tag blocks
	tag_block: bool,
	/// A comment between two statements: it belongs to the next one (parse_value attaches it)
	pending_comment: Option<String>,
	/// The symbol parsed last was a function keyword (`def`, `function`): the next one is the function's name
	after_function_keyword: bool,
	/// The expression continue_expr goes on with heads a definition: `foo` of `def foo: hi=2*2`
	continues_definition_head: bool,
	/// `a ?: b` with a computed left side parsed so far, numbering their hidden variables
	elvis_operands: usize,
	/// `try … catch … finally {…}` parsed so far, numbering the variables holding their values
	finally_blocks: usize,
	/// Names defined with `:=` so far: a braceless call of one may take an identifier argument anywhere (`fac it-1`)
	functions: std::collections::HashSet<String>,
	/// Those of them declared with named parameters (`f x y := …`), the rest take the implicit `it`
	functions_with_parameters: std::collections::HashSet<String>,
	/// Operators the program declares (`suffix operator ‼ := it*2`), longest glyph first, found by a pre-scan of the source
	user_operators: Vec<UserOperator>,
	/// Operator aliases the program declares (`operator ⊕ := +`), found by a pre-scan of the source (operators/aliases.rs)
	source_aliases: Vec<crate::operators::OperatorAlias>,
	/// Types the program declares (`class point{…}`), found by a pre-scan of the source: `point{x:1}` constructs one (D4)
	declared_types: std::collections::HashSet<String>,
	/// The words of the declared types of several words (`class full name {…}`), longest first: read as one name `full-name`
	multi_word_types: Vec<Vec<String>>,
	/// The parameters of each `to` phrase defined so far, to catch a second phrase that differs only by untyped nouns
	phrase_definitions: std::collections::HashMap<String, Vec<Node>>,
}

pub(crate) enum ElseParseMode {
	Atom,
	Expr,
}

impl WarpParser {
	pub fn new(input: String) -> Self {
		Self::new_with_options(input, ParserOptions::default())
	}

	/// Source text is normalized to NFC, so equal-looking text and identifiers are equal
	pub fn new_with_options(input: String, options: ParserOptions) -> Self {
		// most sources are NFC already: normalizing them only copies them, slowly in a debug build
		let input: String = match unicode_normalization::is_nfc_quick(input.chars()) {
			unicode_normalization::IsNormalized::Yes => input,
			_ => input.nfc().collect(),
		};
		let input = if options.data_mode { input } else { crate::text::uniscript_entities::expand_entities(&input) };
		let lines: Vec<String> = input.lines().map(String::from).collect();
		let current_line = lines.first().cloned().unwrap_or_default();
		let chars: Vec<char> = input.chars().collect();
		let user_operators = if options == ParserOptions::default() { scan_user_operators(&input) } else { Vec::new() };
		let source_aliases =
			if options == ParserOptions::default() { crate::operators::source_aliases(&input) } else { Vec::new() };
		let (declared_types, multi_word_types, program_functions) = if options == ParserOptions::default() {
			(scan_declared_types(&input), scan_multi_word_types(&input), scan_function_names(&input))
		} else {
			Default::default()
		};
		let names_end = blocks::names_block_closer(&chars, END_KEYWORD);
		let names_fi = blocks::names_block_closer(&chars, FI_KEYWORD);
		WarpParser {
			names_end,
			names_fi,
			closing_ends: Default::default(),
			user_operators,
			source_aliases,
			declared_types,
			multi_word_types,
			phrase_definitions: Default::default(),
			input,
			chars,
			pos: 0,
			line_nr: 1,
			column: 1,
			char: '\0',
			current_line,
			lines,
			options,
			context: Context::default(),
			key_variables: vec![],
			text_variables: Default::default(),
			variables: Default::default(),
			path_end: None,
			program_functions,
			parameters_in_scope: Vec::new(),
			type_fields: None,
			signed_list_element: None,
			generic_names: None,
			tag_block: false,
			pending_comment: None,
			after_function_keyword: false,
			continues_definition_head: false,
			elvis_operands: 0,
			finally_blocks: 0,
			functions: Default::default(),
			functions_with_parameters: Default::default(),
		}
	}

	/// Set the hint position to current parser position
	fn set_hint_pos(&self) {
		set_hint_position(self.line_nr, self.column);
	}

	pub fn parse(input: &str) -> Node {
		crate::compiler::phase_times::timed("parse", || Self::parse_with_options(input, ParserOptions::default()))
	}

	pub fn parse_with_options(input: &str, options: ParserOptions) -> Node {
		if let Some((line, column, closer)) = comment_hiding_a_closer(input).filter(|_| !options.data_mode) {
			let message = format!("`//` at {line}:{column} starts a comment that hides the closing `{closer}`: floor division is written a {FLOOR_DIVISION} b");
			return glued_floor_division(Diagnostic::default().message(message), input, line, column).into_error();
		}
		let mut parser = WarpParser::new_with_options(input.to_string(), options);
		let program = sugar::sugared(parser.parse_list_with_separators(None, Bracket::None));
		if options == ParserOptions::default() {
			crate::normalize::check_style(&program);
		}
		// marked Empty (`ø!`) is P73 force-unwrap, not a vacant program (is_nothing looks through Meta)
		if program.is_nothing() && crate::mutation::bang_target(&program).is_none() {
			Empty
		} else {
			program
		}
	}
}

/// The one element of `items`, moved out (cloning it copied a whole subtree, the whole program at the top)
fn only<T>(items: Vec<T>) -> T {
	items.into_iter().next().expect("one element")
}

// Tests moved to tests/parser/test_parser.rs

pub(crate) fn mentions(node: &Node, name: &str) -> bool {
	if crate::text::interpolation::template_mentions(node, |hole| mentions(hole, name)) {
		return true;
	}
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(left, _, right) => mentions(left, name) || mentions(right, name),
		Node::List(items, _, _) => items.iter().any(|item| mentions(item, name)),
		_ => false,
	}
}
