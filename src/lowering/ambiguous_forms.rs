//! Asks about written forms the analyzer sees whole (notes/welcoming.md): unbracketed list assignments (D12)
//! and suffix words mixed with infix arithmetic (D9); a trailing percent `10%` is read as `10/100` here too

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const BARE_LIST_TOPIC: &str = "bare-list";
const SUFFIX_PRECEDENCE_TOPIC: &str = "suffix-precedence";
/// `squared` is the suffix form of `square`, `sorted` of `sort` (wiki/function.md: every function generates a suffix operator)
const SUFFIX_WORD_ENDINGS: [&str; 2] = ["d", "ed"];
const PERCENT: i64 = 100;
const ARITHMETIC: [Op; 7] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Rem, Op::Pow];

pub fn lower(node: Node) -> Node {
	let words = suffix_words(&node);
	lower_node(node, &words)
}

/// The suffix words of the program's functions with the function each applies, `squared` → `square`; a name the
/// program assigns is its variable, never a suffix word (`solved = solve(x); return solved`)
fn suffix_words(node: &Node) -> SuffixWords {
	let mut variables = std::collections::HashSet::new();
	crate::library_words::collect_assigned_names(node, &mut variables);
	let functions = crate::analyzer::applicable_function_names(node);
	let words = functions.iter().flat_map(|function| past_forms(function).into_iter().map(|word| (word, function.clone())));
	let mut words: SuffixWords = words.filter(|(word, _)| word.ends_with("ed") && !variables.contains(word)).collect();
	// the library's own: `xs sorted`, `xs reversed`, `x squared` (x²), `x cubed` (x³), unless the program names them
	for (word, function) in LIBRARY_SUFFIX_WORDS {
		if !variables.contains(word) && !functions.contains(function.trim_start_matches(POWER_MARK)) {
			words.entry(word.to_string()).or_insert(function.to_string());
		}
	}
	words
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
		if !is_vowel(before) && is_vowel(vowel) && !is_vowel(last) && !UNDOUBLED_CONSONANTS.contains(last) && last.is_ascii_alphabetic() {
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
const LIBRARY_SUFFIX_WORDS: [(&str, &str); 4] = [("sorted", "sort"), ("reversed", "reverse"), ("squared", "^square"), ("cubed", "^cube")];
const POWER_MARK: char = '^';

/// suffix word → function
type SuffixWords = HashMap<String, String>;

fn lower_node(node: Node, functions: &SuffixWords) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_node(*node, functions)), data },
		// `10%`: a percent is a hundredth, `10/100`
		Node::Key(left, Op::Mod, right) if matches!(right.drop_meta(), Node::Empty) => Node::Key(Box::new(lower_node(*left, functions)), Op::Div, Box::new(Node::int(PERCENT))),
		Node::Key(left, op, right) => Node::Key(Box::new(lower_node(*left, functions)), op, Box::new(lower_node(*right, functions))),
		Node::List(items, bracket, separator) => {
			if let Some(asked) = bare_list_assignment(&items, &bracket, &separator) {
				return asked.map_or_else(|error| error, |chosen| lower_node(chosen, functions));
			}
			let items: Vec<Node> = items.into_iter().map(|item| lower_node(item, functions)).collect();
			apply_suffix_words(items, functions, bracket, separator)
		}
		other => other,
	}
}

/// `1`, `-1`, `"a"`, `'a'`
fn is_literal(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Neg, number) => matches!(number.drop_meta(), Node::Number(_)),
		other => matches!(other, Node::Number(_) | Node::Text(_) | Node::Char(_)),
	}
}

/// `a=1,2,3` / `a=1 2 3` without brackets: a list or separate statements? Unanswered an error (D12)
fn bare_list_assignment(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Result<Node, Node>> {
	if *bracket != Bracket::None || !matches!(separator, Separator::Space | Separator::Colon) {
		return None;
	}
	let (first, rest) = items.split_first()?;
	let Node::Key(name, Op::Assign, value) = first.drop_meta() else { return None };
	if rest.is_empty() || !is_literal(value) || !rest.iter().all(is_literal) {
		return None;
	}
	let values = std::iter::once(value.as_ref().clone()).chain(rest.iter().cloned()).collect();
	let list = Node::Key(name.clone(), Op::Assign, Box::new(Node::List(values, Bracket::Square, separator.clone())));
	let statements = Node::List(items.to_vec(), Bracket::None, Separator::Semicolon);
	let written = Node::List(items.to_vec(), Bracket::None, separator.clone()).serialize();
	let readings = vec![reading("a list", &list.serialize()), reading("separate statements", statements.serialize().trim())];
	let question = Ask::new(BARE_LIST_TOPIC, format!("is `{}` one list or separate statements?", written.trim()), readings, Fallback::Error)
		.written(written.trim()).at_node(first);
	Some(ask(&question).map(|chosen| if chosen == 0 { list } else { statements }))
}

/// The function a suffix word applies: `squared` → `square`
fn suffix_function(word: &Node, functions: &SuffixWords) -> Option<String> {
	let Node::Symbol(word) = word.drop_meta() else { return None };
	functions.get(word).cloned()
}

fn call(function: &str, argument: Node) -> Node {
	match function.strip_prefix(POWER_MARK) {
		Some(power) => {
			let exponent = if power == "square" { 2 } else { 3 };
			Node::Key(Box::new(argument), Op::Pow, Box::new(Node::int(exponent)))
		}
		None => Node::List(vec![Node::Symbol(function.to_string()), argument], Bracket::Round, Separator::None),
	}
}

/// The leftmost operand of an infix chain, replaced: `squared+1` → `(2 squared)+1`
fn with_leftmost(node: Node, replace: impl FnOnce(Node) -> Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_leftmost(*node, replace)), data },
		Node::Key(left, op, right) if ARITHMETIC.contains(&op) => Node::Key(Box::new(with_leftmost(*left, replace)), op, right),
		leaf => replace(leaf),
	}
}

/// The rightmost operand of an infix chain, replaced: `1+2` → `1+(2 squared)`
fn with_rightmost(node: Node, replace: impl FnOnce(Node) -> Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_rightmost(*node, replace)), data },
		Node::Key(left, op, right) if ARITHMETIC.contains(&op) => Node::Key(left, op, Box::new(with_rightmost(*right, replace))),
		leaf => replace(leaf),
	}
}

/// The suffix word heading `item`: `squared` or `squared+1`
fn heading_suffix_word(item: &Node, functions: &SuffixWords) -> Option<(Node, String)> {
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
	Node::List(vec![node], Bracket::Round, Separator::None)
}

/// `x squared` is `square(x)`; after ungrouped arithmetic (`1+2 squared`) the user is asked what the word applies to (D9)
fn apply_suffix_words(items: Vec<Node>, functions: &SuffixWords, bracket: Bracket, separator: Separator) -> Node {
	if !matches!(bracket, Bracket::None | Bracket::Round) || separator != Separator::Space {
		return Node::List(items, bracket, separator);
	}
	let item_count = items.len();
	let mut applied: Vec<Node> = Vec::new();
	for item in items {
		let (Some(operand), Some((word, function))) = (applied.last(), heading_suffix_word(&item, functions)) else {
			applied.push(item);
			continue;
		};
		let operand = operand.clone();
		let argument = match is_infix_arithmetic(&operand) {
			true => match suffix_precedence(&operand, &word) {
				Ok(true) => with_rightmost(operand, |leaf| call(&function, leaf)),
				Ok(false) => call(&function, operand),
				Err(error) => error,
			},
			false => call(&function, operand),
		};
		applied.pop();
		applied.push(with_leftmost(item, |_| argument));
	}
	match applied.len() {
		1 if applied.len() < item_count => applied.pop().expect("one item"),
		_ => Node::List(applied, bracket, separator),
	}
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
	let question = Ask::new(SUFFIX_PRECEDENCE_TOPIC, format!("does `{word_text}` in `{written}` apply to {} or to {whole_text}?", nearest.serialize()),
		readings, Fallback::Error).written(&written).at_node(word);
	ask(&question).map(|chosen| chosen == 0)
}
