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
pub use declaration_scan::*;
mod collections;
pub use collections::*;

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
pub use literals::{shader_blocks, without_shader_texts};
use lookahead::*;
pub use scanning::read_atoms;
use scanning::*;
mod annotations;
mod math_notation;
mod rust_macros;
mod suffixes;
mod types;
pub(crate) use math_notation::*;
pub(crate) use types::*;
use unicode_normalization::UnicodeNormalization;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
/// What follows the scheme of a URL read as one text, whatever the scheme: `https://pannous.com`, `ssh://host` (card add-http)
const URL_MARK: &str = "://";
/// The item of the map a Julia dot call `f.(xs)` lowers to
const BROADCAST_ITEM: &str = "broadcast_item";
/// Words that make a parameter the rest parameter: Kotlin `vararg xs: Int`, C# `params int[] xs`
const REST_MARKERS: [&str; 2] = ["vararg", "params"];
const MAX_INTEGER_EXPONENT: i64 = 4096;
/// Literal suffixes of C, Java and C#, a tight conversion: `0.1f`, `0.1d` (double) → float; C's `0.1l` (long double) is
/// the litre unit instead (decision litre-suffix)
const LITERAL_SUFFIXES: [(char, &str); 4] = [('f', "float"), ('F', "float"), ('d', "float"), ('D', "float")];
/// Words that may precede the name of a global besides a type word (`int`, `long` … see `analyzer::type_word_kind`)
pub(crate) const ORDINAL_SUFFIXES: [&str; 4] = ["st", "nd", "rd", "th"];

/// Type names that take type arguments in angle brackets besides the plural and user types: `list<int>`, `map<text, int>`
const GENERIC_TYPE_HEADS: [&str; 7] = ["list", "array", "set", "map", "option", "result", "tuple"];
/// Other languages' list types: warp's list holds any elements, unboxed numbers too, so `Vec<int>` is `list of int`
const FOREIGN_LIST_HEADS: [&str; 6] = ["Vec", "vector", "Vector", "array", "Array", "ArrayList"];

const DECLARATION_MODIFIERS: [&str; 3] = ["export", "mutable", "mut"];
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
/// `function add`, `func add`: a reference to a function (P82), like `&add`
const FUNCTION_REFERENCE_WORDS: [&str; 2] = ["function", "func"];
/// `nonlocal y` in a nested function: it reads the enclosing function's current y (lowering/definitions/late_binding.rs)
const NONLOCAL_WORD: &str = "nonlocal";
/// Rust's endless `loop { … }`
const LOOP_WORD: &str = "loop";

const SIGNED_OPERAND_TOPIC: &str = "signed-operand";
const LEFT_ARROW_TOPIC: &str = "left-arrow";

/// Control words behind a statement, each lowering to `if`/`while`, negated for `unless`/`until`
/// Words that declare a type from a field block: `struct point{x:int y:int}`, `class contact {name email?}`
pub(crate) const TYPE_DECLARATION_WORDS: [&str; 2] = ["class", "struct"];
/// Words before a class declaration that change nothing in warp: `data class` (a warp class compares by value already),
/// `open`, `abstract`, `sealed`, `final`, visibility
const CLASS_MODIFIERS: [&str; 8] = ["data", "open", "abstract", "sealed", "final", "public", "private", "internal"];
/// The keywords of a field: Kotlin's primary constructor `class Point(val x: Int, var y: Int)`, Swift's `var count = 0`
pub(crate) const FIELD_KEYWORDS: [&str; 3] = ["val", "var", "let"];
/// `new Point(1, 2)`: the construction `Point(1, 2)`
const NEW_WORD: &str = "new";
/// `constant x = 3` is `const x = 3`, with a note naming const (card constant-alias)
const CONSTANT_ALIAS: &str = "constant";
const CONST_WORD: &str = "const";
/// Java's and TypeScript's `class Square implements Shape {…}`
pub const IMPLEMENTS_WORD: &str = "implements";
/// `enum Color {red, green}` (declarations::enum_object), Kotlin's `enum class`
const ENUM_WORD: &str = "enum";
/// Swift's `enum Shape { case circle(radius: Double) }`
const CASE_KEYWORD: &str = "case";
/// Go's `type Shape interface {…}` declares the trait Shape
const GO_INTERFACE_WORD: &str = "interface";
/// Go's `type Point struct {…}` declares the class Point
const GO_STRUCT_WORD: &str = "struct";
/// `type Node: gc struct {…}` (WebAssembly's GC structs): every warp class is one, the word adds nothing
const GC_WORD: &str = "gc";
/// `left: ref Node?`: WebAssembly's reference type word before a field's type
const REF_TYPE_WORD: &str = "ref";
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
/// Python's root class `class Point(object):`, no parent of its own
const PYTHON_ROOT_CLASS: &str = "object";
/// `record point{x:int y:int}` declares a type like `struct`, but `record` is also an everyday variable name:
/// it is a declaration only when a name and a field block follow
const RECORD_WORD: &str = "record";
/// `1 upto 10` excludes 10 (wiki/range.md), asked about because readers expect either
const UPTO: &str = "upto";
/// `10 down to 1` is `reverse(1 to 10)`
const DOWN_WORD: &str = "down";
const REVERSE_WORD: &str = "reverse";
const EXCLUSIVE_DOTS: &str = "..";
/// Topic of the Ask about `f x as T`, which casts the argument (card parameter-named-number)
const ARGUMENT_CAST: &str = "argument-cast";
/// Topic of the Ask about `for i in 0..n-1`, which Kotlin reads inclusive
const KOTLIN_RANGE: &str = "kotlin-range";
/// The control words that negate their condition: `unless c` is `if not c`, `until c` is `while not c`
const NEGATED_CONTROL_WORDS: [(&str, Op); 2] = [("unless", Op::If), ("until", Op::While)];
/// Words that test a value for being ø or falsy, sugar for `not x`; `failed` tests for an Error value (is_error)
const TEST_WORDS: [&str; 6] = ["empty", "missing", "absent", "unknown", "undefined", FAILED_WORD];
const FAILED_WORD: &str = "failed";
const EMPTY_WORD: &str = "empty";
/// Operators that may follow a suffix `!` directly: `x!+1`, `x!*2` (`!=` is the inequality)
const INFIX_AFTER_BANG: [char; 9] = ['+', '-', '*', '/', '%', '^', '<', '>', ')'];
/// Python's `elif`, Perl's and Ruby's `elsif`, PHP's `elseif`: all `else if`
const ELSE_IF_WORDS: [&str; 3] = ["elif", "elsif", "elseif"];
/// `x = data a and b`: the prefix takes the whole expression after it (card data-quoting); `data class` is a modifier
const DATA_KEYWORD: &str = "data";
const CLASS_KEYWORD: &str = "class";
/// `await job` waits for a task; its operand binds like the operand of a unary minus (Op::Neg)
const AWAIT_KEYWORD: &str = "await";
/// The words of a block of code, also glued to it: `go{…}` starts a task as `go {…}` does (lowering/control/go_blocks.rs,
/// card error-beep), `loop{…}` loops, `do{…}` runs its block (card glued-loop); no tags `go:{…}`
const GLUED_BLOCK_KEYWORDS: [&str; 3] = ["go", "loop", "do"];
/// `await all jobs`: every task of a list (P47)
const AWAIT_ALL_WORD: &str = "all";
/// `await x` binds its operand like unary minus
const AWAIT_OPERAND_BP: u8 = Op::Neg.binding_power().1;
/// Other languages' print words (wiki/Features.md); `log` stays the logarithm, `puts` the WASI text output
const PRINT_SPELLINGS: [&str; 2] = ["println", "writeln"];
/// `print a  print b`: statements separated by spaces only (user decision 2026-10-03: a loud error)
const TWO_STATEMENTS_ON_ONE_LINE: &str = "two statements on one line? separate them with `;` or a newline";
/// What ends a loop's iterable besides its body: the statement, a comprehension's bracket or condition (`if`)
const ITERABLE_ENDS: [char; 6] = [';', '}', ']', ')', ',', '\0'];
const IF_WORD: &str = "if";
/// Ruby/Lua blocks: `while c do … end`, `if c then … else … end`
const END_KEYWORD: &str = "end";
const ELIXIR_FUNCTION_KEYWORD: &str = "fn";
const PYTHON_LAMBDA_KEYWORD: &str = "lambda";
const ELSE_KEYWORD: &str = "else";
const END_BLOCK_OPENERS: [&str; 2] = ["do", "then"];
/// Ruby's field declarations `attr_accessor :x, :y`
const RUBY_FIELD_WORDS: [&str; 3] = ["attr_accessor", "attr_reader", "attr_writer"];
/// The words a Ruby `end` closes in a class body: `def … end`, `do … end`
const RUBY_END_OPENERS: [&str; 3] = ["def", "do", "class"];
const AMBIGUOUS_END: &str = "ambiguous `end`: it closes either the `then` or the `do`; as in Ruby and Lua every `then … end` and `do … end` needs its own: write `while c do … if x then … end end` or `while c { … if x { … } }`";
/// Keywords a `[` after never indexes: `in [1, 2]` and `return [x]` take a list
const UNINDEXABLE_KEYWORDS: [&str; 6] = ["in", "return", "yield", "then", "else", "do"];

/// The marker calls the parser leaves for `try X else Y` and `assert C else X`, lowered in `library_words`
pub const TRY_MARKER: &str = "try·else";
/// `after C return V` (wiki/thread.md) as the marker call `after·return(C, V)`, lowered by go_blocks into a waiting task
pub const AFTER_MARKER: &str = "after·return";
/// `class dog extends animal {…}`: the class named on the right is the parent (P117)
pub const EXTENDS_KEYWORD: &str = "extends";
/// The field of a sum type's variant with one unnamed payload, `Some(T)`: `Some(3).value`; several are value1, value2…
const VARIANT_FIELD: &str = "value";
/// `mixin Walker{…}` declares fields and methods classes take in: `class Duck with Walker, Swimmer {…}`
pub const MIXIN_WORD: &str = "mixin";
pub const WITH_KEYWORD: &str = "with";
/// The constructor of a class body, `init{…}` or `init(name){…}` (wiki/constructor.md, P162)
pub const CONSTRUCTOR_WORD: &str = "init";
/// The accessors of a class property, `get age() {…}`, `set age(v) {…}` (wiki/property.md)
pub const ACCESSOR_WORDS: [&str; 2] = ["get", "set"];
/// `static k = 3` in a class body: a member of the class, not of each instance (P122); kept as the annotation `@static`
pub const STATIC_KEYWORD: &str = "static";
const AFTER_KEYWORD: &str = "after";
/// `sleep 1s and print "x"`: an `and` between two statements runs them one after the other
const AND_KEYWORD: &str = "and";
/// Words a condition or subject follows: `xs where #it > 1` counts it, `where` takes no `#` index (card topo-sort-wasm);
/// `match -3 {…}` matches -3, no subtraction from `match` (card match-negative-subject)
const CLAUSE_WORDS: [&str; 3] = ["where", "match", "switch"];
/// Words between two operands besides the operator words (operators::is_operator_word): `of`, `matches`
const JOINING_WORDS: [&str; 2] = [OF_WORD, "matches"];

pub const ASSERT_MARKER: &str = "assert·else";
/// The words that start the fallback of `try X else Y`: `else`, classical `catch`, Python's `except` (P60)
const FALLBACK_WORDS: [&str; 3] = [ELSE_KEYWORD, "catch", "except"];
/// `try X catch Y finally Z`: Z runs after either, the value stays X's or Y's
const FINALLY_KEYWORD: &str = "finally";
/// The hidden name `try X finally Z` binds X's Error to, to hand it on as the value
const UNCAUGHT_ERROR: &str = "uncaught·error";
/// `check C` is `assert C` (user, card check-assert)
const GUARD_MARKERS: [(&str, &str); 3] = [("try", TRY_MARKER), ("assert", ASSERT_MARKER), ("check", ASSERT_MARKER)];
pub const ASSERTION_FAILED: &str = "assertion failed";

/// How tight a suffix `!` binds: below the index `#`, above `^` and arithmetic
const BANG_BP: u8 = Op::Hash.binding_power().0.midpoint(Op::Pow.binding_power().0);
const TO_WORD: &str = "to";
const TO_SENTENCE_WORD: &str = "To";
/// `to render text as pdf: …`: the word before a `to` definition's result type (card render-as)
const RESULT_TYPE_WORD: &str = "as";
/// `a[start:end]` calls the library word `slice`
const SLICE_WORD: &str = crate::words::SLICE;
pub(crate) const TIMES_WORD: &str = "times";
/// The call `n times text` is parsed to: the text repeated n times
pub const TEXT_TIMES: &str = "times·text";
/// The acknowledge-once note that a spaced `//` after code is a comment, not Python's floor division
const SLASH_COMMENT_TOPIC: &str = "slash-comment";
/// The severe warning on a `//` comment meant as floor division, `7//2` (decision floor-division)
pub const FLOOR_DIVISION_TOPIC: &str = "floor-division-comment";
const FLOOR_DIVISION: &str = "/!";
/// Pascal's floor division `7 div 2`, also the html tag `div {…}`
const DIV_WORD: &str = "div";
const FLOOR_DIVISION_ASSIGN: &str = "/!=";
const ELVIS_WORD: &str = "elvis";
/// `N times` takes everything up to an assignment: `n+1 times {…}`
const TIMES_BP: u8 = 50;

/// Superscript digits ⁰…⁹ in digit order: `3⁴` is 3^4 and a run of them is one exponent (`2¹⁰` is 2^10)
const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
/// A vulgar fraction such as ⅓ decomposes (NFKD) into numerator, this slash and denominator: 1⁄3
const FRACTION_SLASH: char = '⁄';

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

/// Superscript letters and the letters they raise: `2ⁿ` is 2^n
const SUPERSCRIPT_LETTERS: [(char, char); 25] = [
	('ᵃ', 'a'),
	('ᵇ', 'b'),
	('ᶜ', 'c'),
	('ᵈ', 'd'),
	('ᵉ', 'e'),
	('ᶠ', 'f'),
	('ᵍ', 'g'),
	('ʰ', 'h'),
	('ⁱ', 'i'),
	('ʲ', 'j'),
	('ᵏ', 'k'),
	('ˡ', 'l'),
	('ᵐ', 'm'),
	('ⁿ', 'n'),
	('ᵒ', 'o'),
	('ᵖ', 'p'),
	('ʳ', 'r'),
	('ˢ', 's'),
	('ᵗ', 't'),
	('ᵘ', 'u'),
	('ᵛ', 'v'),
	('ʷ', 'w'),
	('ˣ', 'x'),
	('ʸ', 'y'),
	('ᶻ', 'z'),
];

/// `floor_quotient(a, b)`: what `a // b` and `a div b` are, the Euclidean quotient that goes with `%`
/// (a == b*(a//b) + a%b): floor(a/b) for a positive divisor, ceil(a/b) for a negative one. Each operand is evaluated
/// once, and an exact quotient rounds exactly (wasm_emitter exact_euclid_div)
pub const FLOOR_QUOTIENT: &str = "floor_quotient";

/// `a | b`: the logical or, or `b(a)` when b names a function (pipes.rs)
const PIPE_GLYPH: &str = "|";

/// Read and parse a WARP file
pub fn parse_file(path: &str) -> Node {
	let options = if path.ends_with(".wit") { ParserOptions::wit() } else { ParserOptions::default() };
	match read_to_string(path) {
		Ok(content) => WarpParser::parse_with_options(&content, options),
		_ => error(&format!("Failed to read {}", path)),
	}
}

/// `=>` reads as in JS while parsing (Op::FatArrow: its binding, a lambda's body, a field holding a lambda); the program
/// has one arrow, `->` (user 2026-10-14: "equal for us, but not for others")
fn one_arrow(node: Node) -> Node {
	let node = match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(one_arrow(*body)) },
		other => crate::node::children_rewritten(other, one_arrow),
	};
	match node {
		Node::Key(parameters, Op::FatArrow, body) => Node::Key(parameters, Op::Arrow, body),
		other => other,
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
	placed(parse(template), crate::errors::diagnostic::line_info(at).as_ref())
}

/// Code a lowering pass writes, without positions: an error inside it names the nearest written position around it
pub fn parse_generated(code: &str) -> Node {
	placed(parse(code), None)
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
	base_indent: usize,
	/// Inside a block indented with spaces below a trailing `:` spaces count as indent too (elsewhere only tabs do)
	indent_counts_spaces: bool,
	options: ParserOptions,
	/// Inside an `if`/`while` condition `=` compares instead of assigning (wiki/Bad.md)
	equals_compares: bool,
	/// Inside the iterable of `for x in …` a block is the loop body, never an argument: `for i in 0..n {…}`
	in_for_header: bool,
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
	/// The `end`s and openers of the source per set of openers, indexed once on the first search for a closing `end`,
	/// so each `do`/`def` costs a lookup instead of a scan of the rest of the source
	closing_ends: std::cell::RefCell<std::collections::HashMap<String, blocks::ClosingEnds>>,
	/// The parameters of the definitions around: ` /name` after one divides, as after a variable; each definition's
	/// until the statement holding it ends (card path-parameter-scope)
	parameters_in_scope: Vec<String>,
	/// The binding power of a glued pair's value (`for:email`): that value is one atom, no call of what follows
	glued_pair_bp: Option<u8>,
	/// Where the innermost bracketed group opened (line, column): an unclosed one names it
	group_start: (usize, usize),
	/// While the `then` body of `if c: body else …` is parsed, `else` ends it instead of joining it
	stops_at_else: bool,
	/// Inside `class Name {…}`: its fields named like a constant (`pi = 3`, `pi:int`), which its methods read instead
	/// of the constant; None outside a type body
	type_fields: Option<std::collections::HashSet<String>>,
	/// The binding power of the `then` or `else` branch or the condition being parsed: there a braceless call takes a
	/// variable argument (`then count xs`, `if square x > 5`), as at assignment level
	branch_bp: Option<u8>,
	/// The position of the sign in `1 -1` read as the list `[1 -1]`: no enclosing expression subtracts it either (`x=1 -1`)
	signed_list_element: Option<usize>,
	/// Directly inside a list literal `[…]` without commas: a space and a glued sign start the next item (`[a -1]`,
	/// decision list-sign)
	in_spaced_list: bool,
	/// Inside `do … end`: the `end` keyword closes the statement list
	stops_at_end: bool,
	/// Inside Python's `f"…{x}…"`: braces are holes, `{{` and `}}` the braces themselves
	brace_holes: bool,
	/// The type parameters skipped after a function's name (`fn id<T>`), marked on the atom (P157)
	generic_names: Option<(String, Vec<String>)>,
	/// Parsing an argument of a braceless call at statement level (`sleep 1s …`): an `and` followed by a statement ends it
	in_command: bool,
	/// Parsing the block of a data literal (`a{ … }`, not a declared type's constructor): a spaced child `c { d:3 }` there
	/// is the child node of the glued `c{ d:3 }`, as no call with a block can be meant (card spaced-child)
	in_data_literal: bool,
	/// The next `{…}` is the block of a named tag (`ul{…}`, `ul {…}`): a repeated `key: value` there is a child, as
	/// repeated elements in XML/HTML (`ul{ li: "a" li: "b" }`), no duplicate key; a plain `{…}` is a map that reports
	/// one (user decision P176, card g-_bGo). Taken by the list it opens, so blocks inside it are not tag blocks
	tag_block: bool,
	/// Parsing the rules of a style sheet `style{ … }`: a blank before `.x` or `#x` is CSS's descendant combinator, which
	/// starts the next part of a selector instead of a member access (`#main .x`, card web-styles-parser)
	in_style_sheet: bool,
	/// Parsing the one argument of a braceless call (`square xs |> sum`): the pipeline after it takes the whole call
	pipe_takes_call: bool,
	/// The atoms read so far, in source order: the lexer's view of the source (read_atoms); a rewind forgets those read
	/// after its mark
	atoms: Vec<Atom>,
	/// A comment between two statements: it belongs to the next one (parse_value attaches it)
	pending_comment: Option<String>,
	/// The symbol parsed last was a function keyword (`def`, `function`): the next one is the function's name
	after_function_keyword: bool,
	/// The expression continue_expr goes on with heads a definition: `foo` of `def foo: hi=2*2`
	continues_definition_head: bool,
	/// inside Elixir's capture `&(…)`, where `&1` is its first argument
	in_capture: bool,
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
		let names_end = blocks::names_end(&chars);
		WarpParser {
			names_end,
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
			base_indent: 0,
			indent_counts_spaces: false,
			options,
			equals_compares: false,
			in_for_header: false,
			key_variables: vec![],
			text_variables: Default::default(),
			variables: Default::default(),
			path_end: None,
			program_functions,
			parameters_in_scope: Vec::new(),
			glued_pair_bp: None,
			group_start: (0, 0),
			stops_at_else: false,
			type_fields: None,
			branch_bp: None,
			signed_list_element: None,
			in_spaced_list: false,
			stops_at_end: false,
			brace_holes: false,
			generic_names: None,
			in_command: false,
			in_data_literal: false,
			tag_block: false,
			in_style_sheet: false,
			pipe_takes_call: false,
			atoms: vec![],
			pending_comment: None,
			after_function_keyword: false,
			continues_definition_head: false,
			in_capture: false,
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
		Self::parse_with_options(input, ParserOptions::default())
	}

	pub fn parse_with_options(input: &str, options: ParserOptions) -> Node {
		if let Some((line, column, closer)) = comment_hiding_a_closer(input).filter(|_| !options.data_mode) {
			let message = format!("`//` at {line}:{column} starts a comment that hides the closing `{closer}`: floor division is written a {FLOOR_DIVISION} b");
			return glued_floor_division(Diagnostic::default().message(message), input, line, column).into_error();
		}
		let mut parser = WarpParser::new_with_options(input.to_string(), options);
		let program = one_arrow(parser.parse_list_with_separators(None, Bracket::None));
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

/// What the lexer reads (card self-lexer, lib/compiler/lexer.warp is its warp twin)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomKind {
	Number,
	Text,
}

/// An atom the parser read: where it starts (line and column from 1, a column per character) and what was written
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Atom {
	pub kind: AtomKind,
	pub line: usize,
	pub column: usize,
	pub written: String,
}

/// The one element of `items`, moved out (cloning it copied a whole subtree, the whole program at the top)
fn only<T>(items: Vec<T>) -> T {
	items.into_iter().next().expect("one element")
}

/// In data only JSON's words are literals; aliases like `yes`, `no`, `none`, `pi` stay symbols.
const DATA_WORD_LITERALS: [&str; 3] = ["true", "false", "null"];

/// The glyphs that are true or false (card keyword-glyphs), longest spelling first: `✔️` is `✔` with an emoji variation
/// selector. Most start an emoji, which would otherwise be a codepoint (atoms.rs, parse_emoji)
pub(crate) const TRUTH_GLYPHS: [(&str, bool); 13] = [
	("✔️", true),
	("✓️", true),
	("☑️", true),
	("⊤", true),
	("✓", true),
	("✔", true),
	("☑", true),
	("✅", true),
	("🗸", true),
	("🗹", true),
	("⊥", false),
	("❌", false),
	("✗", false),
];

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

const COMPARISON_FIRST: &str = "the comparison first, as C";
