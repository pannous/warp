//! Blocks (wiki/charged.md sections 2, 4, 5; notes/blocks.md). Stage 1: `x : e` of a computed expression binds x to the
//! uncharged block e. `x!` and `x!!` run it where they are written (the block is a constant, so it is inlined there and its
//! names resolve at the `!`), a bare `x` is the block as data (`x : 1+2; x` shows 1+2), and `x` where a value is needed
//! (`x + 1`) is a type error with the fix. A literal or a plain word after `:` is simply that value (`age: 3`), types and
//! objects keep their meaning (`x : int`, `person: {…}`). A computed `:` gets a got-it warning where it is written.
//! `x = …` ends the block.
//! Stage 2: `x = {statements}` is a block too (no warning: braces say so), an object's computed `key: value` entry is an
//! uncharged block (`o.s1!` runs it, `o.s1 + 1` is the type error, the object holds it as data), and `obj!` of an object
//! literal runs it as code: each `key: value` is the call `key(value)` (`help!`). `key := value` entries are getters (P71): the method `o·key()`, run at every read `o.key`.
//! Stage 3: `x = code e` / `x = block e` are blocks, `y = data e; y!` runs with a got-it warning ("running data as code"),
//! and a function with a `block` parameter (`when_not(c, body:block) := if not c { body! }`) is expanded at every call:
//! its other parameters bound to fresh names, `body!` the argument's code, so names resolve where the call is written.
//! Any `!` left (known only at run time) is mutation.rs's: a name unwraps, any other expression is a loud error.

use super::nodes::{any_part, assign, call, children_rewritten, is_block, key, word_headed};
use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
use crate::mutation::bang_target;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::IMPLICIT_PARAMETER;
use std::collections::HashMap;

/// The data prefix a bare block name becomes: the block as it is written (list_emitter emit_quoted)
pub(crate) const DATA_WORD: &str = "data";
/// The got-it topic of `x : a+b`, which keeps a block where a value may have been meant
const BLOCK_TOPIC: &str = "uncharged-block";
/// Prefixes of a block: `code e`, `block e` (synonyms, wiki/charged.md section 4)
const BLOCK_WORDS: [&str; 2] = ["code", "block"];
/// The type word of a parameter taking a function or a block (card callable): `run(f: callable) := f()` runs
/// `run(square)`, `run(x => x * 2)` and `run({ print 1 })`; its aliases (type_tests BUILTIN_TYPES) are hinted
pub(crate) const CALLABLE_WORD: &str = "callable";
/// The got-it topic of `!` on data
const DATA_RUN_TOPIC: &str = "data-as-code";
/// `body·m0`: a parameter of an expanded block function, bound once
const EXPANSION_SEPARATOR: &str = "·m";

pub fn lower(program: Node) -> Node {
	let mut functions = HashMap::new();
	program.visit(&mut |node| {
		if let Some((name, function)) = block_function(node) {
			hint_callable_alias(node);
			functions.insert(name, function);
		}
	});
	let mut lowering = Blocks {
		blocks: HashMap::new(),
		objects: HashMap::new(),
		data: HashMap::new(),
		functions,
		expansions: 0,
		methods: HashMap::new(),
		getters: Default::default(),
	};
	lowering.statement(program)
}

/// A function with a `block` parameter: its parameters (name, what it takes) and body
#[derive(Clone)]
struct BlockFunction {
	parameters: Vec<(String, Takes)>,
	body: Node,
}

/// What a parameter of a block function takes
#[derive(Clone, Copy, PartialEq)]
enum Takes {
	Value,
	Block,
	/// a block or a function, as the argument is written
	Callable,
}

impl Takes {
	fn of_type(word: &str) -> Takes {
		match word {
			_ if BLOCK_WORDS.contains(&word) => Takes::Block,
			_ if crate::type_tests::builtin_type_name(word) == Some(CALLABLE_WORD) => Takes::Callable,
			_ => Takes::Value,
		}
	}
}

/// `f(c, body:block) := …`: a definition with at least one `block` (or `code`) parameter; a parameter named `block` or
/// `code` without a type is one too when the body runs it (`def repeats(n: int, block) := … block() …`), else a value
fn block_function(node: &Node) -> Option<(String, BlockFunction)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else {
		return None;
	};
	let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else {
		return None;
	};
	let (name, parameters) = word_headed(items)?;
	let parameters: Vec<(String, Takes)> = parameters
		.iter()
		.map(|parameter| match parameter.drop_meta() {
			Node::Symbol(name) => {
				let block = BLOCK_WORDS.contains(&name.as_str()) && runs_anywhere(body, name);
				Some((name.clone(), if block { Takes::Block } else { Takes::Value }))
			}
			Node::Key(name, Op::Colon, kind) => {
				Some((name.name(), kind.drop_meta().symbol_name().map_or(Takes::Value, Takes::of_type)))
			}
			_ => None,
		})
		.collect::<Option<_>>()?;
	parameters
		.iter()
		.any(|(_, takes)| *takes != Takes::Value)
		.then(|| (name.to_string(), BlockFunction { parameters, body: body.as_ref().clone() }))
}

/// The argument of a callable parameter that is code to run: `{ print 1 }`, `code e`, the rest of a call
fn is_code_argument(argument: &Node) -> bool {
	is_block(argument)
		|| prefixed(argument, &BLOCK_WORDS).is_some()
		|| matches!(argument.drop_meta(), Node::List(_, Bracket::None, Separator::Space))
}

/// `f(x: invocable)`: the hint to write callable
fn hint_callable_alias(node: &Node) {
	node.visit(&mut |part| {
		if let Node::Key(name, Op::Colon, kind) = part.drop_meta() {
			let alias = kind.drop_meta().symbol_name().filter(|word| *word != CALLABLE_WORD);
			if let Some(alias) = alias.filter(|word| Takes::of_type(word) == Takes::Callable) {
				crate::normalize::set_position_of(part);
				let name = name.name();
				crate::normalize::hint(
					&format!("{name}: {alias}"),
					&format!("{name}: {CALLABLE_WORD}"),
					"callable is the type word for a function or a block",
				);
			}
		}
	});
}

/// The prefixed value `code e` / `data e`: the prefix and e
fn prefixed<'a>(value: &'a Node, words: &[&str]) -> Option<&'a Node> {
	let Node::List(items, Bracket::None, Separator::Space) = value.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[prefix, expression] if matches!(prefix.drop_meta(), Node::Symbol(word) if words.contains(&word.as_str())) => {
			Some(expression)
		}
		_ => None,
	}
}

/// `data e`: e, the expression as written
pub(crate) fn quoted_data(node: &Node) -> Option<&Node> {
	let quoted = prefixed(node, &[DATA_WORD])?;
	Some(sealed_code(quoted).unwrap_or(quoted))
}

/// Quoted code the later passes do not see into (card data-quote-lowered): `data a.b` keeps its `.`, as the run-time
/// parse does; `unsealed` gives it back before analysis and emission
#[derive(Clone, PartialEq)]
struct QuotedCode(Node);

fn sealed_code(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Data(data) => data.downcast_ref::<QuotedCode>().map(|QuotedCode(code)| code),
		_ => None,
	}
}

/// `as_code(data e)`: e as code, of the type code (decision body-code)
pub(crate) fn quoted_code(code: Node) -> Node {
	crate::node::call(crate::words::AS_CODE, vec![as_data(code)])
}

/// `data e`: e as written, data rather than code
pub(crate) fn as_data(code: Node) -> Node {
	Node::List(vec![symbol(DATA_WORD), code], Bracket::None, Separator::Space)
}

/// `data e` with e sealed
pub(crate) fn quote(code: Node) -> Node {
	let code = match sealed_code(&code) {
		Some(_) => code,
		None => Node::data(QuotedCode(code)),
	};
	as_data(code)
}

/// Every `data e` sealed, so the passes after blocks leave e as written; `data(e)` too, which the parser keeps as a
/// call until blocks::lower reads it (card data-pattern-sealed)
pub fn seal_quoted(node: Node) -> Node {
	match written_quote(&node) {
		Some(code) => quote(code),
		None => children_rewritten(node, seal_quoted),
	}
}

/// The code of `data e`, `data(e)` and `data(a of b)`, whose words the parser keeps as the items after data
fn written_quote(node: &Node) -> Option<Node> {
	let grouped = prefixed_group(node);
	let Node::List(items, Bracket::None, Separator::Space) = grouped.as_ref().unwrap_or(node).drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[prefix, code] if prefix.is_symbol(DATA_WORD) => Some(code.clone()),
		[prefix, words @ ..] if prefix.is_symbol(DATA_WORD) && words.len() > 1 => {
			Some(Node::List(words.to_vec(), Bracket::None, Separator::Space))
		}
		_ => None,
	}
}

/// Every sealed `data e` open again
pub fn unsealed(node: Node) -> Node {
	match sealed_code(&node) {
		Some(code) => code.clone(),
		None => children_rewritten(node, unsealed),
	}
}

/// `(data q+1)` is `data q+1` and `[data a]` the list of one item `data a`: a prefix takes the rest of its group,
/// it is no function to call
/// `string(data x+1)`: the cast of the one argument `data x+1`
fn prefixed_group(node: &Node) -> Option<Node> {
	let Node::List(items, bracket @ (Bracket::Round | Bracket::Square), Separator::None | Separator::Space) =
		node.drop_meta()
	else {
		return None;
	};
	let is_prefix = |item: &Node| matches!(item.drop_meta(), Node::Symbol(word) if word == DATA_WORD || BLOCK_WORDS.contains(&word.as_str()));
	match items.as_slice() {
		[prefix, _, ..] if is_prefix(prefix) => {
			let prefixed = Node::List(items.clone(), Bracket::None, Separator::Space);
			Some(match bracket {
				Bracket::Square => Node::List(vec![prefixed], Bracket::Square, Separator::None),
				_ => prefixed,
			})
		}
		// only text casts: `f(block rest)` passes a variable named block, interpret and print take the words as they are
		[function, prefix, _, ..]
			if *bracket == Bracket::Round
				&& crate::printable::is_text_word(function)
				&& prefix.drop_meta().name() == DATA_WORD =>
		{
			let argument = Node::List(items[1..].to_vec(), Bracket::None, Separator::Space);
			Some(Node::List(vec![function.clone(), argument], Bracket::Round, Separator::None))
		}
		_ => None,
	}
}

/// In a body: `param!` the argument's code (a group), a bare `param` the argument as data
fn substitute_block(node: Node, parameter: &str, argument: &Node) -> Node {
	if runs_parameter(&node, parameter) {
		return parens(vec![argument.clone()]);
	}
	match node {
		Node::Symbol(name) if name == parameter => as_data(argument.clone()),
		other => children_rewritten(other, |child| substitute_block(child, parameter, argument)),
	}
}

/// `body!` or `body()` of the block parameter body
fn runs_parameter(node: &Node, parameter: &str) -> bool {
	let is_parameter = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if name == parameter);
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) => matches!(items.as_slice(), [only] if is_parameter(only)),
		_ => bang_target(node).is_some_and(|(target, _)| is_parameter(&target)),
	}
}

/// Whether the body runs the parameter somewhere (`block!`, `block()`)
fn runs_anywhere(body: &Node, parameter: &str) -> bool {
	let mut runs = false;
	body.visit(&mut |node| runs |= runs_parameter(node, parameter));
	runs
}

/// The arguments of a call to a function with these parameters: one each, or, when the last parameter is a block, the
/// rest of the call as its argument (`twice(print "hi")` is read as the items twice, print, "hi")
fn fitted_arguments(parameters: &[(String, Takes)], arguments: &[Node]) -> Option<Vec<Node>> {
	if arguments.len() == parameters.len() {
		return Some(arguments.to_vec());
	}
	let last_is_block = parameters.last().is_some_and(|(_, takes)| *takes != Takes::Value);
	if !last_is_block || arguments.len() < parameters.len() {
		return None;
	}
	let (single, rest) = arguments.split_at(parameters.len() - 1);
	let mut fitted = single.to_vec();
	fitted.push(Node::List(rest.to_vec(), Bracket::None, Separator::Space));
	Some(fitted)
}

struct Blocks {
	/// The blocks in scope: a name (`x`) or a field (`o.s1`) → the expression
	blocks: HashMap<String, Node>,
	/// Object literals in scope, by name: their entries, to run the object as code (`help!`)
	objects: HashMap<String, Vec<(Node, Node)>>,
	/// Names holding `data e`: e, run only with a warning
	data: HashMap<String, Node>,
	/// Functions with a `block` parameter, expanded at every call
	functions: HashMap<String, BlockFunction>,
	expansions: usize,
	/// Function entries of objects in scope: `o.f` → the top-level function `o·f` it became
	methods: HashMap<String, Method>,
	/// The getter entries among them (`s := e`, P71): `o.s` without parentheses calls them
	getters: std::collections::HashSet<String>,
}

struct Method {
	function: String,
	/// Whether it takes the object as its first parameter (its body reads the other fields)
	takes_object: bool,
	/// Its parameters as written in the entry
	parameters: Vec<Node>,
}

/// `x : a+b` of a computed expression: the name and the block
pub(crate) fn uncharged(node: &Node) -> Option<(String, Node)> {
	let Node::Key(target, Op::Colon, value) = node.drop_meta() else {
		return None;
	};
	let name = target.symbol_name()?;
	is_computed(value).then(|| (name.to_string(), value.as_ref().clone()))
}

/// Code that computes: an operation or a call, not a literal, a word, a type or an object/list literal
fn is_computed(value: &Node) -> bool {
	match value.drop_meta() {
		// a quantity `5 m` is a value, written as data
		Node::Key(amount, Op::Mul, unit)
			if matches!(amount.drop_meta(), Node::Number(_))
				&& matches!(unit.drop_meta(), Node::Symbol(name) if crate::numbers::units::is_unit(name)) =>
		{
			false
		}
		// `now!`: the time the run reads where it stands is a value, not a block kept for later (card map-now-bang)
		_ if crate::time::is_time_call(value) => false,
		// a range `1..n` is a value too (a descriptor), whatever its bounds compute
		Node::Key(_, Op::Range | Op::To, _) => false,
		// a type expression declares: `int[100]`, `100 * int`, `3 * char`
		Node::Key(left, op, right) => {
			let is_type = |side: &Node| {
				matches!(side.drop_meta(), Node::Symbol(word)
				if crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some())
			};
			!is_type(left) && !is_type(right) && !matches!(op, Op::Colon | Op::Assign | Op::Define)
		}
		// a call `f(x)`; `100 int` (a typed array) and other spaced words stay as they are
		Node::List(items, Bracket::Round, Separator::None) => items.len() > 1,
		_ => false,
	}
}

/// `s = e`, or `s := {statements}` (a getter `s := e` is a function entry, P71): the field and e
fn value_entry(entry: &Node) -> Option<(&Node, &Node)> {
	match entry.drop_meta() {
		Node::Key(field, Op::Assign, value) => Some((field, value)),
		Node::Key(field, Op::Define, value)
			if !crate::warp_parser::mentions(value, IMPLICIT_PARAMETER)
				&& crate::getters::getter_name(entry).is_none() =>
		{
			Some((field, value))
		}
		_ => None,
	}
}

/// A function entry of an object: `f := it*2`, `f := x => …`, `f = x => …`, `f: x => …` or `f(x) := …`; its field,
/// parameters and body
fn function_entry(entry: &Node) -> Option<(String, Vec<Node>, Node)> {
	fn lambda(value: &Node) -> Option<(Node, Node)> {
		match value.drop_meta() {
			Node::Key(parameters, Op::Arrow, body) => Some((parameters.as_ref().clone(), body.as_ref().clone())),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => lambda(&items[0]), // `f: (x => x * 2)`
			_ => None,
		}
	}
	let parameter_list = |parameters: Node| match parameters.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.clone(),
		_ => vec![parameters],
	};
	match entry.drop_meta() {
		Node::Key(head, Op::Define, body) => match head.drop_meta() {
			Node::Symbol(field) => match lambda(body) {
				Some((parameters, body)) => Some((field.clone(), parameter_list(parameters), body)),
				None => (crate::warp_parser::mentions(body, IMPLICIT_PARAMETER)
					|| crate::getters::getter_name(entry).is_some())
				.then(|| (field.clone(), vec![], body.as_ref().clone())),
			},
			Node::List(items, Bracket::Round, Separator::None) => match items.split_first() {
				Some((name, parameters)) if matches!(name.drop_meta(), Node::Symbol(_)) => {
					Some((name.name(), parameters.to_vec(), body.as_ref().clone()))
				}
				_ => None,
			},
			_ => None,
		},
		Node::Key(head, Op::Assign | Op::Colon, value) => match (head.drop_meta(), lambda(value)) {
			(Node::Symbol(field), Some((parameters, body))) => Some((field.clone(), parameter_list(parameters), body)),
			_ => None,
		},
		_ => None,
	}
}

/// `o = {f := it*2}`, which the parser reads as `o := …` for its `it`: the assignment of an object whose `it` is only in
/// function entries
fn object_assignment(node: Node) -> Node {
	if let Node::Key(target, Op::Define, value) = node.drop_meta() {
		let mentions_it = |entry: &Node| crate::warp_parser::mentions(entry, IMPLICIT_PARAMETER);
		let only_functions_mention_it = |entries: &Vec<Node>| {
			entries.iter().any(mentions_it)
				&& entries.iter().filter(|entry| mentions_it(entry)).all(|entry| function_entry(entry).is_some())
		};
		if matches!(target.drop_meta(), Node::Symbol(_)) && object_entries(value).is_some_and(only_functions_mention_it)
		{
			return Node::Key(target.clone(), Op::Assign, value.clone());
		}
	}
	node
}

/// The entries of an object literal `{k: v, k = v, f(x) := …}`: every item names a key
fn object_entries(value: &Node) -> Option<&Vec<Node>> {
	let Node::List(items, Bracket::Curly, _) = value.drop_meta() else {
		return None;
	};
	let is_entry = |item: &Node| {
		function_entry(item).is_some()
			|| matches!(item.drop_meta(), Node::Key(key, Op::Colon | Op::Assign | Op::Define, _) if matches!(key.drop_meta(), Node::Symbol(_)))
	};
	(!items.is_empty() && items.iter().all(is_entry)).then_some(items)
}

/// `{1+2}`, `{a = 1; a*2}` assigned: statements in braces (several, or one computed expression), no object, no data
/// list (`{1 2}`) and no lambda (`{it*2}`, `{x => x+1}`, `{ x in x*2 }`, `{ $0 * 2 }`)
fn statement_block(value: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Curly, separator) = value.drop_meta() else {
		return None;
	};
	let statements = matches!(separator, Separator::Semicolon | Separator::Newline) && items.len() > 1;
	let lambda = items.iter().any(|item| matches!(item.drop_meta(), Node::Key(_, Op::Arrow, _)))
		|| crate::lambdas::arrow_lambda(value).is_some();
	if items.is_empty()
		|| object_entries(value).is_some()
		|| crate::warp_parser::mentions(value, IMPLICIT_PARAMETER)
		|| lambda
	{
		return None;
	}
	if !statements && !matches!(items.as_slice(), [single] if is_computed(single)) {
		return None;
	}
	Some(match items.as_slice() {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::Round, separator.clone()),
	})
}

/// The object `o.a` of a nested function entry as a parameter name, `o·a`
fn object_parameter(path: &str) -> String {
	path.replace('.', METHOD_SEPARATOR)
}

/// `o.a.f` as the field reads it names
fn path_node(path: &str) -> Node {
	let mut parts = path.split('.').map(symbol);
	let first = parts.next().unwrap_or(Node::Empty);
	parts.fold(first, |object, field| key(object, Op::Dot, field))
}

/// `kill{print '🕱'}`, `reset{a = 0; print a}` as an entry: braces holding a statement, the field and its block (card
/// symbolism-object); the braces say it is code, so no warning. A value in braces is content: markup's
/// `p{"clicked " + count}` stays an element
fn code_entry(entry: &Node) -> Option<(String, Node)> {
	let Node::Key(target, Op::Colon, value) = entry.drop_meta() else {
		return None;
	};
	let field = target.symbol_name()?;
	let Node::List(items, Bracket::Curly, separator) = value.drop_meta() else {
		return None;
	};
	let is_statement = |item: &Node| crate::analyzer::is_statement(item, &Bracket::Curly);
	let statement = Node::List(items.clone(), Bracket::None, separator.clone());
	let block = match statement_block(value) {
		Some(block) if items.iter().any(is_statement) => block,
		_ if *separator == Separator::Space && is_statement(&statement) => statement,
		_ => return None,
	};
	Some((field.to_string(), block))
}

/// An entry the object lowering rewrites: a block, a function, a value entry, or a nested object holding one
fn is_rewritten_entry(entry: &Node) -> bool {
	code_entry(entry).is_some()
		|| uncharged(entry).is_some()
		|| function_entry(entry).is_some()
		|| value_entry(entry).is_some()
		|| nested_object(entry).is_some()
}

/// `a: {f: x => x + 1}`: an entry whose value is an object with rewritten entries; its field, the object and its entries
fn nested_object(entry: &Node) -> Option<(String, &Node, &Vec<Node>)> {
	let Node::Key(key, Op::Colon, value) = entry.drop_meta() else {
		return None;
	};
	let field = key.symbol_name()?;
	let entries = object_entries(value).filter(|entries| entries.iter().any(is_rewritten_entry))?;
	Some((field.to_string(), value, entries))
}

/// `o·f`: the top-level function of the function entry `f` of the object `o` (no name the parser reads)
const METHOD_SEPARATOR: &str = "·";
/// Expansions of block functions in one program: a function expanding itself without end stops here
const MAX_EXPANSIONS: usize = 1000;

/// `node` with its innermost value replaced, keeping the meta information around it (`@unit("cm") {x:1}`)
fn with_inner(node: &Node, inner: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_inner(node, inner)), data: data.clone() },
		_ => inner,
	}
}

/// `x` or `o.s1`: the key of a block
fn block_key(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(object, Op::Dot, field) => match field.drop_meta() {
			Node::Symbol(field) => Some(format!("{}.{field}", block_key(object)?)),
			_ => None,
		},
		_ => None,
	}
}

fn type_error(name: &str, block: &Node) -> Node {
	let written = crate::normalize::operand_text(block);
	crate::node::error(&format!(
		"{name} is a block ({written}), no value: run it with {name}!, or write {name} = {written} for its value"
	))
}

/// `run(5)` of `run(f: callable)`: a value that is neither a function nor a block
fn not_callable(function: &str, parameter: &str, argument: &Node, kind: Kind) -> Node {
	let written = crate::normalize::operand_text(argument);
	crate::node::error(&format!(
		"type error: {function} takes a function or a block for {parameter}: {CALLABLE_WORD}, not {} {written}; fix: \
		 {function}({{ {written} }}) to run code, or {function}(() => {written}) for a function giving it",
		crate::analyzer::kind_with_article(if kind == Kind::Codepoint { Kind::Text } else { kind })
	))
}

fn applied(function: &Node, argument: Node) -> Node {
	parens(vec![function.clone(), argument])
}

impl Blocks {
	/// The got-it warning of a computed `:` where it is written: `name` keeps a block where a value may have been meant
	fn warn_uncharged(&self, name: &str, block: &Node, at: &Node) -> Result<(), Node> {
		let written = crate::normalize::operand_text(block);
		let question = Ask::new(
			BLOCK_TOPIC,
			format!(
				"{name} keeps the block {written}, it runs only at {name}!; write {name} = {written} for its value"
			),
			vec![
				reading("a block", &format!("{name} : {written}")),
				reading("its value", &format!("{name} = {written}")),
			],
			Fallback::Warning,
		)
		.written(&format!("{name} : {written}"))
		.at_node(at);
		ask(&question).map(|_| ())
	}

	/// A statement: only here `x : e` binds a block (a branch `c : x`, a case `1: 10` or an entry is no statement)
	fn statement(&mut self, node: Node) -> Node {
		if let Some((name, block)) = uncharged(&node) {
			if let Err(error) = self.warn_uncharged(&name, &block, &node) {
				return error;
			}
			let block = self.rewrite(block);
			self.blocks.insert(name, block);
			return Node::Empty;
		}
		// a function with a `block` parameter exists only in its expansions
		if block_function(&node).is_some() {
			return Node::Empty;
		}
		let node = object_assignment(node);
		if let Node::Key(target, Op::Assign, value) = node.drop_meta() {
			if let Node::Symbol(name) = target.drop_meta() {
				// `x = code 1+2`, `x = block e`: a block
				if let Some(block) = prefixed(value, &BLOCK_WORDS) {
					let block = self.rewrite(block.clone());
					self.forget(name);
					self.blocks.insert(name.clone(), block);
					return Node::Empty;
				}
				// `y = data 1+2`: data, which `y!` runs with a warning
				if let Some(data) = prefixed(value, &[DATA_WORD]) {
					let data = data.clone();
					let name = name.clone();
					let lowered = self.rewrite(node);
					self.data.insert(name, data);
					return lowered;
				}
				// `x = {1+2}`: statements in braces are a block
				if let Some(block) = statement_block(value) {
					let block = self.rewrite(block);
					self.forget(name);
					self.blocks.insert(name.clone(), block);
					return Node::Empty;
				}
				// a plain object stays as it is; its entries can run as code (`help!`)
				if let Some(entries) = object_entries(value).filter(|entries| !entries.iter().any(is_rewritten_entry)) {
					let pairs: Vec<(Node, Node)> = entries
						.iter()
						.filter_map(|entry| match entry.drop_meta() {
							Node::Key(key, Op::Colon, value) => Some((key.as_ref().clone(), value.as_ref().clone())),
							_ => None,
						})
						.collect();
					let name = name.clone();
					let lowered = self.rewrite(node);
					self.objects.insert(name, pairs);
					return lowered;
				}
				if let Some(object) = self.object_statement(name, target, Op::Assign, value) {
					return object;
				}
			}
		}
		// the tagged object `cat{x=1 kill{print '🕱'}}` as `cat = {…}` (card symbolism-object); plain `key: value` data keeps
		// reading its siblings' paths (`circle:{color: colors.red}`); every `=` in a tag is a value entry, `p{shown = "seen"}`
		// is p{shown:"seen"} (user, decision tag-assign)
		if let Node::Key(target, Op::Colon, value) = node.drop_meta() {
			let is_code = |entry: &Node| {
				code_entry(entry).is_some() || value_entry(entry).is_some() || function_entry(entry).is_some()
			};
			if let (Node::Symbol(name), Some(_)) =
				(target.drop_meta(), object_entries(value).filter(|entries| entries.iter().any(is_code)))
			{
				if let Some(object) = self.object_statement(name, target, Op::Colon, value) {
					return object;
				}
			}
		}
		self.rewrite(node)
	}

	/// `o = {s1: a+b, …}`: computed entries are blocks the object holds as data, `s3 = …` entries values; preceded by the
	/// definitions of its function entries
	fn object_statement(&mut self, name: &str, target: &Node, op: Op, value: &Node) -> Option<Node> {
		let entries = object_entries(value).filter(|entries| entries.iter().any(is_rewritten_entry))?;
		self.forget(name);
		let (mut definitions, object) = match self.object_with_entries(name, value, entries) {
			Ok(lowered) => lowered,
			Err(error) => return Some(error),
		};
		let object = key(target.clone(), op, object);
		if definitions.is_empty() {
			return Some(object);
		}
		definitions.push(object);
		Some(Node::List(definitions, Bracket::None, Separator::Semicolon))
	}

	/// The object literal `value` at `path` (`o`, or `o.a` of a nested object) with its entries lowered: the definitions of
	/// its function entries (nested objects' too) and the object holding them as data
	fn object_with_entries(&mut self, path: &str, value: &Node, entries: &[Node]) -> Result<(Vec<Node>, Node), Node> {
		let mut lowered = vec![];
		let mut definitions = vec![];
		for entry in entries {
			if let Some((field, parameters, body)) = function_entry(entry) {
				if crate::getters::getter_name(entry).is_some() {
					self.getters.insert(format!("{path}.{field}"));
				}
				definitions.push(self.method(path, &field, parameters, body, entries));
				let written = match entry.drop_meta() {
					Node::Key(head, _, value) if matches!(head.drop_meta(), Node::Symbol(_)) => value.as_ref().clone(),
					whole => whole.clone(),
				};
				let written = as_data(written);
				lowered.push(key(Node::Symbol(field), Op::Colon, written));
				continue;
			}
			// `x = {a: {f: x => x + 1}}`: the nested object's function entries are methods of `x.a`
			if let Some((field, inner, inner_entries)) = nested_object(entry) {
				let (inner_definitions, inner) =
					self.object_with_entries(&format!("{path}.{field}"), inner, inner_entries)?;
				definitions.extend(inner_definitions);
				lowered.push(key(Node::Symbol(field), Op::Colon, inner));
				continue;
			}
			let block_entry = match (code_entry(entry), uncharged(entry)) {
				(Some(code), _) => Some(code),
				(None, Some((field, block))) => {
					self.warn_uncharged(&field, &block, entry)?;
					Some((field, block))
				}
				(None, None) => None,
			};
			lowered.push(match block_entry {
				Some((field, block)) => {
					let block = self.rewrite(block);
					self.blocks.insert(format!("{path}.{field}"), block.clone());
					let data = as_data(block);
					key(Node::Symbol(field), Op::Colon, data)
				}
				// `s3 = a+b`, `s3 := a+b` (P71 open: now): a value entry, evaluated now
				None => match value_entry(entry) {
					Some((field, value)) => {
						let value = self.rewrite(value.clone());
						key(field.clone(), Op::Colon, parens(vec![value]))
					}
					None => self.rewrite(entry.clone()),
				},
			});
		}
		let pairs = lowered
			.iter()
			.filter_map(|entry| match entry.drop_meta() {
				Node::Key(key, Op::Colon, value) => Some((key.as_ref().clone(), value.as_ref().clone())),
				_ => None,
			})
			.collect();
		self.objects.insert(path.to_string(), pairs);
		let separator = match value.drop_meta() {
			Node::List(_, _, separator) => separator.clone(),
			_ => Separator::Space,
		};
		Ok((definitions, with_inner(value, Node::List(lowered, Bracket::Curly, separator))))
	}

	/// The function entry `o.f` as the top-level definition `o·f(parameters) := body`; a body reading the object's other
	/// fields takes the object as its first parameter `o`, the fields read as `o.a`
	fn method(
		&mut self,
		object: &str,
		field: &str,
		mut parameters: Vec<Node>,
		mut body: Node,
		entries: &[Node],
	) -> Node {
		let written_parameters = parameters.clone();
		let parameter_names: Vec<String> = parameters.iter().map(|parameter| parameter.drop_meta().name()).collect();
		let fields: Vec<String> = entries
			.iter()
			.filter(|entry| function_entry(entry).is_none())
			.filter_map(|entry| match entry.drop_meta() {
				Node::Key(key, _, _) => Some(key.drop_meta().name()),
				_ => None,
			})
			.filter(|other| {
				other != field && !parameter_names.contains(other) && crate::warp_parser::mentions(&body, other)
			})
			.collect();
		for other in &fields {
			let read = key(Node::Symbol(object_parameter(object)), Op::Dot, Node::Symbol(other.clone()));
			body = crate::library_words::substitute(body, other, &read);
		}
		let takes_object = !fields.is_empty();
		if takes_object {
			if parameters.is_empty() && crate::warp_parser::mentions(&body, IMPLICIT_PARAMETER) {
				parameters.push(symbol(IMPLICIT_PARAMETER));
			}
			parameters.insert(0, Node::Symbol(object_parameter(object)));
		}
		let mut name = format!("{}{METHOD_SEPARATOR}{field}", object_parameter(object));
		if self.methods.values().any(|taken| taken.function == name) {
			name = format!("{name}{METHOD_SEPARATOR}{}", self.methods.len());
		}
		self.methods.insert(
			format!("{object}.{field}"),
			Method { function: name.clone(), takes_object, parameters: written_parameters },
		);
		let head = match parameters.is_empty() {
			true => Node::Symbol(name),
			false => call(&name, parameters),
		};
		let body = self.rewrite(body);
		key(head, Op::Define, body)
	}

	/// `o.f(3)`, `o.f 3` of a function entry, `o.s` of a getter entry: the call of its top-level function
	fn method_call(&mut self, node: &Node) -> Option<Node> {
		let (method, arguments, bracket, separator) = match node.drop_meta() {
			Node::Key(object, Op::Dot, call) => match call.drop_meta() {
				Node::List(items, Bracket::Round, Separator::None) => (
					block_key(&Node::Key(object.clone(), Op::Dot, Box::new(items.first()?.clone())))?,
					items[1..].to_vec(),
					Bracket::Round,
					Separator::None,
				),
				Node::Symbol(_) => match block_key(node)? {
					getter if self.getters.contains(&getter) => (getter, vec![], Bracket::Round, Separator::None),
					entry => return self.method_value(&entry),
				},
				_ => return None,
			},
			Node::List(items, Bracket::None, Separator::Space) => (
				block_key(items.first()?).filter(|key| key.contains('.'))?,
				items[1..].to_vec(),
				Bracket::None,
				Separator::Space,
			),
			_ => return None,
		};
		let arguments: Vec<Node> = arguments.into_iter().map(|argument| self.rewrite(argument)).collect();
		self.method_call_of(&method, arguments, bracket, separator)
	}

	fn method_call_of(
		&self,
		method: &str,
		arguments: Vec<Node>,
		bracket: Bracket,
		separator: Separator,
	) -> Option<Node> {
		let Method { function, takes_object, .. } = self.methods.get(method)?;
		let object = takes_object.then(|| path_node(method.rsplit_once('.').map_or(method, |(object, _)| object)));
		Some(Node::List(
			[vec![Node::Symbol(function.clone())], object.into_iter().collect(), arguments].concat(),
			bracket,
			separator,
		))
	}

	/// `h = o.f` of a function entry: the function as a value, `function o·f`, or the lambda calling it with the object
	fn method_value(&self, method: &str) -> Option<Node> {
		let Method { function, takes_object, parameters } = self.methods.get(method)?;
		if !takes_object {
			return Some(crate::closures::function_reference(function.clone()));
		}
		let names: Vec<Node> = parameters.iter().map(|parameter| Node::Symbol(parameter.drop_meta().name())).collect();
		let call = self.method_call_of(method, names.clone(), Bracket::Round, Separator::None)?;
		Some(key(parens(names), Op::Arrow, call))
	}

	/// A name given a new value: its blocks and object entries are gone
	fn forget(&mut self, name: &str) {
		self.blocks.remove(name);
		self.objects.remove(name);
		self.data.remove(name);
		let prefix = format!("{name}.");
		self.blocks.retain(|key, _| !key.starts_with(&prefix));
		self.methods.retain(|key, _| !key.starts_with(&prefix));
		self.getters.retain(|key| !key.starts_with(&prefix));
	}

	/// `x!`, `o.s1!`, `help!`: the block inlined where it runs, or the object run as code (each `key: value` the call
	/// `key(value)`); None for anything else (mutation.rs decides)
	fn run(&self, target: &Node) -> Option<Node> {
		let key = block_key(target)?;
		if let Some(block) = self.blocks.get(&key) {
			return Some(Node::List(vec![block.clone()], Bracket::Round, Separator::None));
		}
		if let Some(data) = self.data.get(&key) {
			let written = crate::normalize::operand_text(data);
			let question = Ask::new(
				DATA_RUN_TOPIC,
				format!("running data as code: {key} is data ({written})"),
				vec![reading("run it", &format!("{key}!")), reading("a block", &format!("{key} = code {written}"))],
				Fallback::Warning,
			)
			.written(&format!("{key}!"));
			if let Err(error) = ask(&question) {
				return Some(error);
			}
			return Some(Node::List(vec![data.clone()], Bracket::Round, Separator::None));
		}
		let entries = self.objects.get(&key)?;
		let calls = entries.iter().map(|(function, argument)| applied(function, argument.clone())).collect();
		Some(Node::List(calls, Bracket::Round, Separator::Semicolon))
	}

	/// `f(a, b)` of a function with a `block` parameter: its body with the block arguments' code and the other parameters bound
	fn expand(&mut self, items: &[Node]) -> Option<Node> {
		let (name, arguments) = word_headed(items)?;
		let function = self.functions.get(name)?.clone();
		let arguments = fitted_arguments(&function.parameters, arguments)?;
		if self.expansions > MAX_EXPANSIONS {
			return None;
		}
		let number = self.expansions;
		self.expansions += 1;
		let mut statements = vec![];
		let mut body = function.body;
		for ((parameter, takes), argument) in function.parameters.iter().zip(&arguments) {
			let callable = *takes == Takes::Callable;
			if let Some(kind) =
				crate::analyzer::argument_literal_kind(argument).filter(|kind| callable && *kind != Kind::Function)
			{
				return Some(not_callable(name, parameter, argument, kind));
			}
			if *takes == Takes::Block || callable && is_code_argument(argument) {
				body = substitute_block(body, parameter, argument);
			} else if callable && argument.drop_meta().symbol_name().is_some() {
				// a function's name stands for the function: `f(3)` is `square(3)`
				body = crate::library_words::substitute(body, parameter, argument);
			} else {
				let bound = Node::Symbol(format!("{parameter}{EXPANSION_SEPARATOR}{number}"));
				statements.push(assign(bound.clone(), self.rewrite(argument.clone())));
				body = crate::library_words::substitute(body, parameter, &bound);
			}
		}
		statements.push(self.rewrite(body));
		Some(Node::List(statements, Bracket::Round, Separator::Semicolon))
	}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Some((target, _)) = bang_target(&node) {
			return self.run(&target).unwrap_or(node);
		}
		// `f(a, b)` and `f a {…}`
		if let Node::List(items, Bracket::Round, Separator::None) | Node::List(items, Bracket::None, Separator::Space) =
			node.drop_meta()
		{
			if let Some(expanded) = self.expand(items) {
				return expanded;
			}
		}
		if let Some(call) = self.method_call(&node) {
			return call;
		}
		if let Some(prefixed) = prefixed_group(&node) {
			return self.rewrite(prefixed);
		}
		// `data (j: 1⏎ k: n * 1)` is code as written: its `k:` binds no block (tree-patterns step 7)
		if quoted_data(&node).is_some() {
			return node;
		}
		// `o.s1!`: the parser marks the field it was reading (as for `x.upper!`)
		if let Node::Key(object, Op::Dot, field) = node.drop_meta() {
			if let Some((field, _)) = bang_target(field) {
				if let Some(run) = self.run(&Node::Key(object.clone(), Op::Dot, Box::new(field))) {
					return run;
				}
			}
		}
		// an element's block holds attributes and children, not statements: `li{ key: todo.id⏎ todo.text }` names its item
		if crate::element_events::element_items(&node).is_some() {
			return crate::element_events::with_element_items(node, |items| {
				items.into_iter().map(|item| self.rewrite(item)).collect()
			});
		}
		match node {
			Node::Symbol(name) if self.blocks.contains_key(&name) => as_data(self.blocks[&name].clone()),
			// `x = …` ends the block named x
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let value = self.rewrite(*value);
				if let Node::Symbol(name) = target.drop_meta() {
					self.forget(name);
				}
				Node::Key(target, op, Box::new(value))
			}
			// a block where a value is needed
			Node::Key(left, op, right) if op.is_arithmetic() || op.is_comparison() => {
				for operand in [&left, &right] {
					if let Some(block) =
						block_key(operand).and_then(|key| self.blocks.get(&key).map(|block| (key, block)))
					{
						return type_error(&block.0, block.1);
					}
				}
				key(self.rewrite(*left), op, self.rewrite(*right))
			}
			Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
				Node::List(items.into_iter().map(|item| self.statement(item)).collect(), bracket, separator)
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}
}

/// A block after a call that leaves a parameter of the function without its value is that argument (Kotlin's and
/// Swift's trailing closure, Ruby's block): `Card("Hi") { p:"text" }`, `apply(3) { |x| x*2 }`, `twice(3) do |x| … end`.
/// Ruby's `yield a, b` in a function is `block(a, b)` of its extra last parameter `block` (card functions-ruby-yield).
pub fn lower_trailing_blocks(program: Node) -> Node {
	let program = with_yield_parameters(program);
	let mut parameter_counts = HashMap::new();
	program.visit(&mut |node| parameter_counts.extend(parameter_count(node)));
	match parameter_counts.is_empty() {
		true => program,
		false => with_trailing_blocks(program, &parameter_counts),
	}
}

const YIELD_WORD: &str = "yield";

pub(crate) fn contains_yield(body: &Node) -> bool {
	any_part(body, |part| part.is_symbol(YIELD_WORD))
}

fn with_yield_parameters(node: Node) -> Node {
	if let Node::Key(head, Op::Define, body) = node.drop_meta() {
		if let Node::List(items, Bracket::Round, separator) = head.drop_meta() {
			if contains_yield(body) && items.first().is_some_and(|name| name.symbol_name().is_some()) {
				let head =
					Node::List([&items[..], &[symbol(BLOCK_WORDS[1])]].concat(), Bracket::Round, separator.clone());
				return key(head, Op::Define, yields_as_calls(body.as_ref().clone()));
			}
		}
	}
	node.map_children(with_yield_parameters)
}

/// `yield`, `yield v` or `yield a, b`: the values yielded
fn yielded(node: &Node) -> Option<Vec<Node>> {
	match node.drop_meta() {
		_ if node.is_symbol(YIELD_WORD) => Some(vec![]),
		Node::List(items, _, Separator::Space | Separator::None) if items.first()?.is_symbol(YIELD_WORD) => {
			Some(items[1..].to_vec())
		}
		Node::List(items, _, Separator::Comma) => Some([yielded(items.first()?)?, items[1..].to_vec()].concat()),
		_ => None,
	}
}

fn yields_as_calls(node: Node) -> Node {
	match yielded(&node) {
		Some(values) => call(BLOCK_WORDS[1], values.into_iter().map(yields_as_calls).collect()),
		None => node.map_children(yields_as_calls),
	}
}

/// `f(a, b) := body`: the name and the number of parameters
fn parameter_count(node: &Node) -> Option<(String, usize)> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else {
		return None;
	};
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	let name = items.first()?.drop_meta().symbol_name()?;
	(items.len() > 1).then(|| (name.to_string(), items.len() - 1))
}

fn with_trailing_blocks(node: Node, parameter_counts: &HashMap<String, usize>) -> Node {
	let node = trailing_block_call(&node, parameter_counts).unwrap_or(node);
	node.map_children(|child| with_trailing_blocks(child, parameter_counts))
}

/// `f {…}`, `f(a) {…}`, `f(a) do … end`, `f a do … end`: the call with the block as its last argument
fn trailing_block_call(node: &Node, parameter_counts: &HashMap<String, usize>) -> Option<Node> {
	let (callee, more_arguments, block) = match node.drop_meta() {
		Node::List(items, _, Separator::Space | Separator::None) if items.len() == 2 && is_block(&items[1]) => {
			(&items[0], vec![], &items[1])
		}
		Node::Key(callee, Op::Do, block) if is_block(block) => (callee.as_ref(), vec![], block.as_ref()),
		Node::List(items, _, Separator::Space) if items.len() == 2 => match items[1].drop_meta() {
			Node::Key(argument, Op::Do, block) if is_block(block) => {
				(&items[0], grouped_arguments(argument), block.as_ref())
			}
			_ => return None,
		},
		_ => return None,
	};
	let (name, mut arguments) = call_parts(callee, parameter_counts)?;
	arguments.extend(more_arguments);
	(arguments.len() < parameter_counts[&name]).then(|| {
		arguments.push(block.clone());
		call(&name, arguments)
	})
}

/// `f`, `f(a)` or `f a`: the function and its arguments
fn call_parts(node: &Node, parameter_counts: &HashMap<String, usize>) -> Option<(String, Vec<Node>)> {
	let (name, arguments) = match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) => items.split_first()?,
		single => (single, &[][..]),
	};
	let name = name.drop_meta().symbol_name().filter(|name| parameter_counts.contains_key(*name))?;
	Some((name.to_string(), arguments.iter().flat_map(grouped_arguments).collect()))
}

/// `(3)` and `(a, b)` hold the arguments
fn grouped_arguments(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::Comma) => items.clone(),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items.clone(),
		_ => vec![node.clone()],
	}
}
