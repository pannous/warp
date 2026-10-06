use crate::diagnostic::{ask, reading, Ask, Diagnostic, Fallback};
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Symbol};
use crate::extensions::reals::{Exact, Rational, Real};
use crate::node::{error, key_ops, Bracket, Node, Separator};
use crate::operators::{glyph_operator, is_function_keyword, Op};
use crate::normalize::{hints as norm, set_hint_position, ListTypeStyle};
use log::warn;
use std::fs::read_to_string;
mod user_operators;
mod scanning;
mod xml;
mod lookahead;
pub use lookahead::PREFIX_OPERATOR_WORDS;
mod atoms;
mod expressions;
mod statements;
mod suffixes;
mod literals;
mod lists;
use unicode_normalization::UnicodeNormalization;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
/// The unit words a for loop walks a text by, the item being `it` (wiki/string.md)
/// The schemes of a URL read as one text: `https://pannous.com`
const URL_SCHEMES: [&str; 7] = ["http", "https", "ftp", "file", "data", "ws", "wss"];
const UNIT_LOOP_WORDS: [&str; 4] = ["chars", "characters", "codepoints", "bytes"];
const BYTES_WORD: &str = "bytes";
pub const IT_WORD: &str = "it";
/// The item of the map a Julia dot call `f.(xs)` lowers to
const BROADCAST_ITEM: &str = "broadcast_item";
const MAP_WORD: &str = "map";
/// Words that make a parameter the rest parameter: Kotlin `vararg xs: Int`, C# `params int[] xs`
const REST_MARKERS: [&str; 2] = ["vararg", "params"];
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
/// Modifiers of other languages with no meaning in wasp: skipped with a note (user decision P78)
const FOREIGN_MODIFIERS: [&str; 19] = ["public", "private", "protected", "internal", "static", "extern", "external", "C", "inline", "local",
	"virtual", "override", "abstract", "constexpr", "volatile", "thread_local", "synchronized", "transient", "native"];
/// Modifiers with a wasp meaning that change nothing before a function definition (a function is global and constant)
const DEFINITION_MODIFIERS: [&str; 7] = ["global", "export", "import", "const", "final", "mutable", "mut"];
const FOREIGN_MODIFIER_TOPIC: &str = "foreign-modifier";
/// `function add`, `func add`: a reference to a function (P82), like `&add`
const FUNCTION_REFERENCE_WORDS: [&str; 2] = ["function", "func"];
/// `nonlocal y` in a nested function: it reads the enclosing function's current y (lowering/late_binding.rs)
const NONLOCAL_WORD: &str = "nonlocal";

const SIGNED_OPERAND_TOPIC: &str = "signed-operand";
const LEFT_ARROW_TOPIC: &str = "left-arrow";
/// `xs .+ 4`: an arithmetic operator behind a dot applies to each element (D3)
const ELEMENT_WISE_OPERATORS: [(char, Op); 5] = [('+', Op::Add), ('-', Op::Sub), ('*', Op::Mul), ('/', Op::Div), ('^', Op::Pow)];

/// Control words behind a statement, each lowering to `if`/`while`, negated for `unless`/`until`
/// Words that declare a type from a field block: `struct point{x:int y:int}`, `class contact {name email?}`
const TYPE_DECLARATION_WORDS: [&str; 2] = ["class", "struct"];
/// Words before a class declaration that change nothing in wasp: `data class` (a wasp class compares by value already),
/// `open`, `abstract`, `sealed`, `final`, visibility
const CLASS_MODIFIERS: [&str; 8] = ["data", "open", "abstract", "sealed", "final", "public", "private", "internal"];
/// The keywords of a field in a primary constructor `class Point(val x: Int, var y: Int)`
const FIELD_KEYWORDS: [&str; 3] = ["val", "var", "let"];
/// `new Point(1, 2)`: the construction `Point(1, 2)`
const NEW_WORD: &str = "new";
/// Java's and TypeScript's `class Square implements Shape {…}`
const IMPLEMENTS_WORD: &str = "implements";
/// The got-it topic of a class naming its traits (`implements Shape`, Swift's `: Shape`)
const CONFORMANCE_TOPIC: &str = "conformance-list";
/// `enum Color {red, green}` (declarations::enum_object), Kotlin's `enum class`
const ENUM_WORD: &str = "enum";
/// Go's `type Shape interface {…}` declares the trait Shape
const GO_INTERFACE_WORD: &str = "interface";
/// Go's `type Point struct {…}` declares the class Point
const GO_STRUCT_WORD: &str = "struct";
/// C++'s and C#'s `operator +(o)`: the method of `+` named by its glyph
const OPERATOR_WORD: &str = "operator";
/// Words before a member of a class body that change nothing in wasp: Swift's `mutating func`, visibility, `override`
pub const MEMBER_MODIFIERS: [&str; 11] = ["mutating", "override", "public", "private", "protected", "internal", "fileprivate", "open", "final", "async", "operator"];
/// Python's root class `class Point(object):`, no parent of its own
const PYTHON_ROOT_CLASS: &str = "object";
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
/// `await all jobs`: every task of a list (P47)
const AWAIT_ALL_WORD: &str = "all";
/// `await x` binds its operand like unary minus
const AWAIT_OPERAND_BP: u8 = Op::Neg.binding_power().1;
const PRINT_WORD: &str = "print";
/// `print a  print b`: statements separated by spaces only (user decision 2026-10-03: a loud error)
const TWO_STATEMENTS_ON_ONE_LINE: &str = "two statements on one line? separate them with `;` or a newline";
const IN_KEYWORD: &str = "in";
/// Ruby/Lua blocks: `while c do … end`, `if c then … else … end`
const END_KEYWORD: &str = "end";
const ELIXIR_FUNCTION_KEYWORD: &str = "fn";
const PYTHON_LAMBDA_KEYWORD: &str = "lambda";
const ELSE_KEYWORD: &str = "else";
const END_BLOCK_OPENERS: [&str; 2] = ["do", "then"];
/// The receiver a Ruby instance variable `@x` reads, `self.x`
const RECEIVER_WORD: &str = "self";
/// Ruby's field declarations `attr_accessor :x, :y`
const RUBY_FIELD_WORDS: [&str; 3] = ["attr_accessor", "attr_reader", "attr_writer"];
/// The words a Ruby `end` closes in a class body: `def … end`, `do … end`
const RUBY_END_OPENERS: [&str; 3] = ["def", "do", "class"];
const AMBIGUOUS_END: &str = "ambiguous `end`: it closes either the `then` or the `do`; as in Ruby and Lua every `then … end` and `do … end` needs its own: write `while c do … if x then … end end` or `while c { … if x { … } }`";
/// Keywords a `[` after never indexes: `in [1, 2]` and `return [x]` take a list
const UNINDEXABLE_KEYWORDS: [&str; 6] = ["in", "return", "yield", "then", "else", "do"];

fn is_print_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Symbol(word) if word == PRINT_WORD)
}

/// `print` or the call `print(…)`: a print statement starts here
pub(crate) fn starts_print(node: &Node) -> bool {
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
pub(crate) fn print_call(arguments: impl IntoIterator<Item = Node>) -> Node {
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
	/// Julia's dot call `f.(xs)`, `add.(xs, 10)`: f mapped over the first argument
	DotCall,
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
			// Java's and Scala's `import math.*`: a glob, nothing after the `*`, is the module whole
			SpecialInfix::ElementWise(Op::Mul) if matches!(operand, Empty) => {
				crate::normalize::hint(&format!("{}.*", lhs.serialize()), &lhs.serialize(), "wasp imports a module whole");
				lhs
			}
			SpecialInfix::ElementWise(op) => crate::analyzer::element_wise(lhs, op, operand),
			SpecialInfix::Membership => Node::List(vec![lhs, Symbol(IN_KEYWORD.to_string()), operand], Bracket::None, Separator::Space),
			SpecialInfix::DotCall => dot_call(lhs, operand),
		}
	}
}

/// `f.(xs, a)` → `map(xs, broadcast_item => f(broadcast_item, a))`
fn dot_call(function: Node, arguments: Node) -> Node {
	let mut arguments = match arguments.drop_meta() {
		Node::List(items, Bracket::Round, _) if !items.is_empty() => items.clone(),
		_ => vec![arguments],
	};
	// a number broadcasts as itself (Julia, Elixir's `f.(4)`): the call
	if matches!(arguments[0].drop_meta(), Node::Number(_)) {
		return Node::List([vec![function], arguments].concat(), Bracket::Round, Separator::None);
	}
	let list = arguments.remove(0);
	let item = Symbol(BROADCAST_ITEM.to_string());
	let call = Node::List([vec![function, item.clone()], arguments].concat(), Bracket::Round, Separator::None);
	let each = Node::Key(Box::new(item), Op::FatArrow, Box::new(call));
	Node::List(vec![Symbol(MAP_WORD.to_string()), list, each], Bracket::Round, Separator::None)
}

/// `for int in xs` as the explicit filter `for x in xs.filter(x => x is int)`, the body unchanged; None when the body
/// names the item by the type word (then the body would change too)
fn type_filter_header(name: &str, iterable: &Node, body: &Node) -> Option<(String, String)> {
	if mentions(body, name) {
		return None;
	}
	let collection = crate::normalize::operand_text(iterable);
	Some((format!("for {name} in {collection}"), format!("for x in {collection}.filter(x => x is {name})")))
}

fn is_unindexable_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if UNINDEXABLE_KEYWORDS.contains(&word.as_str()))
}
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
const AFTER_KEYWORD: &str = "after";
/// `sleep 1s and print "x"`: an `and` between two statements runs them one after the other
const AND_KEYWORD: &str = "and";
/// Words after `and` that continue an expression rather than start a statement: `a and b or c`
const CONTINUING_WORDS: [&str; 7] = ["and", "or", "xor", "then", "else", "is", "in"];
pub const ASSERT_MARKER: &str = "assert·else";
/// The words that start the fallback of `try X else Y`: `else`, classical `catch`, Python's `except` (P60)
const FALLBACK_WORDS: [&str; 3] = [ELSE_KEYWORD, "catch", "except"];
/// `try X catch Y finally Z`: Z runs after either, the value stays X's or Y's
const FINALLY_KEYWORD: &str = "finally";
const GUARD_MARKERS: [(&str, &str); 2] = [("try", TRY_MARKER), ("assert", ASSERT_MARKER)];
/// `nand` and its glyph pair, both `not (a and b)`
/// `name` in a loop body as the item: every bare use, not the head of a call `name(…)`
fn item_named(node: Node, name: &str, item: &Node) -> Node {
	match node {
		Symbol(symbol) if symbol == name => item.clone(),
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Symbol(head)) if head == name) => {
			let mut items = items.into_iter();
			let head = items.next().expect("a head");
			Node::List(std::iter::once(head).chain(items.map(|part| item_named(part, name, item))).collect(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| item_named(child, name, item)),
	}
}

/// A list literal whose items all are literals of the type: `[1, 2]` for `number`
fn literal_items_of_type(iterable: &Node, type_name: &str) -> bool {
	let Node::List(items, Bracket::Square, _) = iterable.drop_meta() else { return false };
	let spec = crate::analyzer::type_word_kind(type_name).map(|_| type_name).unwrap_or(type_name);
	items.iter().all(|item| match item.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) => crate::type_tests::type_matches(&item.drop_meta().kind().to_string(), spec),
		_ => false,
	})
}

/// `keys(m)`, `m.keys`, `m.keys()`: an iterable whose items are a map's keys
fn iterates_keys(iterable: &Node) -> bool {
	let is_keys_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if word == "keys" || word == crate::library_words::MAP_KEYS);
	let is_keys_call = |call: &Node| is_keys_word(call) || matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_keys_word));
	match iterable.drop_meta() {
		Node::Key(_, Op::Dot, method) => is_keys_call(method),
		call => is_keys_call(call),
	}
}

/// The got-it topic of a filtering loop (`for friend in xs`, `for (it>2) in xs`)
pub(crate) const FILTER_LOOP_TOPIC: &str = "for-filter";
/// Built-in adjectives of a loop filter `(even number)`, when no function of that name is defined
const EVEN_WORD: &str = "even";
const ODD_WORD: &str = "odd";
/// How tight a suffix `!` binds: below the index `#`, above `^` and arithmetic
const BANG_BP: u8 = Op::Hash.binding_power().0.midpoint(Op::Pow.binding_power().0);
const NAND_SPELLINGS: [&str; 3] = ["nand", "¬&", "⊼"];
const TO_WORD: &str = "to";
const TO_SENTENCE_WORD: &str = "To";
/// `to greet p do …`: the word between a `to` definition's parameters and its body
const DO_WORD: &str = "do";
const OF_WORD: &str = "of";
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

/// Superscript letters and the letters they raise: `2ⁿ` is 2^n
const SUPERSCRIPT_LETTERS: [(char, char); 25] = [('ᵃ', 'a'), ('ᵇ', 'b'), ('ᶜ', 'c'), ('ᵈ', 'd'), ('ᵉ', 'e'), ('ᶠ', 'f'), ('ᵍ', 'g'),
	('ʰ', 'h'), ('ⁱ', 'i'), ('ʲ', 'j'), ('ᵏ', 'k'), ('ˡ', 'l'), ('ᵐ', 'm'), ('ⁿ', 'n'), ('ᵒ', 'o'), ('ᵖ', 'p'), ('ʳ', 'r'),
	('ˢ', 's'), ('ᵗ', 't'), ('ᵘ', 'u'), ('ᵛ', 'v'), ('ʷ', 'w'), ('ˣ', 'x'), ('ʸ', 'y'), ('ᶻ', 'z')];

fn superscript_letter(ch: char) -> Option<char> {
	SUPERSCRIPT_LETTERS.iter().find(|(raised, _)| *raised == ch).map(|(_, letter)| *letter)
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
	// every such glyph is a numeric character outside ASCII: asked of every identifier character, this skips the
	// decomposition of vulgar_fraction for all others
	!ch.is_ascii() && ch.is_numeric() && (superscript_digit(ch).is_some() || vulgar_fraction(ch).is_some())
}

fn is_identifier_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

/// `floor_quotient(a, b)`: what `a // b` and `a div b` are, the Euclidean quotient that goes with `%`
/// (a == b*(a//b) + a%b): floor(a/b) for a positive divisor, ceil(a/b) for a negative one. Each operand is evaluated
/// once, and an exact quotient rounds exactly (wasm_emitter exact_euclid_div)
pub const FLOOR_QUOTIENT: &str = "floor_quotient";

/// `a | b`: the logical or, or `b(a)` when b names a function (pipes.rs)
const PIPE_GLYPH: &str = "|";
/// `|>` binds below range and arithmetic, above `as` and comparisons
const PIPELINE_BINDING_POWER: (u8, u8) = (127, 128);

/// `value |> f(args)` → `f(value, args)`, `value |> f` → `f(value)`; a braceless call is grouped, one argument:
/// `square xs |> filter(p)` → `filter((square xs), p)`
pub(crate) fn piped(value: Node, stage: Node) -> Node {
	let value = match value.drop_meta() {
		Node::List(_, Bracket::None, Separator::Space) => Node::List(vec![value], Bracket::Round, Separator::None),
		_ => value,
	};
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
	/// The binding power of a glued pair's value (`for:email`): that value is one atom, no call of what follows
	glued_pair_bp: Option<u8>,
	/// Where the innermost bracketed group opened (line, column): an unclosed one names it
	group_start: (usize, usize),
	/// While the `then` body of `if c: body else …` is parsed, `else` ends it instead of joining it
	stops_at_else: bool,
	/// Inside `class Name {…}`: its fields named like a constant (`pi = 3`, `pi:int`), which its methods read instead
	/// of the constant; None outside a type body
	type_fields: Option<std::collections::HashSet<String>>,
	/// The binding power of the `then` or `else` branch being parsed: there it is a statement, so a braceless call takes a
	/// variable argument (`then count xs`), as at assignment level
	branch_bp: Option<u8>,
	/// The position of the sign in `1 -1` read as the list `[1 -1]`: no enclosing expression subtracts it either (`x=1 -1`)
	signed_list_element: Option<usize>,
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
	/// Parsing the one argument of a braceless call (`square xs |> sum`): the pipeline after it takes the whole call
	pipe_takes_call: bool,
	/// `N times` loops parsed so far, numbering their hidden counters
	times_loops: usize,
	/// A comment between two statements: it belongs to the next one (parse_value attaches it)
	pending_comment: Option<String>,
	/// The symbol parsed last was a function keyword (`def`, `function`): the next one is the function's name
	after_function_keyword: bool,
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
const USER_SUFFIX_BP: u8 = Op::Square.binding_power().0;
const USER_PREFIX_RIGHT_BP: u8 = Op::Neg.binding_power().1;
const USER_INFIX_BP: (u8, u8) = Op::Add.binding_power();
/// `operator ⊕ has precedence above *`: ⊕ binds this much tighter than `*`; the built-in levels are at least 5 apart
const PRECEDENCE_STEP: u8 = 2;
const PRECEDENCE_DIRECTIONS: [(&str, bool); 2] = [("above", true), ("below", false)];

/// A glyph a program may declare as an operator: symbols with a non-ASCII character (`‼`, `⊕`), never a letter or digit.
/// Superscript digits and signs too (P48, user-decided) (`suffix operator ³`, `prefix operator ⁻`, wiki/operator.md)
fn is_operator_glyph(glyph: &str) -> bool {
	let is_superscript = |c: char| superscript_digit(c).is_some() || superscript_sign(c).is_some();
	!glyph.is_empty() && glyph.chars().all(|c| is_superscript(c) || (!c.is_alphanumeric() && !c.is_whitespace())) && !glyph.is_ascii()
}

/// `suffix operator ⁰ := …` and the short `suffix ⁰ := …` (P48): the glyph and the words after `:=`
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
	let words: Vec<&str> = source.split_whitespace().collect();
	words.windows(2)
		.filter(|pair| TYPE_DECLARATION_WORDS.contains(&pair[0]) || pair[0] == RECORD_WORD || pair[0] == "type")
		// the name right after the word: `class Point{`, `class Point(val x: Int)`, `record Point(int X)`, `class P:`;
		// `record = find(…)` declares nothing
		.filter_map(|pair| {
			let name: String = pair[1].chars().take_while(|ch| is_identifier_char(*ch)).collect();
			let rest = &pair[1][name.len()..];
			(rest.is_empty() || rest.starts_with(['(', '{', '<', ':', ';'])).then_some(name)
		})
		.filter(|name| is_plain_name(name))
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
	let is_type_word = |node: &Node| matches!(node.drop_meta(), Symbol(word) if crate::analyzer::type_word_kind(word).is_some());
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) => match items.as_slice() {
			[type_word, name] if is_type_word(type_word) && matches!(name.drop_meta(), Symbol(_)) => Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone())),
			// Kotlin `vararg xs: Int`, C# `params int[] xs`: the rest parameter `*xs` (lowering/variadic.rs)
			[marker, .., name] if matches!(marker.drop_meta(), Symbol(word) if REST_MARKERS.contains(&word.as_str())) => match name.drop_meta() {
				Symbol(name) => Symbol(format!("{}{name}", crate::tuples::STARRED)),
				Node::Key(name, Op::Colon, _) => Symbol(format!("{}{}", crate::tuples::STARRED, name.name())),
				_ => arguments,
			},
			// `int b = 2` (C#, C++): `b:int = 2`
			[type_word, default] if is_type_word(type_word) => match default.drop_meta() {
				Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Symbol(_)) => {
					Node::Key(Box::new(Node::Key(name.clone(), Op::Colon, Box::new(type_word.clone()))), Op::Assign, value.clone())
				}
				_ => arguments,
			},
			_ => arguments,
		},
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
		// most sources are NFC already: normalizing them only copies them, slowly in a debug build
		let input: String = match unicode_normalization::is_nfc_quick(input.chars()) {
			unicode_normalization::IsNormalized::Yes => input,
			_ => input.nfc().collect(),
		};
		let input = if options.data_mode { input } else { crate::uniscript_entities::expand_entities(&input) };
		let lines: Vec<String> = input.lines().map(String::from).collect();
		let current_line = lines.first().cloned().unwrap_or_default();
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
			lines,
			base_indent: 0,
			indent_counts_spaces: false,
			options,
			equals_compares: false,
			in_for_header: false,
			key_variables: vec![],
			glued_pair_bp: None,
			group_start: (0, 0),
			stops_at_else: false,
			type_fields: None,
			branch_bp: None,
			signed_list_element: None,
			stops_at_end: false,
			brace_holes: false,
			generic_names: None,
			in_command: false,
			in_data_literal: false,
			pipe_takes_call: false,
			times_loops: 0,
			pending_comment: None,
			after_function_keyword: false,
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
			return error(&format!("`//` at {line}:{column} starts a comment that hides the closing `{closer}`: floor division is written glued, a//b"));
		}
		let mut parser = WaspParser::new_with_options(input.to_string(), options);
		let program = parser.parse_list_with_separators(None, Bracket::None);
		if options == ParserOptions::default() {
			crate::normalize::check_style(&program);
		}
		// marked Empty (`ø!`) is P73 force-unwrap, not a vacant program (is_nothing looks through Meta)
		if program.is_nothing() && crate::mutation::bang_target(&program).is_none() { Empty } else { program }
	}

}

/// The one element of `items`, moved out (cloning it copied a whole subtree, the whole program at the top)
fn only<T>(items: Vec<T>) -> T {
	items.into_iter().next().expect("one element")
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
	let (chained, grouped) = (format!("{left} and {middle} {symbol} {right}"), format!("({left}) {symbol} {right}"));
	Diagnostic::default().fix(format!("{chained} or {grouped}")).message(format!("ambiguous: equality does not chain in {left} {symbol} {right}"))
		.offer("both equalities, as Python chains them", format!("{left} {symbol} {right}"), chained)
		.offer("compare the first result, as C does", format!("{left} {symbol} {right}"), grouped)
}

const COMPARISON_FIRST: &str = "the comparison first, as C";

/// `3 & 4 == 4` is `(3&4)==4` in Python but `3 & (4==4)` in C: `&`/`|` next to an ungrouped comparison is ambiguous.
/// The word forms `and`/`or` read unambiguously and are not affected: `x==1 and y==2`.
fn logic_mixed_with_comparison(lhs: &Node, symbol: char, rhs: &Node) -> Option<Diagnostic> {
	let comparison = |node: &Node| match node.drop_meta() {
		Node::Key(left, op, right) if op.is_comparison() => Some((left.serialize(), op.as_str(), right.serialize())),
		_ => None,
	};
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let readings = match (comparison(lhs), comparison(rhs)) {
		(None, Some((a, op, b))) => vec![(COMPARISON_FIRST.to_string(), format!("{left} {symbol} ({a} {op} {b})")), (format!("`{symbol}` first, as Python"), format!("({left} {symbol} {a}) {op} {b}"))],
		(Some((a, op, b)), None) => vec![(COMPARISON_FIRST.to_string(), format!("({a} {op} {b}) {symbol} {right}")), (format!("`{symbol}` first, as Python"), format!("{a} {op} ({b} {symbol} {right})"))],
		(Some(_), Some(_)) => vec![("both comparisons first".to_string(), format!("({left}) {symbol} ({right})"))],
		(None, None) => return None,
	};
	let word = if symbol == '&' { "and" } else { "or" };
	let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
	let written = format!("{left} {symbol} {right}");
	let diagnostic = Diagnostic::default().fix(forms.join(" or "))
		.message(format!("ambiguous: `{symbol}` mixed with a comparison in {written}; group it or write `{word}`"));
	Some(readings.iter().fold(diagnostic, |diagnostic, (meaning, form)| diagnostic.offer(meaning, &written, form)))
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

/// `xs#a..b` reads as the range from the value xs#a; P58 (user: "create a strong warning"): a warning naming the slice
/// `xs#(a..b)` and the range `(xs#a)..b`
fn hash_range_warning(lhs: &Node, op: Op, written: &str, rhs: &Node) -> Option<Diagnostic> {
	let Node::Key(target, Op::Hash, index) = lhs.drop_meta() else { return None };
	if !matches!(op, Op::Range | Op::To) {
		return None;
	}
	let (target, index, end) = (target.serialize(), index.serialize(), rhs.serialize());
	let (target, index, end) = (target.trim(), index.trim(), end.trim());
	let (as_written, slice, range) = (format!("{target}#{index}{written}{end}"), format!("{target}#({index}{written}{end})"), format!("({target}#{index}){written}{end}"));
	let message = format!("{as_written} is the range from the value {target}#{index}: for the slice write {slice}, for the range write {range}");
	Some(Diagnostic::at(lhs, message).offer("the slice", &as_written, slice).offer("the range from the value", &as_written, range))
}

/// `xs#(a…b)`, `xs#(a..b)`: the 0-based start and exclusive end of a 1-based slice (`xs#a` is 1-based); only a range in
/// parentheses, `xs#a..b` stays the range from the value xs#a
fn hash_slice_bounds(index: &Node) -> Option<(Node, Node)> {
	let Node::List(items, Bracket::Round, _) = index.drop_meta() else { return None };
	let [range] = items.as_slice() else { return None };
	let Node::Key(start, op @ (Op::Range | Op::To), end) = range.drop_meta() else { return None };
	let minus_one = |bound: &Node| Node::Key(Box::new(bound.clone()), Op::Sub, Box::new(Node::Number(Number::Int(1))));
	let end = if *op == Op::To { end.as_ref().clone() } else { minus_one(end) };
	Some((minus_one(start), end))
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

/// A braceless call `sleep 1s`, `f 3`: a word applied to what follows it
fn is_command(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && matches!(items[0].drop_meta(), Symbol(_)))
}
