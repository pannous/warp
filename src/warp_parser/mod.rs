use crate::diagnostic::{ask, reading, Ask, Diagnostic, Fallback};
use crate::extensions::numbers::Number;
use crate::extensions::reals::{Exact, Rational, Real};
use crate::meta::LineInfo;
use crate::node::call;
use crate::node::Node::{Empty, Symbol};
use crate::node::{error, key_ops, Bracket, Node, Separator};
use crate::normalize::{hints as norm, set_hint_position, ListTypeStyle};
use crate::operators::{glyph_operator, is_function_keyword, Op};
use log::warn;
use std::fs::read_to_string;

use crate::node::ATTRIBUTE_MARK;
mod declaration_scan;
pub use declaration_scan::*;
mod subscripts;
pub use subscripts::*;

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
mod scanning;
mod user_operators;
mod xml;
pub use lookahead::PREFIX_OPERATOR_WORDS;
mod atoms;
mod expressions;
mod lists;
mod literals;
pub use literals::shader_blocks;
mod statements;
mod suffixes;
use unicode_normalization::UnicodeNormalization;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
/// What follows the scheme of a URL read as one text, whatever the scheme: `https://pannous.com`, `ssh://host` (card add-http)
const URL_MARK: &str = "://";
pub const IT_WORD: &str = "it";
/// The item of the map a Julia dot call `f.(xs)` lowers to
const BROADCAST_ITEM: &str = "broadcast_item";
const MAP_WORD: &str = "map";
/// Words that make a parameter the rest parameter: Kotlin `vararg xs: Int`, C# `params int[] xs`
const REST_MARKERS: [&str; 2] = ["vararg", "params"];
const MAX_INTEGER_EXPONENT: i64 = 4096;
/// Literal suffixes of C, Java and C#, a tight conversion: `0.1f`, `0.1d` (double) → float; `0.1l` (long double) → exact, the default anyway
const LITERAL_SUFFIXES: [(char, &str); 6] = [
	('f', "float"),
	('F', "float"),
	('d', "float"),
	('D', "float"),
	('l', "exact"),
	('L', "exact"),
];
/// Words that may precede the name of a global besides a type word (`int`, `long` … see `analyzer::type_word_kind`)
pub(crate) const ORDINAL_SUFFIXES: [&str; 4] = ["st", "nd", "rd", "th"];

/// Type names that take type arguments in angle brackets besides the plural and user types: `list<int>`, `map<text, int>`
const GENERIC_TYPE_HEADS: [&str; 7] = ["list", "array", "set", "map", "option", "result", "tuple"];
/// Other languages' list types: warp's list holds any elements, unboxed numbers too, so `Vec<int>` is `list of int`
const FOREIGN_LIST_HEADS: [&str; 6] = ["Vec", "vector", "Vector", "array", "Array", "ArrayList"];
const LIST_HEAD: &str = "list";

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
/// `nonlocal y` in a nested function: it reads the enclosing function's current y (lowering/late_binding.rs)
const NONLOCAL_WORD: &str = "nonlocal";

const SIGNED_OPERAND_TOPIC: &str = "signed-operand";
const LEFT_ARROW_TOPIC: &str = "left-arrow";

/// Control words behind a statement, each lowering to `if`/`while`, negated for `unless`/`until`
/// Words that declare a type from a field block: `struct point{x:int y:int}`, `class contact {name email?}`
pub(crate) const TYPE_DECLARATION_WORDS: [&str; 2] = ["class", "struct"];
/// Words before a class declaration that change nothing in warp: `data class` (a warp class compares by value already),
/// `open`, `abstract`, `sealed`, `final`, visibility
const CLASS_MODIFIERS: [&str; 8] = [
	"data", "open", "abstract", "sealed", "final", "public", "private", "internal",
];
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
const IS_IN_PHRASE: &str = "is in";
/// `xs contains x` (and its synonyms, lowering/library_words.rs COLLECTION_CONTAINS): `x in xs` the other way round
const CONTAINMENT_WORDS: [&str; 3] = ["contains", "has", "includes"];
/// `for each item in basket` is `for item in basket`
const EACH_WORD: &str = "each";
/// `10 down to 1` is `reverse(1 to 10)`
const DOWN_WORD: &str = "down";
const REVERSE_WORD: &str = "reverse";
const EXCLUSIVE_DOTS: &str = "..";
/// Topic of the Ask about `for i in 0..n-1`, which Kotlin reads inclusive
const KOTLIN_RANGE: &str = "kotlin-range";
const STATEMENT_MODIFIERS: [(&str, Op, bool); 4] = [
	("if", Op::If, false),
	("unless", Op::If, true),
	("while", Op::While, false),
	("until", Op::While, true),
];
/// Words that test a value for being ø or falsy, sugar for `not x`; `failed` tests for an Error value (is_error)
const TEST_WORDS: [&str; 6] = ["empty", "missing", "absent", "unknown", "undefined", FAILED_WORD];
const FAILED_WORD: &str = "failed";
/// The runtime test of `x failed` (wasm_emitter/text_builtins.rs)
const IS_ERROR_CALL: &str = "is_error";
const COUNT_CALL: &str = "count";
const EMPTY_WORD: &str = "empty";
/// Operators that may follow a suffix `!` directly: `x!+1`, `x!*2` (`!=` is the inequality)
const INFIX_AFTER_BANG: [char; 9] = ['+', '-', '*', '/', '%', '^', '<', '>', ')'];
/// Python's `elif`, Perl's and Ruby's `elsif`, PHP's `elseif`: all `else if`
const ELSE_IF_WORDS: [&str; 3] = ["elif", "elsif", "elseif"];
const RETURN_KEYWORD: &str = "return";
/// `x = data a and b`: the prefix takes the whole expression after it (card data-quoting); `data class` is a modifier
const DATA_KEYWORD: &str = "data";
const CLASS_KEYWORD: &str = "class";
/// `await job` waits for a task; its operand binds like the operand of a unary minus (Op::Neg)
const AWAIT_KEYWORD: &str = "await";
/// The words of a block of code, also glued to it: `go{…}` starts a task as `go {…}` does (lowering/go_blocks.rs,
/// card error-beep), `loop{…}` loops, `do{…}` runs its block (card glued-loop); no tags `go:{…}`
const GLUED_BLOCK_KEYWORDS: [&str; 3] = ["go", "loop", "do"];
/// `await all jobs`: every task of a list (P47)
const AWAIT_ALL_WORD: &str = "all";
/// `await x` binds its operand like unary minus
const AWAIT_OPERAND_BP: u8 = Op::Neg.binding_power().1;
const PRINT_WORD: &str = "print";
/// `print a  print b`: statements separated by spaces only (user decision 2026-10-03: a loud error)
const TWO_STATEMENTS_ON_ONE_LINE: &str = "two statements on one line? separate them with `;` or a newline";
const IN_KEYWORD: &str = "in";
/// What ends a loop's iterable besides its body: the statement, a comprehension's bracket or condition (`if`)
const ITERABLE_ENDS: [char; 6] = [';', '}', ']', ')', ',', '\0'];
const IF_WORD: &str = "if";
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
	node.is_symbol(PRINT_WORD)
}

/// `print` or the call `print(…)`: a print statement starts here
pub(crate) fn starts_print(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) | Node::List(items, Bracket::None, Separator::Space) => {
			items.first().is_some_and(is_print_word)
		}
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
	Node::List(
		[Symbol(PRINT_WORD.to_string())].into_iter().chain(arguments).collect(),
		Bracket::Round,
		Separator::None,
	)
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
	let items = if separator == Separator::Space {
		with_arrow_bodies(items)
	} else {
		items
	};
	let items = match separator {
		Separator::Space if bracket != Bracket::Square => {
			guarded_command(&items).map_or(items, |guarded| vec![guarded])
		}
		_ => items,
	};
	// `{ print upper n }`: a block of the one statement prints the one expression too
	let is_print_block =
		bracket == Bracket::Curly && separator == Separator::Space && items.len() > 2 && is_print_word(&items[0]);
	if is_print_block {
		return Node::List(vec![items[0].clone(), one_expression(&items[1..])], bracket, separator);
	}
	// `{ print i, i*2 }`: the block of the one call print(i, i*2)
	if let (Bracket::Curly, Separator::Colon, Some(Node::List(head, Bracket::None, Separator::Space))) =
		(&bracket, &separator, items.first().map(Node::drop_meta))
	{
		if is_print_word(&head[0]) {
			return Node::List(
				vec![print_call(head[1..].iter().chain(&items[1..]).cloned())],
				bracket,
				Separator::None,
			);
		}
	}
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

/// `print it if it%2`, `do {i++} while c`: a trailing `if` guards and a trailing `while` loops the whole command, not
/// its last word, as in Ruby and Perl (card glued-loop)
fn guarded_command(items: &[Node]) -> Option<Node> {
	let (last, command) = items.split_last()?;
	if !matches!(command.first()?.drop_meta(), Symbol(_)) {
		return None;
	}
	let whole = |tail: &Node| {
		Node::List(
			[command, std::slice::from_ref(tail)].concat(),
			Bracket::None,
			Separator::Space,
		)
	};
	match last.drop_meta() {
		Node::Key(head, Op::Do, body) => match head.drop_meta() {
			Node::Key(nothing, Op::While, condition) if **nothing == Node::Empty => {
				Some(while_do((**condition).clone(), whole(body)))
			}
			_ => None,
		},
		Node::Key(condition, Op::Then, guarded) => match condition.drop_meta() {
			Node::Key(nothing, Op::If, _) if **nothing == Node::Empty => {
				Some(Node::Key(condition.clone(), Op::Then, Box::new(whole(guarded))))
			}
			_ => None,
		},
		_ => None,
	}
}

/// `(x => print x)`: `=>` binds looser than the space, so the words after a lambda's arrow are its body (`print x`), not
/// further items; a run of entries `1 => "a" 2 => "b"` stays a list of pairs
fn with_arrow_bodies(mut items: Vec<Node>) -> Vec<Node> {
	let is_arrow = |node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::FatArrow, _));
	let Some(arrow) = items.iter().position(is_arrow) else {
		return items;
	};
	if arrow + 1 == items.len() || items[arrow + 1..].iter().any(is_arrow) {
		return items;
	}
	let rest = items.split_off(arrow + 1);
	let Node::Key(parameters, op, body) = items.pop().expect("the arrow").drop_meta().clone() else {
		unreachable!("an arrow")
	};
	let body = one_expression(&[vec![*body], rest].concat());
	items.push(Node::Key(parameters, op, Box::new(body)));
	items
}

/// The infix operators continue_expr reads apart from the operator table
enum SpecialInfix {
	/// `7//2`, `7 div 2`, `x//=2`
	FloorDivision { compound: bool },
	/// `xs |> f(a)`
	Pipeline,
	/// `xs .+ 4`
	ElementWise(Op),
	/// `x in xs` and `xs contains x`: the list [lhs word operand]
	Membership(&'static str),
	/// Julia's dot call `f.(xs)`, `add.(xs, 10)`: f mapped over the first argument
	DotCall,
}

impl SpecialInfix {
	fn applied(self, lhs: Node, operand: Node) -> Node {
		match self {
			SpecialInfix::FloorDivision { compound } => {
				let quotient = floor_division(lhs.clone(), operand);
				if compound {
					Node::Key(Box::new(lhs), Op::Assign, Box::new(quotient))
				} else {
					quotient
				}
			}
			SpecialInfix::Pipeline => piped(lhs, operand),
			// Java's and Scala's `import math.*`: a glob, nothing after the `*`, is the module whole
			SpecialInfix::ElementWise(Op::Mul) if matches!(operand, Empty) => {
				crate::normalize::hint(
					&format!("{}.*", lhs.serialize()),
					&lhs.serialize(),
					"warp imports a module whole",
				);
				lhs
			}
			// kept as written, lowering/broadcasting.rs lower_element_wise_operators gives it its meaning
			SpecialInfix::ElementWise(op) => Node::List(
				vec![lhs, Symbol(crate::operators::element_wise_word(op)), operand],
				Bracket::None,
				Separator::Space,
			),
			SpecialInfix::Membership(word) => Node::List(
				vec![lhs, Symbol(word.to_string()), operand],
				Bracket::None,
				Separator::Space,
			),
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
	let applied = Node::List(
		[vec![function, item.clone()], arguments].concat(),
		Bracket::Round,
		Separator::None,
	);
	let each = Node::Key(Box::new(item), Op::FatArrow, Box::new(applied));
	call(MAP_WORD, vec![list, each])
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
/// Words between two operands besides the operator words (operators::is_operator_word): membership, `div`, `of`
const JOINING_WORDS: [&str; 7] = [IN_KEYWORD, "contains", "has", "includes", "div", OF_WORD, "matches"];

/// A word joining two operands, never a call of the word before it: `a and b`, `n mod 100`, `x in xs`, `p is int`
pub(crate) fn joins_operands(word: &str) -> bool {
	crate::operators::is_operator_word(word) || JOINING_WORDS.contains(&word)
}
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
/// `keys(m)`, `m.keys`, `m.keys()`: an iterable whose items are a map's keys
fn iterates_keys(iterable: &Node) -> bool {
	let is_keys_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if word == "keys" || word == crate::words::MAP_KEYS);
	let is_keys_call = |call: &Node| {
		is_keys_word(call)
			|| matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_keys_word))
	};
	match iterable.drop_meta() {
		Node::Key(_, Op::Dot, method) => is_keys_call(method),
		call => is_keys_call(call),
	}
}

/// How tight a suffix `!` binds: below the index `#`, above `^` and arithmetic
const BANG_BP: u8 = Op::Hash.binding_power().0.midpoint(Op::Pow.binding_power().0);
const TO_WORD: &str = "to";
const TO_SENTENCE_WORD: &str = "To";
/// `to greet p do …`: the word between a `to` definition's parameters and its body
const DO_WORD: &str = "do";
/// `to render text as pdf: …`: the word before a `to` definition's result type (card render-as)
const RESULT_TYPE_WORD: &str = "as";
const OF_WORD: &str = "of";
/// `a[start:end]` calls the library word `slice`
const SLICE_WORD: &str = crate::words::SLICE;
pub(crate) const TIMES_WORD: &str = "times";
/// The call `n times text` is parsed to: the text repeated n times
pub const TEXT_TIMES: &str = "times·text";
/// The acknowledge-once note that a spaced `//` after code is a comment, not Python's floor division
const SLASH_COMMENT_TOPIC: &str = "slash-comment";
const ELVIS_WORD: &str = "elvis";
/// `x is int` tests the type; `x == int` stays equality (user decision #30)
const IS_WORD: &str = "is";
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
		ParserOptions {
			wit_mode: true,
			..Default::default()
		}
	}

	pub fn xml() -> Self {
		ParserOptions {
			xml_mode: true,
			..Default::default()
		}
	}

	pub fn data() -> Self {
		ParserOptions {
			data_mode: true,
			..Default::default()
		}
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

/// The sign of a superscript exponent: ⁺ is +1, ⁻ is -1
fn superscript_sign(ch: char) -> Option<i64> {
	match ch {
		'⁺' => Some(1),
		'⁻' => Some(-1),
		_ => None,
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

fn superscript_letter(ch: char) -> Option<char> {
	SUPERSCRIPT_LETTERS
		.iter()
		.find(|(raised, _)| *raised == ch)
		.map(|(_, letter)| *letter)
}

fn superscript_digit(ch: char) -> Option<i64> {
	SUPERSCRIPT_DIGITS
		.chars()
		.position(|digit| digit == ch)
		.map(|position| position as i64)
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
	c.is_alphanumeric() || c == '_' || is_music_accidental(c)
}

/// The sharp and flat signs continue a name: `C♯4`, `D♭5` are note names (lowering/music_words.rs)
fn is_music_accidental(c: char) -> bool {
	matches!(c, '♯' | '♭')
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
		Node::List(items, Bracket::Round, Separator::None)
			if matches!(items.first().map(Node::drop_meta), Some(Symbol(_))) =>
		{
			let mut items = items.clone();
			items.insert(1, value);
			Node::List(items, Bracket::Round, Separator::None)
		}
		_ => Node::List(vec![stage, value], Bracket::Round, Separator::None),
	}
}

fn floor_division(dividend: Node, divisor: Node) -> Node {
	call(FLOOR_QUOTIENT, vec![dividend, divisor])
}

/// Read and parse a WARP file
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
		Node::List(items, bracket, separator) if items.len() == 3 && items[1].is_symbol(IN_KEYWORD) => {
			split_trailing_block(&items[2], empty_is_block).map(|(collection, block)| {
				(
					Node::List(
						vec![items[0].clone(), items[1].clone(), collection],
						bracket.clone(),
						separator.clone(),
					),
					block,
				)
			})
		}
		Node::Key(left, op, right) if op.is_comparison() || op.is_arithmetic() || op.is_logical() || op.is_prefix() => {
			split_trailing_block(right, empty_is_block)
				.map(|(operand, block)| (Node::Key(left.clone(), *op, Box::new(operand)), block))
		}
		_ => None,
	}
}

pub fn parse_file(path: &str) -> Node {
	let options = if path.ends_with(".wit") {
		ParserOptions::wit()
	} else {
		ParserOptions::default()
	};
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

/// The number a text spells if the whole text is one number literal, so `int "12a"` never guesses; "1/3" spells the
/// exact ratio of two Ints, as the code 1/3 does (card fraction-number)
pub fn number_in_text(text: &str) -> Option<Number> {
	let text = text.trim();
	if let Some((numerator, denominator)) = text.split_once('/') {
		return match (number_in_text(numerator)?, number_in_text(denominator)?) {
			(numerator @ Number::Int(_), denominator @ Number::Int(_)) => Some(Number::ratio(numerator, denominator)),
			_ => None,
		};
	}
	let spells_number = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || "+-._eE".contains(c));
	if !spells_number {
		return None;
	}
	// read as data: code marks `2.5e-2` `as float` (scientific_float), a text spells just the number
	match parse_data(text).drop_meta() {
		Node::Number(number) => Some(*number),
		_ => None,
	}
}

/// `(col // 3) * 3`: a spaced `//` after code starts a comment, and when the rest of its line closes a bracket that is
/// open before it, the writer meant floor division. The line, column and the hidden closing bracket.
fn comment_hiding_a_closer(input: &str) -> Option<(usize, usize, char)> {
	for (line_index, line) in input.lines().enumerate() {
		let chars: Vec<char> = line.chars().collect();
		let (mut depth, mut quote, mut escaped): (i32, Option<char>, bool) = (0, None, false);
		for (column, &c) in chars.iter().enumerate() {
			match (quote, c) {
				// `\"` inside a text leaves it open
				(Some(_), _) if escaped => escaped = false,
				(Some(_), '\\') => escaped = true,
				(Some(open), _) if c == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(c),
				(None, '(' | '[' | '{') => depth += 1,
				(None, ')' | ']' | '}') => depth -= 1,
				(None, '/') if chars.get(column + 1) == Some(&'/') => {
					let spaced = column > 0 && chars[column - 1] == ' ' && chars.get(column + 2) == Some(&' ');
					let rest = &chars[column + 2..];
					let closes = rest
						.iter()
						.map(|c| match c {
							')' | ']' | '}' => 1,
							'(' | '[' | '{' => -1,
							_ => 0,
						})
						.sum::<i32>();
					if spaced && depth > 0 && closes > 0 {
						let closer = rest
							.iter()
							.find(|c| matches!(c, ')' | ']' | '}'))
							.copied()
							.unwrap_or(')');
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

/// `(7 // 2)`: the fix `7//2`, the operands around the `//` at `line`:`column` glued to it
fn glued_floor_division(diagnostic: Diagnostic, input: &str, line: usize, column: usize) -> Diagnostic {
	let text: Vec<char> = input.lines().nth(line - 1).unwrap_or_default().chars().collect();
	let before = text[..column - 1].iter().rposition(|c| !c.is_whitespace());
	let after = text[column + 1..]
		.iter()
		.position(|c| !c.is_whitespace())
		.map(|offset| column + 1 + offset);
	let (Some(before), Some(after)) = (before, after) else {
		return diagnostic;
	};
	let written: String = text[before..=after].iter().collect();
	diagnostic.offer("floor division", written, format!("{}//{}", text[before], text[after]))
}

/// The error `message`, fixed by adding the missing `closer` at `at` (line, column)
/// `for variable in iterable body`, the loop for_loop.rs lowers
pub(crate) fn for_in_loop(variable: Node, iterable: Node, body: Node) -> Node {
	Node::List(
		vec![
			Symbol("for".to_string()),
			variable,
			Symbol("in".to_string()),
			iterable,
			body,
		],
		Bracket::None,
		Separator::Space,
	)
}

pub(super) fn missing_closer(message: String, closer: impl Into<String>, at: (usize, usize)) -> Node {
	let closer = closer.into();
	Diagnostic::default()
		.message(message)
		.offering(crate::fixits::inserted(format!("the closing {closer}"), closer, at))
		.into_error()
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
	/// Types the program declares (`class point{…}`), found by a pre-scan of the source: `point{x:1}` constructs one (D4)
	declared_types: std::collections::HashSet<String>,
	/// The words of the declared types of several words (`class full name {…}`), longest first: read as one name `full-name`
	multi_word_types: Vec<Vec<String>>,
	/// The parameters of each `to` phrase defined so far, to catch a second phrase that differs only by untyped nouns
	phrase_definitions: std::collections::HashMap<String, Vec<Node>>,
}

/// The single argument `(int x)` of `f(int x)` is the typed parameter `x:int`, so that flattening the call `f(T, y)` into
/// `(f T y)` never reads a parameter named like a type (`offset_of(text, byte, start)`) as the type of the next one
fn typed_parameter(arguments: Node) -> Node {
	let is_type_word =
		|node: &Node| matches!(node.drop_meta(), Symbol(word) if crate::analyzer::type_word_kind(word).is_some());
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) => match items.as_slice() {
			[type_word, name] if is_type_word(type_word) && matches!(name.drop_meta(), Symbol(_)) => {
				Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()))
			}
			// `x of int`: the parameter x of type int
			[name, of, type_word]
				if of.drop_meta().is_symbol(OF_WORD)
					&& is_type_word(type_word)
					&& matches!(name.drop_meta(), Symbol(_))
					&& !is_type_word(name) =>
			{
				Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()))
			}
			// Kotlin `vararg xs: Int`, C# `params int[] xs`: the rest parameter `*xs` (lowering/variadic.rs)
			[marker, .., name] if matches!(marker.drop_meta(), Symbol(word) if REST_MARKERS.contains(&word.as_str())) => {
				match name.drop_meta() {
					Symbol(name) => Symbol(format!("{}{name}", crate::words::STARRED)),
					Node::Key(name, Op::Colon, _) => Symbol(format!("{}{}", crate::words::STARRED, name.name())),
					_ => arguments,
				}
			}
			// `int b = 2` (C#, C++): `b:int = 2`
			[type_word, default] if is_type_word(type_word) => match default.drop_meta() {
				Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Symbol(_)) => Node::Key(
					Box::new(Node::Key(name.clone(), Op::Colon, Box::new(type_word.clone()))),
					Op::Assign,
					value.clone(),
				),
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
		Node::List(items, bracket, Separator::Colon) => Node::List(
			items.iter().cloned().map(typed_parameter).collect(),
			bracket.clone(),
			Separator::Colon,
		),
		_ => typed_parameter(arguments),
	}
}

fn operator_call(glyph: &[char], arguments: Vec<Node>) -> Node {
	let name = format!("{OPERATOR_FUNCTION_PREFIX}{}", glyph.iter().collect::<String>());
	Node::List(
		[vec![Symbol(name)], arguments].concat(),
		Bracket::Round,
		Separator::None,
	)
}

enum ElseParseMode {
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
		let input = if options.data_mode {
			input
		} else {
			crate::uniscript_entities::expand_entities(&input)
		};
		let lines: Vec<String> = input.lines().map(String::from).collect();
		let current_line = lines.first().cloned().unwrap_or_default();
		let chars: Vec<char> = input.chars().collect();
		let user_operators = if options == ParserOptions::default() {
			scan_user_operators(&input)
		} else {
			Vec::new()
		};
		let (declared_types, multi_word_types) = if options == ParserOptions::default() {
			(scan_declared_types(&input), scan_multi_word_types(&input))
		} else {
			Default::default()
		};
		WarpParser {
			user_operators,
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
			let message = format!("`//` at {line}:{column} starts a comment that hides the closing `{closer}`: floor division is written glued, a//b");
			return glued_floor_division(Diagnostic::default().message(message), input, line, column).into_error();
		}
		let mut parser = WarpParser::new_with_options(input.to_string(), options);
		let program = parser.parse_list_with_separators(None, Bracket::None);
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

/// The atoms the parser reads in a source, in order, as code: the token stream of this scannerless parser
pub fn read_atoms(source: &str) -> Vec<Atom> {
	let mut parser = WarpParser::new_with_options(source.to_string(), ParserOptions::default());
	parser.parse_list_with_separators(None, Bracket::None);
	parser.atoms
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

pub(crate) fn truth_glyph(glyph: &str) -> Option<Node> {
	TRUTH_GLYPHS
		.iter()
		.find(|(spelling, _)| *spelling == glyph)
		.map(|(_, truth)| if *truth { Node::True } else { Node::False })
}

fn check_constants(s: &str, data_mode: bool) -> Option<Node> {
	let is_word = s.chars().all(|c| c.is_ascii_alphabetic());
	if data_mode && is_word && !DATA_WORD_LITERALS.contains(&s) {
		return None;
	}
	match s.to_lowercase().as_str() {
		"true" | "yes" => Some(Node::True),
		"false" | "no" | "" => Some(Node::False),
		glyph if TRUTH_GLYPHS.iter().any(|(spelling, _)| *spelling == glyph) => truth_glyph(glyph),
		"ø" | "null" | "nul" | "none" | "nil" | "nill" | "nix" | "nada" | "nothing" | "empty" | "void" => Some(Empty),
		// exact generators (extensions/reals.rs); a bare `e` or `i` stays a free name
		"π" | "pi" => Some(real(Exact::pi())),
		"τ" | "tau" => Some(real(Exact::pi().scale(&Rational::integer(2)))),
		"euler" | "ℯ" => Some(real(Exact::euler())),
		"ⅈ" => Some(real(Exact::imaginary())),
		// hyperreals (wiki/hyperreals.md): the glyphs only, `epsilon` stays a free name like `e`
		"ε" => Some(real(Exact::epsilon())),
		"ω" => Some(real(Exact::omega())),
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
	if crate::interpolation::template_mentions(node, |hole| mentions(hole, name)) {
		return true;
	}
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
	let (chained, grouped) = (
		format!("{left} and {middle} {symbol} {right}"),
		format!("({left}) {symbol} {right}"),
	);
	Diagnostic::default()
		.fix(format!("{chained} or {grouped}"))
		.message(format!("ambiguous: equality does not chain in {left} {symbol} {right}"))
		.offer(
			"both equalities, as Python chains them",
			format!("{left} {symbol} {right}"),
			chained,
		)
		.offer(
			"compare the first result, as C does",
			format!("{left} {symbol} {right}"),
			grouped,
		)
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
		(None, Some((a, op, b))) => vec![
			(COMPARISON_FIRST.to_string(), format!("{left} {symbol} ({a} {op} {b})")),
			(
				format!("`{symbol}` first, as Python"),
				format!("({left} {symbol} {a}) {op} {b}"),
			),
		],
		(Some((a, op, b)), None) => vec![
			(COMPARISON_FIRST.to_string(), format!("({a} {op} {b}) {symbol} {right}")),
			(
				format!("`{symbol}` first, as Python"),
				format!("{a} {op} ({b} {symbol} {right})"),
			),
		],
		(Some(_), Some(_)) => vec![(
			"both comparisons first".to_string(),
			format!("({left}) {symbol} ({right})"),
		)],
		(None, None) => return None,
	};
	let word = if symbol == '&' { "and" } else { "or" };
	let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
	let written = format!("{left} {symbol} {right}");
	let diagnostic = Diagnostic::default().fix(forms.join(" or ")).message(format!(
		"ambiguous: `{symbol}` mixed with a comparison in {written}; group it or write `{word}`"
	));
	Some(readings.iter().fold(diagnostic, |diagnostic, (meaning, form)| {
		diagnostic.offer(meaning, &written, form)
	}))
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
		Node::Key(head, op @ (Op::Then | Op::Else | Op::Do), body) => {
			Node::Key(Box::new(negate_condition(*head)), op, body)
		}
		Node::Key(empty, op @ (Op::If | Op::While), condition) if matches!(empty.drop_meta(), Empty) => {
			Node::Key(empty, op, Box::new(Node::Key(Box::new(Empty), Op::Not, condition)))
		}
		other => other,
	}
}

/// `f={it*2}` defines a function like `f:={it*2}`: a name assigned a block that uses `it`; other blocks are data
fn is_function_block(name: &Node, block: &Node) -> bool {
	matches!(name.drop_meta(), Node::Symbol(_))
		&& matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))
		&& mentions(block, "it")
}

/// A braceless call `sleep 1s`, `f 3`: a word applied to what follows it
fn is_command(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && matches!(items[0].drop_meta(), Symbol(_)))
}
