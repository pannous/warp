//! Asks about written forms the analyzer sees whole (notes/welcoming.md): unbracketed list assignments (D12)
//! and suffix words mixed with infix arithmetic (D9); a trailing percent `10%` is read as `10/100` here too

use super::nodes::{call, children_rewritten, is_binding, key};
use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
use crate::lowering::typed_library_words::library_word_of;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const BARE_LIST_TOPIC: &str = "bare-list";
const SUFFIX_PRECEDENCE_TOPIC: &str = "suffix-precedence";
/// `squared` is the suffix form of `square`, `sorted` of `sort` (wiki/function.md: every function generates a suffix operator)
const SUFFIX_WORD_ENDINGS: [&str; 2] = ["d", "ed"];
const PERCENT: i64 = 100;
const ARITHMETIC: [Op; 7] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Rem, Op::Pow];

pub fn lower(node: Node) -> Node {
	let words = suffix_words(&node);
	hint_broadcast_maps(&node, &words);
	operator_values(library_word_values(lower_node(node, &words), &words))
}

const BROADCAST_REASON: &str = "a function of one value maps over a list by itself";

/// `xs.map(square)`, `map square on: xs`: a named function of one value maps over a list by itself (broadcasting.rs),
/// `square xs` (wiki/Broadcasting.md, card hint-map); not a word taking the list whole (`reverse`, `count`)
fn hint_broadcast_maps(program: &Node, words: &SuffixWords) {
	let broadcasting = crate::broadcasting::broadcasting_functions(program);
	// a variable of a library word's name shadows it; a program's own function broadcasting.rs decided already
	let broadcasts =
		|name: &str| broadcasting.contains(name) && (words.functions.contains(name) || !words.variables.contains(name));
	program.visit(&mut |part| {
		if let Some((function, list)) = mapped_function(part, words, &broadcasts) {
			crate::normalize::set_position_of(part);
			let mapped = format!("{function} {}", crate::normalize::operand_text(&list));
			let written = written_map(part).unwrap_or_else(|| crate::normalize::operand_text(part));
			crate::normalize::hint(&written, &mapped, BROADCAST_REASON);
		}
	});
}

/// The source of a `map` as written, to the parenthesis closing `map(` (`map(xs, f)`, `xs.map(f)`), else to the end of
/// the statement (`map abs xs`)
fn written_map(part: &Node) -> Option<String> {
	let map = crate::words::MAP_WORD;
	let text = crate::errors::diagnostic::written_statement(part)?;
	// `map(xs, f)`: the call's position is its first argument, the statement ends at its closing parenthesis
	if matches!(part.drop_meta(), Node::List(_, Bracket::Round, _)) && !text.starts_with(map) {
		return Some(format!("{map}({text})"));
	}
	match text.find(&format!("{map}(")) {
		Some(call) => closing_parenthesis(&text, call + map.len() + 1).map(|end| text[..end].to_string()),
		None => text.contains(map).then_some(text),
	}
}

/// The end of the parenthesis closing a group opened before `from`
fn closing_parenthesis(text: &str, from: usize) -> Option<usize> {
	let mut depth = 0;
	for (at, letter) in text.char_indices().skip_while(|(at, _)| *at < from) {
		match letter {
			'(' | '[' | '{' => depth += 1,
			')' if depth == 0 => return Some(at + 1),
			')' | ']' | '}' => depth -= 1,
			_ => {}
		}
	}
	None
}

/// The named function and the list of a `map`: `xs.map(f)`, `map(xs, f)`, `map f xs`, `map f on: xs`
fn mapped_function(part: &Node, words: &SuffixWords, broadcasts: &dyn Fn(&str) -> bool) -> Option<(String, Node)> {
	let is_map = |head: &Node| {
		head.drop_meta().symbol_name() == Some(crate::words::MAP_WORD)
			&& !words.functions.contains(crate::words::MAP_WORD)
	};
	let named = |node: &Node| function_name(node).filter(|name| broadcasts(name));
	match part.drop_meta() {
		Node::Key(list, Op::Dot, method) => match method.drop_meta() {
			Node::List(items, _, _) => match items.as_slice() {
				[head, function] if is_map(head) => Some((named(function)?, list.as_ref().clone())),
				_ => None,
			},
			_ => None,
		},
		Node::List(items, _, _) if items.first().is_some_and(is_map) => {
			let arguments = crate::closures::with_operator_word_unfolded(items[1..].to_vec(), 0);
			let arguments: Vec<Node> = arguments
				.iter()
				.flat_map(phrase_words)
				.filter(|argument| !argument.is_symbol(crate::words::ON_WORD))
				.flat_map(|argument| unlabeled(&argument))
				.collect();
			match arguments.as_slice() {
				[first, second] => named(first)
					.map(|function| (function, second.clone()))
					.or_else(|| named(second).map(|function| (function, first.clone()))),
				_ => None,
			}
		}
		_ => None,
	}
}

/// The name a function is written with: `square`, `function floor`, `abs` without its operand
fn function_name(node: &Node) -> Option<String> {
	crate::closures::referenced_function(node)
		.or_else(|| operator_reference(node).map(|_| written_word(node).name()))
		.or_else(|| node.drop_meta().symbol_name().map(String::from))
}

/// `square on xs` grouped as braceless phrases after `map`, `(square (on xs))`: their words
fn phrase_words(argument: &Node) -> Vec<Node> {
	match argument.drop_meta() {
		Node::List(words, Bracket::None, _) => words.iter().flat_map(phrase_words).collect(),
		_ => vec![argument.clone()],
	}
}

/// The labeled argument `on: xs` as `xs` after the words before its label (lambdas::labeled_list)
fn unlabeled(argument: &Node) -> Vec<Node> {
	match crate::lambdas::labeled_list(argument) {
		Some((list, words)) => [words, vec![list]].concat(),
		None => vec![argument.clone()],
	}
}

/// The parameter of the function an operator word stands for
const OPERAND_NAME: &str = "value";

fn operand() -> Box<Node> {
	Box::new(symbol(OPERAND_NAME))
}

/// `f = abs`, `xs.map(sqrt)`: an operator word left without an operand is the function `value => abs value`
fn operator_values(node: Node) -> Node {
	if let Node::Key(_, Op::Assign, value) = node.drop_meta() {
		let word_of = |op| crate::warp_parser::PREFIX_OPERATOR_WORDS.iter().find(|(_, of)| *of == op);
		if let Some((word, _)) = operator_reference(value).and_then(word_of) {
			return needs_arguments(value, word);
		}
	}
	match operator_reference(&node) {
		Some(op) => Node::Key(operand(), Op::Arrow, Box::new(Node::Key(Box::new(Node::Empty), op, operand()))),
		None => node.map_children(operator_values),
	}
}

/// P82, as for a program's own function: `f = floor` is no function value, `function floor` is
fn needs_arguments(at: &Node, word: &str) -> Node {
	let count = crate::lowering::library_signatures::library_signature(word).map_or(1, |(params, _)| params.len());
	let plural = if count == 1 { "" } else { "s" };
	crate::errors::diagnostic::Diagnostic::at(at, format!("{word} needs {count} argument{plural}"))
		.fix(format!("function {word}"))
		.into_error()
}

/// `xs.map(upper)`, `map(xs, floor)`, `f := floor`: a library word passed to an iteration word or defined as a name is
/// the function `value => upper value`
fn library_word_values(node: Node, words: &SuffixWords) -> Node {
	let node = node.map_children(|child| library_word_values(child, words));
	match node {
		Node::List(items, bracket, separator) => {
			// a `map` the program defines is its own function, no iteration word (`def map(list, fn)`)
			let is_iteration = items.first().is_some_and(|head| matches!(head.drop_meta(), Node::Symbol(name) if crate::function_values::ITERATION_WORDS.contains(&name.as_str()) && !words.functions.contains(name)));
			// `map sqrt xs`, `map sqrt on: xs`: the parser applied sqrt, the iteration word takes it as its function
			let mut items = if is_iteration { crate::closures::with_operator_word_unfolded(items, 1) } else { items };
			let position = function_position(&items, words);
			if let Some(function) =
				items.get_mut(position).filter(|item| is_iteration && words.is_iterated_function(item))
			{
				*function = word_function(function);
			}
			Node::List(items, bracket, separator)
		}
		// P82: `f = floor` needs floor's arguments; `f = function floor` is the function (closures.rs)
		Node::Key(_, Op::Assign, value)
			if words.is_function_word(&value) && crate::closures::referenced_function(&value).is_none() =>
		{
			needs_arguments(&value, &value.drop_meta().name())
		}
		Node::Key(target, Op::Define, value) if words.is_function_word(&value) => {
			Node::Key(target, Op::Define, Box::new(word_function(&value)))
		}
		other => other,
	}
}

/// `value => word value`
fn word_function(word: &Node) -> Node {
	// `xs.map(function floor)`: the call of floor, no reference to it
	let word = crate::closures::referenced_function(word).unwrap_or_else(|| word.drop_meta().name());
	Node::Key(operand(), Op::Arrow, Box::new(crate::closures::word_applied(&word, vec![*operand()])))
}

/// Where the function stands among an iteration call's items: first in `map floor on xs`, `map(function floor, xs)`,
/// else last as in `map(xs, floor)`
fn function_position(items: &[Node], words: &SuffixWords) -> usize {
	match items {
		[_, function, _, ..] if words.is_iterated_function(function) => 1,
		_ => items.len().saturating_sub(1),
	}
}

/// The suffix words of the program's functions with the function each applies, `squared` → `square`; a name the
/// program assigns is its variable, never a suffix word (`solved = solve(x); return solved`)
fn suffix_words(node: &Node) -> SuffixWords {
	// loop variables and parameters too: `for ord in xs { x = ord }` reads the item (card codepoint-loop-variable)
	let variables = crate::program_names::Bindings::of(node).bound();
	let mut functions = crate::analyzer::applicable_function_names(node);
	// `trim(x) := x-1` is the program's trim·any beside the library's trim: `4 trimmed` chooses between them
	let own_library_words: Vec<String> =
		functions.iter().filter_map(|function| library_word_of(function)).map(String::from).collect();
	// the library's words of one value apply too (`x sined`, `x floored`), unless the program names them
	let library_words = crate::lowering::library_signatures::one_value_words()
		.filter(|word| {
			!variables.contains(*word) && !functions.contains(*word) && !own_library_words.contains(&word.to_string())
		})
		.map(String::from);
	let applicable: Vec<String> = functions.iter().cloned().chain(library_words).collect();
	functions.extend(own_library_words);
	let spelled = |function: &str| library_word_of(function).unwrap_or(function).to_string();
	let words = applicable
		.iter()
		.flat_map(|function| past_forms(&spelled(function)).into_iter().map(|word| (word, function.clone())));
	let mut words: HashMap<String, String> =
		words.filter(|(word, _)| word.ends_with("ed") && !variables.contains(word)).collect();
	// the library's own: `xs sorted`, `xs reversed`, `x squared` (x²), `x cubed` (x³), unless the program names them
	for (word, function) in LIBRARY_SUFFIX_WORDS {
		if !variables.contains(word) && !functions.contains(function.trim_start_matches(POWER_MARK)) {
			words.entry(word.to_string()).or_insert(function.to_string());
		}
	}
	SuffixWords { words, variables, functions }
}

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];
/// Final consonants English never doubles before -ed (`fixed`, `showed`, `played`)
const UNDOUBLED_CONSONANTS: [char; 3] = ['w', 'x', 'y'];

/// The English past forms of a function name (P24): `halve` → `halved`, `fix` → `fixed`, `stop` → `stopped` (a final
/// consonant after a single vowel doubles), `copy` → `copied`
fn past_forms(function: &str) -> Vec<String> {
	let mut forms: Vec<String> = SUFFIX_WORD_ENDINGS.iter().map(|ending| format!("{function}{ending}")).collect();
	let letters: Vec<char> = function.chars().collect();
	let is_vowel = |c: &char| VOWELS.contains(c);
	if let [.., before, vowel, last] = letters.as_slice() {
		if !is_vowel(before)
			&& is_vowel(vowel)
			&& !is_vowel(last)
			&& !UNDOUBLED_CONSONANTS.contains(last)
			&& last.is_ascii_alphabetic()
		{
			forms.push(format!("{function}{last}ed"));
		}
	}
	if let [.., before, 'y'] = letters.as_slice() {
		if !is_vowel(before) {
			forms.push(format!("{}ied", &function[..function.len() - 1]));
		}
	}
	forms
}

/// Suffix words of library functions; a function marked `^` is the power of its number: `squared` is `^2`
const LIBRARY_SUFFIX_WORDS: [(&str, &str); 4] =
	[("sorted", "sort"), ("reversed", "reverse"), ("squared", "^square"), ("cubed", "^cube")];
const POWER_MARK: char = '^';

struct SuffixWords {
	/// suffix word → function
	words: HashMap<String, String>,
	/// the names the program gives: variables, parameters, loop variables
	variables: HashSet<String>,
	/// the program's own functions
	functions: HashSet<String>,
}

impl SuffixWords {
	/// A library word the program does not name itself: `first sorted xs` is `first(sort(xs))`, `xs.map(upper)` maps it
	fn is_library_word(&self, node: &Node) -> bool {
		self.unnamed(node).is_some_and(crate::library_words::is_library_word)
	}

	/// A word calling the values after it, the program's own function or a standard word it does not name: `max sorted
	/// xs` is max(sort(xs)) and `f sorted xs` f(sort(xs)), no suffix `sorted` of max or f; `map sorted xs` maps sort
	fn is_calling_word(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::Symbol(name) if self.functions.contains(name))
			|| self.unnamed(node).is_some_and(crate::library_words::is_calling_word)
			|| (self.unnamed(node).is_some() && crate::lowering::lambdas::takes_a_function_first(node))
	}

	/// A library word of one value the program does not name itself, also libm's (`floor`, `sin`), the host's
	/// (`print`) and a counting word (`len`), but no type word (`type Name = string`): a function value
	fn is_function_word(&self, node: &Node) -> bool {
		let takes_one_value = |name: &str| {
			crate::analyzer::type_word_kind(name).is_none()
				&& (crate::analyzer::is_counting_word(name)
					|| crate::lowering::library_signatures::library_signature(name)
						.is_some_and(|(params, _)| params.len() == 1))
		};
		self.is_library_word(node) || self.unnamed(node).is_some_and(takes_one_value)
	}

	/// What an iteration word takes as its function: a function word, or a conversion (`xs.map(str)`), which elsewhere
	/// is the type it names (`type Age = int`)
	fn is_iterated_function(&self, node: &Node) -> bool {
		self.is_function_word(node)
			|| self.unnamed(node).is_some_and(|name| crate::library_words::CONVERSION_METHODS.contains(&name))
	}

	/// The word `node` is, unless the program names it as a variable or function of its own
	fn unnamed<'a>(&self, node: &'a Node) -> Option<&'a str> {
		match node.drop_meta() {
			Node::Symbol(name) if !self.variables.contains(name) && !self.functions.contains(name) => Some(name),
			_ => None,
		}
	}
}

fn lower_node(node: Node, functions: &SuffixWords) -> Node {
	match node {
		// `10%`: a percent is a hundredth, `10/100`
		Node::Key(left, Op::Mod, right) if right.is_empty_node() => {
			key(lower_node(*left, functions), Op::Div, Node::int(PERCENT))
		}
		Node::List(items, bracket, separator) => lower_list(items, bracket, separator, functions, true),
		other => children_rewritten(other, |child| lower_node(child, functions)),
	}
}

/// `values_may_be_list`: false for the arguments of a spaced call, `print 1+1 2+2` prints `2 4`, not `[2 4]`
fn lower_list(
	items: Vec<Node>,
	bracket: Bracket,
	separator: Separator,
	functions: &SuffixWords,
	values_may_be_list: bool,
) -> Node {
	if let Some(asked) = bare_list_assignment(&items, &bracket, &separator) {
		return asked.map_or_else(|error| error, |chosen| lower_node(chosen, functions));
	}
	if values_may_be_list {
		if let Some(list) = spaced_values(&items, &bracket, &separator) {
			return lower_node(list, functions);
		}
	}
	// `print 1+1 2+2`, `{print 1+1 2+2}`: a word with spaced arguments
	let is_spaced_call = bracket != Bracket::Square
		&& separator == Separator::Space
		&& items.first().is_some_and(|head| head.drop_meta().symbol_name().is_some());
	let lower_item = |item: Node| match item {
		Node::List(arguments, Bracket::None, Separator::Space) if is_spaced_call => {
			lower_list(arguments, Bracket::None, Separator::Space, functions, false)
		}
		other => lower_node(other, functions),
	};
	let items: Vec<Node> = items.into_iter().map(lower_item).collect();
	apply_suffix_words(items, functions, bracket, separator)
}

/// `1`, `-1`, `"a"`, `'a'`
fn is_number_or_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Neg, number) => matches!(number.drop_meta(), Node::Number(_)),
		other => matches!(other, Node::Number(_) | Node::Text(_) | Node::Char(_)),
	}
}

/// `2⁻¹`, `2²`, `3+1`: arithmetic of numbers
fn is_arithmetic_of_values(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(left, op, right) if ARITHMETIC.contains(op) => {
			(left.is_empty_node() || is_number_or_text(left) || is_arithmetic_of_values(left))
				&& (right.is_empty_node() || is_number_or_text(right) || is_arithmetic_of_values(right))
		}
		other => is_number_or_text(other),
	}
}

/// `2⁻¹ 2⁰ 2¹`, `1+1 2+2` on a line of their own, in a block too: spaced values are a list (reconsider-spaced, user
/// 2026-10-15)
fn spaced_values(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	if *separator != Separator::Space || !are_spaced_values(items) {
		return None;
	}
	let list = Node::List(items.to_vec(), Bracket::Square, Separator::Space);
	match bracket {
		Bracket::None => Some(list),
		// `{1+1 2+2}`: the block of the one line; `{1 2}` stays the data block of its values
		Bracket::Curly if !items.iter().all(is_number_or_text) => {
			Some(Node::List(vec![list], Bracket::Curly, Separator::Newline))
		}
		_ => None,
	}
}

/// The arguments gathered for a word of one value: spaced values are one list, `sum 1+1 2+2` is `sum [2 4]`
pub(crate) fn spaced_phrase(items: Vec<Node>) -> Node {
	let bracket = if are_spaced_values(&items) { Bracket::Square } else { Bracket::None };
	Node::List(items, bracket, Separator::Space)
}

/// Several values or number expressions, none after the first with a glued sign: `1+1 2+2`, not `5 -2`
pub(crate) fn are_spaced_values(items: &[Node]) -> bool {
	items.len() > 1 && items.iter().all(is_arithmetic_of_values) && !items[1..].iter().any(crate::analyzer::is_negative)
}

/// `a=1,2,3` without brackets: a list or separate statements? Unanswered an error (D12). Spaced, `a=1 2 3` is the list
/// (reconsider-spaced, user 2026-10-15) unless a later value has a glued sign: `x = 5 -2` still asks. A function body
/// alike: `f() := 1+1 2+2` gives [2 4]
fn bare_list_assignment(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Result<Node, Node>> {
	if *bracket != Bracket::None || !matches!(separator, Separator::Space | Separator::Comma) {
		return None;
	}
	let (first, rest) = items.split_first()?;
	let spaced = *separator == Separator::Space;
	let (name, op, value) = match first.drop_meta() {
		Node::Key(name, Op::Assign, value) => (name, Op::Assign, value),
		Node::Key(name, Op::Define, value) if spaced => (name, Op::Define, value),
		_ => return None,
	};
	let is_value = |node: &Node| is_number_or_text(node) || (spaced && is_arithmetic_of_values(node));
	if rest.is_empty() || !is_value(value) || !rest.iter().all(is_value) {
		return None;
	}
	let values = std::iter::once(value.as_ref().clone()).chain(rest.iter().cloned()).collect();
	let list = Node::Key(name.clone(), op, Box::new(Node::List(values, Bracket::Square, separator.clone())));
	if spaced && !rest.iter().any(crate::analyzer::is_negative) {
		return Some(Ok(list));
	}
	let statements = Node::List(items.to_vec(), Bracket::None, Separator::Semicolon);
	let written = Node::List(items.to_vec(), Bracket::None, separator.clone()).serialize();
	let readings =
		vec![reading("a list", &list.serialize()), reading("separate statements", statements.serialize().trim())];
	let question = Ask::new(
		BARE_LIST_TOPIC,
		format!("is `{}` one list or separate statements?", written.trim()),
		readings,
		Fallback::Error,
	)
	.written(written.trim())
	.at_node(first);
	Some(ask(&question).map(|chosen| if chosen == 0 { list } else { statements }))
}

/// What a suffix word applies: a function (`squared` → `square`) or an operator word after its value (`-7 abs`)
enum Suffix {
	Function(String),
	Operator(Op),
}

/// The function a suffix word applies: `squared` → `square`; `abs` with no operand (the parser's `‖ø`) → ‖
fn suffix_function(word: &Node, functions: &SuffixWords) -> Option<Suffix> {
	if let Some(op) = operator_reference(word) {
		return Some(Suffix::Operator(op));
	}
	let word = word.symbol_name()?;
	functions.words.get(word).cloned().map(Suffix::Function)
}

/// The word as written: `sqrt` for the parser's `√ø`
pub(crate) fn written_word(word: &Node) -> Node {
	let spelling = operator_reference(word)
		.and_then(|op| crate::warp_parser::PREFIX_OPERATOR_WORDS.into_iter().find(|(_, known)| *known == op));
	spelling.map_or_else(|| word.clone(), |(spelling, _)| symbol(spelling))
}

/// `abs`, `sqrt`, `cbrt` written without an operand: the operator itself
fn operator_reference(node: &Node) -> Option<Op> {
	match node.drop_meta() {
		Node::Key(nothing, op @ (Op::Abs | Op::Sqrt | Op::Cbrt), operand)
			if nothing.is_nothing() && operand.is_nothing() =>
		{
			Some(*op)
		}
		_ => None,
	}
}

fn suffix_call(suffix: &Suffix, argument: Node) -> Node {
	let function = match suffix {
		Suffix::Operator(op) => return key(Node::Empty, *op, argument),
		Suffix::Function(function) => function,
	};
	match function.strip_prefix(POWER_MARK) {
		Some(power) => {
			let exponent = if power == "square" { 2 } else { 3 };
			key(argument, Op::Pow, Node::int(exponent))
		}
		None => match library_word_of(function) {
			Some(library_word) => crate::lowering::typed_library_words::dispatched(library_word, vec![argument]),
			None => call(function, vec![argument]),
		},
	}
}

/// The leftmost operand of an infix chain, replaced: `squared+1` → `(2 squared)+1`
fn with_leftmost(node: Node, replace: impl FnOnce(Node) -> Node) -> Node {
	match node {
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_leftmost(node, replace)),
		Node::Key(left, op, right) if ARITHMETIC.contains(&op) => {
			Node::Key(Box::new(with_leftmost(*left, replace)), op, right)
		}
		leaf => replace(leaf),
	}
}

/// The rightmost operand of an infix chain, replaced: `1+2` → `1+(2 squared)`
fn with_rightmost(node: Node, replace: impl FnOnce(Node) -> Node) -> Node {
	match node {
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_rightmost(node, replace)),
		Node::Key(left, op, right) if ARITHMETIC.contains(&op) => {
			Node::Key(left, op, Box::new(with_rightmost(*right, replace)))
		}
		leaf => replace(leaf),
	}
}

/// The suffix word heading `item`: `squared` or `squared+1`
fn heading_suffix_word(item: &Node, functions: &SuffixWords) -> Option<(Node, Suffix)> {
	let mut leftmost = None;
	with_leftmost(item.clone(), |leaf| {
		leftmost = Some(leaf.clone());
		leaf
	});
	let word = leftmost?;
	suffix_function(&word, functions).map(|function| (word, function))
}

fn is_infix_arithmetic(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, op, _) if ARITHMETIC.contains(op))
}

/// `x squared` is `square(x)`; after ungrouped arithmetic (`1+2 squared`) the user is asked what the word applies to (D9)
fn apply_suffix_words(items: Vec<Node>, functions: &SuffixWords, bracket: Bracket, separator: Separator) -> Node {
	if !matches!(bracket, Bracket::None | Bracket::Round) || separator != Separator::Space {
		return Node::List(items, bracket, separator);
	}
	let item_count = items.len();
	let mut applied: Vec<Node> = Vec::new();
	for item in items {
		let suffix = heading_suffix_word(&item, functions);
		// `map xs abs` passes abs to map: an operator word applies only to a whole first operand (`-7 abs`, `x = -7 abs`)
		let suffix = suffix.filter(|(_, suffix)| matches!(suffix, Suffix::Function(_)) || applied.len() == 1);
		let (Some(operand), Some((word, function))) =
			(applied.last().filter(|operand| !functions.is_calling_word(operand)), suffix)
		else {
			applied.push(item);
			continue;
		};
		let operand = operand.clone();
		applied.pop();
		applied.push(suffix_applied(operand, item, &word, &function));
	}
	match applied.len() {
		1 if applied.len() < item_count => applied.pop().expect("one item"),
		_ => Node::List(applied, bracket, separator),
	}
}

/// `operand item`, where `item` heads with the suffix word: `x = 7 squared + 1` squares the assigned value, `x = 49 + 1`
fn suffix_applied(operand: Node, item: Node, word: &Node, function: &Suffix) -> Node {
	match operand {
		Node::Meta { node, data } if is_assignment(&node) => {
			Node::Meta { node: Box::new(suffix_applied(*node, item, word, function)), data }
		}
		Node::Key(target, op, value) if is_binding(&op) => {
			Node::Key(target, op, Box::new(suffix_applied(*value, item, word, function)))
		}
		operand => {
			let argument = match is_infix_arithmetic(&operand) {
				true => match suffix_precedence(&operand, &written_word(word)) {
					Ok(true) => with_rightmost(operand, |leaf| suffix_call(function, leaf)),
					Ok(false) => suffix_call(function, operand),
					Err(error) => error,
				},
				false => suffix_call(function, operand),
			};
			with_leftmost(item, |_| argument)
		}
	}
}

fn is_assignment(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, op, _) if is_binding(op))
}

/// `1+2 squared`: does the word bind to the nearest operand (`1+(2 squared)`, true) or apply to the whole (`(1+2) squared`)?
fn suffix_precedence(operand: &Node, word: &Node) -> Result<bool, Node> {
	let mut nearest = Node::Empty;
	let tight = with_rightmost(operand.clone(), |leaf| {
		nearest = leaf.clone();
		Node::List(vec![leaf, word.clone()], Bracket::Round, Separator::Space)
	});
	let loose = Node::List(vec![parens(vec![operand.clone()]), word.clone()], Bracket::None, Separator::Space);
	let (word_text, whole_text) = (word.serialize(), operand.serialize());
	let written = format!("{whole_text} {word_text}");
	let readings = vec![
		reading(&format!("{word_text} binds to {}", nearest.serialize()), tight.serialize().trim()),
		reading(&format!("{word_text} applies to {whole_text}"), loose.serialize().trim()),
	];
	let question = Ask::new(
		SUFFIX_PRECEDENCE_TOPIC,
		format!("does `{word_text}` in `{written}` apply to {} or to {whole_text}?", nearest.serialize()),
		readings,
		Fallback::Error,
	)
	.written(&written)
	.at_node(word);
	ask(&question).map(|chosen| chosen == 0)
}
