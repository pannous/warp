//! `rewrite PATTERN => TEMPLATE` (card compiler-modules): code extending the compiler. A pattern's holes `$a` and
//! `$xs...` bind as in a code pattern of a match (switch.rs code_tests), the template splices them as a data template
//! (quasi_quote.rs). `where $c = channel()` holds where the hole binds a variable assigned code of that shape, or a
//! parameter of a function called with one; `$x: text` (also symbol, list, block, int, number) only code of that kind.
//! A phrase pattern (`$n beat`) matches each run of those words in a longer phrase or call (of `x = 2 beats`, which the
//! parser keeps as `(x = 2) beats`, the value), a statement pattern also a block of just that statement, and a
//! template of several statements `($a; $b)` takes the statement's place. Rules go bottom-up, children first, each node
//! rewritten once. A program's own rules apply to it; a standard module's rules apply to every program mentioning
//! their words but binding none itself, and one that fires brings its module along with `use` (lib/channels.warp,
//! lib/sound.warp).
use super::*;
use crate::lowering::nodes::block;
use crate::lowering::switch::{hole_name, rest_hole};

const REWRITE_WORD: &str = "rewrite";
const REWRITE_LINE_START: &str = "rewrite ";
/// The word between a rule's pattern and its template, as written and as parsed (decision rewrite-general: it may
/// become `as` or `with`, swapped here)
const ARROW: &str = "=>";
const ARROW_OP: Op = Op::Arrow;
const WHERE_CLAUSE: &str = " where ";
const WHERE_WORD: &str = "where";
/// The kinds a typed hole `$x: text` names
const HOLE_KINDS: [&str; 6] = ["text", "symbol", "list", "block", "int", "number"];

#[derive(Clone)]
struct Rule {
	pattern: Node,
	template: Node,
	/// `where $c = channel()`: the hole, and the shape of the code assigned to the variable it binds
	origin: Option<(String, Node)>,
	/// The standard module declaring the rule
	module: Option<&'static str>,
}

type Bindings = HashMap<String, Node>;

/// The program rewritten by its own rules and those of the standard modules it mentions, its own rules taken out
pub fn lower(program: Node) -> Node {
	let (program, mut rules) = without_rules(program, None);
	let mentioned = symbols(&program);
	for (module, source) in STD_MODULES {
		if rule_words(source).any(|word| mentioned.contains(word)) {
			rules.extend(standard_rules(module).into_iter().filter(|rule| mentioned.contains(needed_word(rule))));
		}
	}
	if rules.iter().any(|rule| rule.module.is_some()) {
		let bound = crate::program_names::Bindings::of(&program).bound();
		rules.retain(|rule| {
			rule.module.is_none() || !literal_words(&rule.pattern).iter().any(|word| bound.contains(*word))
		});
	}
	if rules.is_empty() {
		return program;
	}
	let origins = rules
		.iter()
		.map(|rule| rule.origin.as_ref().map(|(_, shape)| assigned(&program, shape)).unwrap_or_default())
		.collect();
	let mut rewrite = Rewrite { rules: &rules, origins, fired: vec![] };
	let program = rewrite.rewritten(program);
	let mut fired = rewrite.fired;
	fired.sort();
	fired.dedup();
	let uses = fired.iter().map(|module| crate::warp_parser::parse_generated(&format!("use {module}"))).collect();
	with_statements_first(program, uses)
}

/// The word each rule of a module needs, read off its text: a program mentioning none parses no rules
fn rule_words(source: &str) -> impl Iterator<Item = &str> {
	let is_word_part = |character: char| character.is_alphanumeric() || character == '_' || character == '$';
	source.lines().filter_map(|line| line.strip_prefix(REWRITE_LINE_START)).filter_map(move |rule| {
		let needing = match rule.split_once(WHERE_CLAUSE) {
			Some((_, origin)) => origin.split_once('=')?.1,
			None => rule.split(ARROW).next()?,
		};
		needing.split(|character| !is_word_part(character)).find(|word| word.starts_with(char::is_alphabetic))
	})
}

/// The word a program must mention for the rule to fire: the first of its origin's shape, else of its pattern
fn needed_word(rule: &Rule) -> &str {
	first_word_of(rule.origin.as_ref().map_or(&rule.pattern, |(_, shape)| shape)).unwrap_or_default()
}

fn standard_rules(module: &'static str) -> Vec<Rule> {
	thread_local!(static RULES: std::cell::RefCell<HashMap<&'static str, Vec<Rule>>> = Default::default());
	RULES.with_borrow_mut(|rules| {
		let parsed = || without_rules(parsed_std_module(module).expect("embedded"), Some(module)).1;
		rules.entry(module).or_insert_with(parsed).clone()
	})
}

/// The program without its top-level `rewrite` statements, and their rules
fn without_rules(program: Node, module: Option<&'static str>) -> (Node, Vec<Rule>) {
	let statements = crate::law::top_level_items(&program);
	if !statements.iter().any(|statement| rule_of(statement, module).is_some()) {
		return (program, vec![]);
	}
	let (rules, kept): (Vec<Node>, Vec<Node>) =
		statements.into_iter().partition(|statement| rule_of(statement, module).is_some());
	let rules = rules.iter().filter_map(|statement| rule_of(statement, module)).collect();
	(Node::List(kept, Bracket::None, Separator::Semicolon), rules)
}

/// `rewrite p => t` and `rewrite p => t where $c = shape`; the words of a pattern before its last part are its first
/// ones (`rewrite channel $name => …` reads as `rewrite channel ($name => …)`)
fn rule_of(statement: &Node, module: Option<&'static str>) -> Option<Rule> {
	let Node::List(items, Bracket::None, _) = statement.drop_meta() else {
		return None;
	};
	let (first, rest) = items.split_first()?;
	let (last, words) = rest.split_last()?;
	let Node::Key(last_word, ARROW_OP, right) = last.drop_meta() else {
		return None;
	};
	if !first.is_symbol(REWRITE_WORD) {
		return None;
	}
	let pattern = match words {
		[] => statement_pattern(last_word),
		_ => phrase(words.iter().chain([last_word.as_ref()]).flat_map(self::words).collect()),
	};
	let (template, origin) = match with_origin(right) {
		Some((template, hole, shape)) => (template, Some((hole_name(&hole)?.to_string(), shape))),
		None => (right.as_ref().clone(), None),
	};
	// `($a; $b)`: statements that take the rewritten statement's place
	let template = match template.drop_meta() {
		Node::List(statements, Bracket::Round, separator) if separator.separates_statements() => {
			Node::List(statements.clone(), Bracket::None, separator.clone())
		}
		_ => template,
	};
	Some(Rule { pattern, template, origin, module })
}

/// `t where $c = shape`: the template, the hole and the shape, however the words before `=` are grouped (on a line of
/// its own `((t where) $c) = shape`, else `t where ($c = shape)`)
fn with_origin(right: &Node) -> Option<(Node, Node, Node)> {
	let (mut parts, shape) = match right.drop_meta() {
		Node::Key(left, Op::Assign, shape) => (words(left), shape.as_ref().clone()),
		Node::List(parts, Bracket::None, _) => match parts.split_last()?.0.drop_meta() {
			Node::Key(hole, Op::Assign, shape) => {
				let before = parts[..parts.len() - 1].iter().flat_map(words);
				(before.chain([hole.as_ref().clone()]).collect(), shape.as_ref().clone())
			}
			_ => return None,
		},
		_ => return None,
	};
	let hole = parts.pop()?;
	parts.pop().filter(|word| word.is_symbol(WHERE_WORD))?;
	let template = match parts.as_slice() {
		[single] => single.clone(),
		several => phrase(several.to_vec()),
	};
	Some((template, hole, shape))
}

/// `(for $x in $c $body)`: parentheses around the words of a statement only group them
fn statement_pattern(pattern: &Node) -> Node {
	match pattern.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, Separator::Space) if items.len() > 1 => {
			phrase(items.iter().flat_map(words).collect())
		}
		_ => pattern.clone(),
	}
}

fn phrase(words: Vec<Node>) -> Node {
	Node::List(words, Bracket::None, Separator::Space)
}

/// The words of a phrase, its unbracketed groups taken apart
fn words(phrase: &Node) -> Vec<Node> {
	match phrase.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => items.iter().flat_map(words).collect(),
		_ => vec![phrase.clone()],
	}
}

/// The pattern's words that are no holes
fn literal_words(pattern: &Node) -> Vec<&str> {
	let mut found = vec![];
	pattern.visit(&mut |part| found.extend(part.symbol_name().filter(|_| hole_name(part).is_none())));
	found
}

fn first_word_of(pattern: &Node) -> Option<&str> {
	match pattern.drop_meta() {
		Node::Key(left, _, right) => first_word_of(left).or_else(|| first_word_of(right)),
		Node::List(items, _, _) => items.iter().find_map(first_word_of),
		part => part.symbol_name().filter(|_| hole_name(part).is_none()),
	}
}

fn symbols(program: &Node) -> HashSet<&str> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			names.insert(name.as_str());
		}
	});
	names
}

struct Rewrite<'a> {
	rules: &'a [Rule],
	origins: Vec<HashSet<String>>,
	/// The standard modules whose rules fired
	fired: Vec<&'static str>,
}

impl Rewrite<'_> {
	fn rewritten(&mut self, node: Node) -> Node {
		let node = node.map_children(|child| self.rewritten(child));
		let node = match node.drop_meta() {
			Node::List(items, bracket, separator) if separator.separates_statements() => {
				let statements = items.iter().flat_map(|statement| match statement.drop_meta() {
					Node::List(inner, Bracket::None, made) if made.separates_statements() => inner.clone(),
					_ => vec![statement.clone()],
				});
				Node::List(statements.collect(), bracket.clone(), separator.clone()).with_meta_of(&node)
			}
			Node::List(items, bracket, separator) => {
				Node::List(self.with_runs(items), bracket.clone(), separator.clone()).with_meta_of(&node)
			}
			_ => node,
		};
		for (rule, origin) in self.rules.iter().zip(&self.origins) {
			if is_run_pattern(&rule.pattern) {
				continue;
			}
			if let Some(result) = applied(rule, origin, &node) {
				self.fired.extend(rule.module);
				return result.with_meta_of(&node);
			}
		}
		node
	}

	/// The items with each run a phrase rule matches rewritten, left to right
	fn with_runs(&mut self, items: &[Node]) -> Vec<Node> {
		let mut items = items.to_vec();
		let mut at = 0;
		while at < items.len() {
			match self.run_at(&items[at..]) {
				Some((length, replacement)) => {
					let replaced = words(&replacement);
					let count = replaced.len();
					items.splice(at..at + length, replaced);
					at += count;
				}
				None => at += 1,
			}
		}
		items
	}

	/// The length of a run starting the items that a phrase rule matches, and its rewrite; a run starting with `x = v`
	/// or `x: v` rewrites v
	fn run_at(&mut self, items: &[Node]) -> Option<(usize, Node)> {
		for (rule, origin) in self.rules.iter().zip(&self.origins) {
			let Node::List(words, Bracket::None, Separator::Space) = rule.pattern.drop_meta() else {
				continue;
			};
			if !is_run_pattern(&rule.pattern) || items.len() < words.len() {
				continue;
			}
			let mut run = items[..words.len()].to_vec();
			let assigned = match run[0].drop_meta().clone() {
				Node::Key(target, op @ (Op::Assign | Op::Colon), value) => {
					run[0] = *value;
					Some((target, op))
				}
				_ => None,
			};
			let Some(result) = applied(rule, origin, &phrase(run)) else {
				continue;
			};
			self.fired.extend(rule.module);
			let result = match assigned {
				Some((target, op)) => Node::Key(target, op, Box::new(result)),
				None => result,
			};
			return Some((words.len(), result));
		}
		None
	}
}

/// A pattern of words without a rest: it matches runs of a phrase
fn is_run_pattern(pattern: &Node) -> bool {
	matches!(pattern.drop_meta(), Node::List(words, Bracket::None, Separator::Space) if !words.iter().any(|word| rest_hole(word).is_some()))
}

/// The rule's template for the node; a block's single statement keeps its block
fn applied(rule: &Rule, origin: &HashSet<String>, node: &Node) -> Option<Node> {
	let holds = |bindings: &Bindings| match &rule.origin {
		Some((hole, _)) => bindings.get(hole).and_then(Node::symbol_name).is_some_and(|name| origin.contains(name)),
		None => true,
	};
	let mut bindings = Bindings::new();
	if matches(&rule.pattern, node, &mut bindings) && holds(&bindings) {
		return Some(spliced(&rule.template, &bindings));
	}
	let (Node::List(_, Bracket::None, _), Node::List(statement, Bracket::Curly, separator)) =
		(rule.pattern.drop_meta(), node.drop_meta())
	else {
		return None;
	};
	let mut bindings = Bindings::new();
	let unbracketed = Node::List(statement.clone(), Bracket::None, separator.clone());
	(matches(&rule.pattern, &unbracketed, &mut bindings) && holds(&bindings))
		.then(|| block(spliced(&rule.template, &bindings).sequence_statements().to_vec()))
}

/// The code has the pattern's shape: a hole binds the subtree there, a repeated hole must be the same subtree, a key has
/// the same operator, a list the same bracket and length (a rest hole binds the items from there on), other parts equal
fn matches(pattern: &Node, code: &Node, bindings: &mut Bindings) -> bool {
	if let Some(name) = hole_name(pattern) {
		return bound(name, code.clone(), bindings);
	}
	if let Some((name, kind)) = typed_hole(pattern) {
		return is_kind(code, kind) && bound(name, code.clone(), bindings);
	}
	match (pattern.drop_meta(), code.drop_meta()) {
		(Node::Key(left, op, right), Node::Key(code_left, code_op, code_right)) => {
			op == code_op && matches(left, code_left, bindings) && matches(right, code_right, bindings)
		}
		(Node::List(items, bracket, _), Node::List(code_items, code_bracket, _)) => {
			bracket == code_bracket && items_match(items, code_items, bindings)
		}
		(Node::List(..) | Node::Key(..), _) | (_, Node::List(..) | Node::Key(..)) => false,
		(part, code_part) => part == code_part,
	}
}

/// `$x: text`: the hole and the kind of code it takes
fn typed_hole(pattern: &Node) -> Option<(&str, &str)> {
	let Node::Key(hole, Op::Colon, kind) = pattern.drop_meta() else {
		return None;
	};
	Some((hole_name(hole)?, kind.symbol_name().filter(|kind| HOLE_KINDS.contains(kind))?))
}

fn is_kind(code: &Node, kind: &str) -> bool {
	match (kind, code.drop_meta()) {
		("text", Node::Text(_)) | ("number", Node::Number(_)) | ("int", Node::Number(crate::Number::Int(_))) => true,
		("list", Node::List(_, Bracket::Square, _)) | ("block", Node::List(_, Bracket::Curly, _)) => true,
		("symbol", part @ Node::Symbol(_)) => hole_name(part).is_none(),
		_ => false,
	}
}

fn items_match(patterns: &[Node], items: &[Node], bindings: &mut Bindings) -> bool {
	let mut each = |patterns: &[Node], items: &[Node]| {
		patterns.len() == items.len()
			&& patterns.iter().zip(items).all(|(pattern, item)| matches(pattern, item, bindings))
	};
	let Some(rest_at) = patterns.iter().position(|pattern| rest_hole(pattern).is_some()) else {
		return each(patterns, items);
	};
	let after = patterns.len() - rest_at - 1;
	if items.len() < rest_at + after {
		return false;
	}
	let rest_end = items.len() - after;
	let rest = Node::List(items[rest_at..rest_end].to_vec(), Bracket::None, Separator::Space);
	each(&patterns[..rest_at], &items[..rest_at])
		&& each(&patterns[rest_at + 1..], &items[rest_end..])
		&& bound(rest_hole(&patterns[rest_at]).unwrap_or_default(), rest, bindings)
}

fn bound(name: &str, code: Node, bindings: &mut Bindings) -> bool {
	match bindings.get(name) {
		Some(earlier) => *earlier == code,
		None => bindings.insert(name.to_string(), code).is_none(),
	}
}

/// The template with each hole's code, a rest hole's items in its place
fn spliced(template: &Node, bindings: &Bindings) -> Node {
	if let Some(code) = hole_name(template).and_then(|name| bindings.get(name)) {
		return code.clone();
	}
	match template {
		Node::Key(left, op, right) => {
			Node::Key(Box::new(spliced(left, bindings)), *op, Box::new(spliced(right, bindings)))
		}
		Node::List(items, bracket, separator) => {
			let items = items.iter().flat_map(|item| match rest_hole(item).and_then(|name| bindings.get(name)) {
				Some(rest) if separator.separates_statements() => rest.body_statements(),
				Some(Node::List(rest, _, _)) => rest.clone(),
				_ => vec![spliced(item, bindings)],
			});
			Node::List(items.collect(), bracket.clone(), separator.clone())
		}
		Node::Meta { .. } => template.clone().under_meta(|inner| spliced(&inner, bindings)),
		other => other.clone(),
	}
}

/// The variables assigned code of the shape, then the parameters of the functions called with one, until none is new
fn assigned(program: &Node, shape: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	let mut functions: HashMap<String, Vec<String>> = HashMap::new();
	program.visit(&mut |part| match part {
		Node::Key(target, Op::Assign, value) if matches(shape, value, &mut Bindings::new()) => {
			names.extend(target.drop_meta().symbol_name().map(str::to_string));
		}
		Node::Key(head, Op::Define, _) => {
			if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
				if let Some((name, parameters)) = items.split_first() {
					functions.insert(name.name(), parameters.iter().map(Node::name).collect());
				}
			}
		}
		_ => {}
	});
	while !names.is_empty() && !functions.is_empty() {
		let mut found = vec![];
		program.visit(&mut |part| {
			let Node::List(items, Bracket::Round, _) = part else {
				return;
			};
			let Some(parameters) = items.split_first().and_then(|(callee, _)| functions.get(&callee.name())) else {
				return;
			};
			let given = items[1..].iter().zip(parameters).filter(|(argument, _)| names.contains(&argument.name()));
			found.extend(given.map(|(_, parameter)| parameter.clone()).filter(|parameter| !names.contains(parameter)));
		});
		if found.is_empty() {
			break;
		}
		names.extend(found);
	}
	names
}
