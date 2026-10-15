//! Asks about written forms the analyzer sees whole (notes/welcoming.md): unbracketed list assignments (D12)
//! and suffix words mixed with infix arithmetic (D9); a trailing percent `10%` is read as `10/100` here too

use super::nodes::{call, children_rewritten, is_binding, key};
use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
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
	operator_values(library_word_values(lower_node(node, &words), &words))
}

/// The parameter of the function an operator word stands for
const OPERAND_NAME: &str = "value";

fn operand() -> Box<Node> {
	Box::new(symbol(OPERAND_NAME))
}

/// `f = abs`, `xs.map(sqrt)`: an operator word left without an operand is the function `value => abs value`
fn operator_values(node: Node) -> Node {
	match operator_reference(&node) {
		Some(op) => Node::Key(operand(), Op::Arrow, Box::new(Node::Key(Box::new(Node::Empty), op, operand()))),
		None => node.map_children(operator_values),
	}
}

/// `xs.map(upper)`, `map(xs, floor)`, `f := floor`: a library word passed to an iteration word or defined as a name is
/// the function `value => upper value`. Assigned (`f = function floor`, which parses as `f = floor`) too, until
/// builtin-functions marks the reference and gives the bare `f = floor` P82's error
fn library_word_values(node: Node, words: &SuffixWords) -> Node {
	let node = node.map_children(|child| library_word_values(child, words));
	match node {
		Node::List(mut items, bracket, separator) => {
			let is_iteration = items.first().is_some_and(|head| matches!(head.drop_meta(), Node::Symbol(name) if crate::function_values::ITERATION_WORDS.contains(&name.as_str())));
			if let Some(function) = items.last_mut().filter(|last| is_iteration && words.is_function_word(last)) {
				*function = word_function(function);
			}
			Node::List(items, bracket, separator)
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) if words.is_function_word(&value) => {
			Node::Key(target, op, Box::new(word_function(&value)))
		}
		other => other,
	}
}

/// `value => word value`
fn word_function(word: &Node) -> Node {
	let call = Node::List(vec![word.clone(), *operand()], Bracket::None, Separator::Space);
	Node::Key(operand(), Op::Arrow, Box::new(call))
}

/// The suffix words of the program's functions with the function each applies, `squared` → `square`; a name the
/// program assigns is its variable, never a suffix word (`solved = solve(x); return solved`)
fn suffix_words(node: &Node) -> SuffixWords {
	let mut variables = HashSet::new();
	crate::library_words::collect_assigned_names(node, &mut variables);
	let functions = crate::analyzer::applicable_function_names(node);
	// the built-in real functions apply too (`x sined`), yet stay library words a program may pass on (card sin-cos)
	let real_functions = crate::numbers::real::FUNCTIONS.iter().map(|name| name.to_string());
	let applicable: Vec<String> = functions.iter().cloned().chain(real_functions).collect();
	let words =
		applicable.iter().flat_map(|function| past_forms(function).into_iter().map(|word| (word, function.clone())));
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
	/// the names the program assigns
	variables: HashSet<String>,
	/// the program's own functions
	functions: HashSet<String>,
}

impl SuffixWords {
	/// A library word the program does not name itself: `first sorted xs` is `first(sort(xs))`, `xs.map(upper)` maps it
	fn is_library_word(&self, node: &Node) -> bool {
		self.unnamed(node).is_some_and(crate::library_words::is_library_word)
	}

	/// A library word of one value the program does not name itself, also libm's (`floor`, `sin`): a function value
	fn is_function_word(&self, node: &Node) -> bool {
		let is_libm_word = |name: &str| {
			crate::lowering::library_signatures::typed_first_arity(name) == Some(1)
				|| crate::numbers::real::FUNCTIONS.contains(&name)
		};
		self.is_library_word(node) || self.unnamed(node).is_some_and(is_libm_word)
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
		Node::List(items, bracket, separator) => {
			if let Some(asked) = bare_list_assignment(&items, &bracket, &separator) {
				return asked.map_or_else(|error| error, |chosen| lower_node(chosen, functions));
			}
			if let Some(list) = spaced_values(&items, &bracket, &separator) {
				return lower_node(list, functions);
			}
			let items: Vec<Node> = items.into_iter().map(|item| lower_node(item, functions)).collect();
			apply_suffix_words(items, functions, bracket, separator)
		}
		other => children_rewritten(other, |child| lower_node(child, functions)),
	}
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
		Node::Key(left, op, right) if ARITHMETIC.contains(op) || matches!(op, Op::Square | Op::Cube) => {
			(left.is_empty_node() || is_number_or_text(left) || is_arithmetic_of_values(left))
				&& (right.is_empty_node() || is_number_or_text(right) || is_arithmetic_of_values(right))
		}
		other => is_number_or_text(other),
	}
}

/// `2⁻¹ 2⁰ 2¹`, `1+1 2+2` on a line of their own: spaced values are a list (reconsider-spaced, user 2026-10-15)
fn spaced_values(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let is_spaced = *bracket == Bracket::None && *separator == Separator::Space && items.len() > 1;
	let is_list =
		is_spaced && items.iter().all(is_arithmetic_of_values) && !items[1..].iter().any(crate::analyzer::is_negative);
	is_list.then(|| Node::List(items.to_vec(), Bracket::Square, Separator::Space))
}

/// `a=1,2,3` without brackets: a list or separate statements? Unanswered an error (D12). Spaced, `a=1 2 3` is the list
/// (reconsider-spaced, user 2026-10-15) unless a later value has a glued sign: `x = 5 -2` still asks
fn bare_list_assignment(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Result<Node, Node>> {
	if *bracket != Bracket::None || !matches!(separator, Separator::Space | Separator::Colon) {
		return None;
	}
	let (first, rest) = items.split_first()?;
	let Node::Key(name, Op::Assign, value) = first.drop_meta() else {
		return None;
	};
	let spaced = *separator == Separator::Space;
	let is_value = |node: &Node| is_number_or_text(node) || (spaced && is_arithmetic_of_values(node));
	if rest.is_empty() || !is_value(value) || !rest.iter().all(is_value) {
		return None;
	}
	let values = std::iter::once(value.as_ref().clone()).chain(rest.iter().cloned()).collect();
	let list = Node::Key(name.clone(), Op::Assign, Box::new(Node::List(values, Bracket::Square, separator.clone())));
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
	let Node::Symbol(word) = word.drop_meta() else {
		return None;
	};
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
		None => call(function, vec![argument]),
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

fn grouped(node: Node) -> Node {
	parens(vec![node])
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
			(applied.last().filter(|operand| !functions.is_library_word(operand)), suffix)
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
	let loose = Node::List(vec![grouped(operand.clone()), word.clone()], Bracket::None, Separator::Space);
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
