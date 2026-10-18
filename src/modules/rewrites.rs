//! `rewrite PATTERN => TEMPLATE` (card compiler-modules): code extending the compiler. A pattern's holes `$a` and
//! `$xs...` bind as in a code pattern of a match (switch.rs code_tests), the template splices them as a data template
//! (quasi_quote.rs). `where $c = channel()` holds where the hole binds a variable assigned code of that shape, or a
//! parameter of a function called with one. A statement pattern also matches a block of just that statement. Rules go
//! bottom-up, children first, each node rewritten once. A program's own rules apply to it; a standard module's rules
//! apply to every program mentioning their words, and one that fires brings its module along with `use`
//! (lib/channels.warp).
use super::*;
use crate::lowering::nodes::block;
use crate::lowering::switch::{hole_name, rest_hole};

const REWRITE_WORD: &str = "rewrite";
const REWRITE_LINE_START: &str = "rewrite ";
const ARROW: &str = "=>";
const WHERE_CLAUSE: &str = " where ";
const WHERE_WORD: &str = "where";

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
	if rules.is_empty() {
		return program;
	}
	let origins: Vec<HashSet<String>> = rules
		.iter()
		.map(|rule| rule.origin.as_ref().map(|(_, shape)| assigned(&program, shape)).unwrap_or_default())
		.collect();
	let mut fired = vec![];
	let program = rewritten(program, &rules, &origins, &mut fired);
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
	let Node::Key(last_word, Op::Arrow, right) = last.drop_meta() else {
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

fn rewritten(node: Node, rules: &[Rule], origins: &[HashSet<String>], fired: &mut Vec<&'static str>) -> Node {
	let node = node.map_children(|child| rewritten(child, rules, origins, fired));
	for (rule, origin) in rules.iter().zip(origins) {
		if let Some(result) = applied(rule, origin, &node) {
			fired.extend(rule.module);
			return result.with_meta_of(&node);
		}
	}
	node
}

/// The rule's template for the node, a block's single statement kept a block
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
		.then(|| block(vec![spliced(&rule.template, &bindings)]))
}

/// The code has the pattern's shape: a hole binds the subtree there, a repeated hole must be the same subtree, a key has
/// the same operator, a list the same bracket and length (a rest hole binds the items from there on), other parts equal
fn matches(pattern: &Node, code: &Node, bindings: &mut Bindings) -> bool {
	if let Some(name) = hole_name(pattern) {
		return bound(name, code.clone(), bindings);
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
