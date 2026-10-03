//! Typed templates for other languages (Footguns.md "SQL and shell injection").
//!
//! `sql "SELECT * FROM t WHERE name = $name"` is not text building: the literal is the query,
//! each `$name` / `${expr}` hole becomes a parameter, so a value can never change the query's shape.
//! `sh "ls -l $dir"` is an argument vector run without a shell: a hole is one whole argument.
//! Building a template from computed text is a compile error, and running one (`execute`, `exec`)
//! needs the `sql` / `process` capability, which `eval` does not grant.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::WaspParser;
use std::collections::HashMap;
use std::fmt;

const SQL_PLACEHOLDER: &str = "?";
const SHELL_OPERATORS: [char; 9] = [';', '|', '&', '<', '>', '(', ')', '`', '\\'];
const QUOTES: [char; 2] = ['\'', '"'];

/// The language a template is written in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
	Sql,
	Shell,
}

impl Language {
	pub fn tagged(tag: &str) -> Option<Language> {
		match tag {
			"sql" => Some(Language::Sql),
			"sh" => Some(Language::Shell),
			_ => None,
		}
	}

	pub fn tag(self) -> &'static str {
		match self {
			Language::Sql => "sql",
			Language::Shell => "sh",
		}
	}

	/// The function that runs a template of this language
	pub fn runner(self) -> &'static str {
		match self {
			Language::Sql => "execute",
			Language::Shell => "exec",
		}
	}

	fn runs(runner: &str) -> Option<Language> {
		[Language::Sql, Language::Shell].into_iter().find(|language| language.runner() == runner)
	}
}

/// One piece of a template: literal source text or a hole holding an expression
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
	Literal(String),
	Hole(Node),
}

/// A parsed template: for SQL the query text with `?` per parameter, for shell the argument vector.
/// Parameters and hole arguments are expressions, evaluated as values, never spliced into the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
	pub language: Language,
	/// SQL: `[query text]`; shell: one entry per argument, `None` where a hole is the argument
	pub text: Vec<Option<String>>,
	pub holes: Vec<Node>,
}

impl Template {
	/// SQL query text with one `?` per parameter
	pub fn query(&self) -> Option<&str> {
		match self.language {
			Language::Sql => self.text.first().and_then(|text| text.as_deref()),
			Language::Shell => None,
		}
	}

	/// Runtime value: SQL `(query param…)`, shell `(program arg…)`
	pub fn to_node(&self) -> Node {
		let mut holes = self.holes.iter();
		let items = match self.language {
			Language::Sql => self.text.iter().flatten().map(|text| Node::Text(text.clone())).chain(holes.cloned()).collect(),
			Language::Shell => self.text.iter().map(|word| match word {
				Some(literal) => Node::Text(literal.clone()),
				None => holes.next().expect("one hole per hole argument").clone(),
			}).collect(),
		};
		Node::List(items, Bracket::Round, Separator::Space)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateError(pub String);

impl fmt::Display for TemplateError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Split a template literal into literal text and holes: `$name`, `${expr}`, `$$` for a literal `$`
pub fn parts(source: &str) -> Result<Vec<Part>, TemplateError> {
	parts_with(source, true)
}

/// As `parts`; without `bare_names` a `$name` is literal text (interpolated text holes need braces, decision D1)
pub fn parts_with(source: &str, bare_names: bool) -> Result<Vec<Part>, TemplateError> {
	let mut parts = vec![];
	let mut literal = String::new();
	let mut chars = source.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch != '$' {
			literal.push(ch);
			continue;
		}
		let hole = match chars.peek().copied() {
			Some('$') => {
				chars.next();
				literal.push('$');
				continue;
			}
			Some('{') => {
				chars.next();
				let mut depth = 1;
				let expression: String = chars.by_ref().take_while(|c| {
					depth += match *c { '{' => 1, '}' => -1, _ => 0 };
					depth > 0
				}).collect();
				if depth > 0 {
					return Err(TemplateError(format!("unterminated hole ${{{expression}")));
				}
				if expression.trim().is_empty() {
					return Err(TemplateError("empty hole ${}".into()));
				}
				WaspParser::parse(&expression)
			}
			Some(c) if bare_names && (c.is_alphabetic() || c == '_') => {
				let mut name = String::new();
				while let Some(c) = chars.peek().copied().filter(|c| c.is_alphanumeric() || *c == '_') {
					name.push(c);
					chars.next();
				}
				Node::Symbol(name)
			}
			_ => {
				literal.push('$');
				continue;
			}
		};
		if !literal.is_empty() {
			parts.push(Part::Literal(std::mem::take(&mut literal)));
		}
		parts.push(Part::Hole(hole));
	}
	if !literal.is_empty() {
		parts.push(Part::Literal(literal));
	}
	Ok(parts)
}

/// Compile a template literal of `language`
pub fn template(language: Language, source: &str) -> Result<Template, TemplateError> {
	let parts = parts(source)?;
	match language {
		Language::Sql => sql(parts),
		Language::Shell => shell(parts),
	}
}

/// Holes become `?` parameters; a hole inside a quoted SQL string or identifier would be a literal `?`
fn sql(parts: Vec<Part>) -> Result<Template, TemplateError> {
	let mut query = String::new();
	let mut holes = vec![];
	let mut open_quote: Option<char> = None;
	for part in parts {
		match part {
			Part::Literal(text) => {
				for c in text.chars() {
					open_quote = match open_quote {
						None if QUOTES.contains(&c) => Some(c),
						Some(quote) if quote == c => None,
						other => other,
					};
				}
				query.push_str(&text);
			}
			Part::Hole(hole) => {
				if let Some(quote) = open_quote {
					return Err(TemplateError(format!(
						"sql hole ${} inside {quote}…{quote}: a parameter is a value, not text to paste into a quoted string; remove the quotes around it",
						hole.serialize())));
				}
				query.push_str(SQL_PLACEHOLDER);
				holes.push(hole);
			}
		}
	}
	if let Some(quote) = open_quote {
		return Err(TemplateError(format!("unterminated {quote} in sql template")));
	}
	Ok(Template { language: Language::Sql, text: vec![Some(query)], holes })
}

/// Words split at whitespace; a hole is one whole argument, never re-split, globbed or parsed by a shell
fn shell(parts: Vec<Part>) -> Result<Template, TemplateError> {
	let mut words: Vec<Option<String>> = vec![];
	let mut holes = vec![];
	let mut word_open = false; // the last word may still grow
	for part in parts {
		match part {
			Part::Literal(text) => {
				if let Some(c) = text.chars().find(|c| SHELL_OPERATORS.contains(c) || QUOTES.contains(c)) {
					return Err(TemplateError(format!(
						"sh runs one program with arguments and no shell: `{c}` would be passed literally; use one hole per argument (`${{x}}`) instead of quoting, and separate commands"
					)));
				}
				let starts_word = text.starts_with(|c: char| !c.is_whitespace());
				if word_open && starts_word && matches!(words.last(), Some(None)) {
					return Err(hole_in_word());
				}
				let mut pieces = text.split_whitespace();
				if word_open && starts_word {
					let first = pieces.next().expect("text starts with a non-space");
					if let Some(Some(word)) = words.last_mut() {
						word.push_str(first);
					}
				}
				words.extend(pieces.map(|word| Some(word.to_string())));
				word_open = !text.ends_with(char::is_whitespace);
			}
			Part::Hole(hole) => {
				if word_open && !words.is_empty() {
					return Err(hole_in_word());
				}
				words.push(None);
				holes.push(hole);
				word_open = true;
			}
		}
	}
	match words.first() {
		None => Err(TemplateError("empty sh command".into())),
		Some(None) => Err(TemplateError("the program of a sh command must be literal, not a hole".into())),
		Some(Some(_)) => Ok(Template { language: Language::Shell, text: words, holes }),
	}
}

fn hole_in_word() -> TemplateError {
	TemplateError("a sh hole must be a whole argument: `--out=$f` would glue text to a value; pass `--out ${f}` or build the argument as a value".into())
}

/// Replace every `sql "…"` / `sh "…"` by its typed value and check what reaches `execute` / `exec`.
/// The first misuse comes back as a diagnostic.
pub fn lower_templates(program: Node) -> Result<Node, Node> {
	Lowering::default().node(program).map_err(Diagnostic::into_error)
}

#[derive(Default)]
struct Lowering {
	/// Variables holding a template, by the template's language
	bound: HashMap<String, Language>,
}

impl Lowering {
	fn node(&mut self, node: Node) -> Result<Node, Diagnostic> {
		Ok(match node {
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node)?), data },
			Node::List(mut items, bracket, separator) => {
				// braceless application nests to the left: `execute sql "…"` → ((execute sql) "…")
				if let Some(Node::List(inner, Bracket::None, _)) = items.first().map(Node::drop_meta) {
					if items.len() > 1 && inner.last().and_then(tag_of).is_some() {
						let mut flat = inner.clone();
						flat.extend(items.drain(1..));
						items = flat;
					}
				}
				if let [tag, argument] = items.as_slice() {
					if let Some(language) = tag_of(tag) {
						return lowered_template(language, tag, argument);
					}
				}
				Node::List(self.items(items)?, bracket, separator)
			}
			Node::Key(left, op @ (Op::Assign | Op::Define), right) => {
				let language = language_of(&right);
				let right = self.node(*right)?;
				match (left.drop_meta(), language) {
					(Node::Symbol(name), Some(language)) => self.bound.insert(name.clone(), language),
					(Node::Symbol(name), None) => self.bound.remove(name),
					_ => None,
				};
				Node::Key(left, op, Box::new(right))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.node(*left)?), op, Box::new(self.node(*right)?)),
			Node::Error(inner) => Node::Error(Box::new(self.node(*inner)?)),
			other => other,
		})
	}

	/// Items of a flat statement like `execute sql "…"`: a tag takes the item after it,
	/// a runner is checked against the item after it (in order, so earlier assignments count)
	fn items(&mut self, items: Vec<Node>) -> Result<Vec<Node>, Diagnostic> {
		let mut lowered: Vec<Node> = vec![];
		let mut index = 0;
		while index < items.len() {
			let item = &items[index];
			if let Some(language) = runner_of(item) {
				self.check_run(language, item, items.get(index + 1))?;
			}
			match tag_of(item) {
				Some(language) => {
					let argument = items.get(index + 1).cloned().unwrap_or(Node::Empty);
					lowered.push(lowered_template(language, item, &argument)?);
					index += 2;
				}
				None => {
					lowered.push(self.node(item.clone())?);
					index += 1;
				}
			}
		}
		Ok(lowered)
	}

	/// `execute q` runs only a sql template, `exec c` only a sh template
	fn check_run(&self, language: Language, runner: &Node, argument: Option<&Node>) -> Result<(), Diagnostic> {
		let given = argument.and_then(|argument| language_of(argument).or_else(|| match argument.drop_meta() {
			Node::Symbol(name) => self.bound.get(name).copied(),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => match items[0].drop_meta() {
				Node::Symbol(name) => self.bound.get(name).copied(),
				_ => language_of(&items[0]),
			},
			_ => None,
		}));
		if given == Some(language) {
			return Ok(());
		}
		let got = argument.map_or("nothing".to_string(), |argument| argument.serialize());
		Err(Diagnostic::at(runner, format!(
			"{} takes a {} template, got {got}: text is never run as {}",
			language.runner(), language.tag(), language.tag())).fix(format!("{} {} \"… $value …\"", language.runner(), language.tag())))
	}
}

fn lowered_template(language: Language, tag: &Node, argument: &Node) -> Result<Node, Diagnostic> {
	let source = literal_text(argument).ok_or_else(|| not_literal(language, tag, argument))?;
	let template = template(language, &source).map_err(|error| Diagnostic::at(tag, error.0))?;
	Ok(template.to_node())
}

fn tag_of(node: &Node) -> Option<Language> {
	match node.drop_meta() {
		Node::Symbol(tag) => Language::tagged(tag),
		_ => None,
	}
}

fn runner_of(node: &Node) -> Option<Language> {
	match node.drop_meta() {
		Node::Symbol(name) => Language::runs(name),
		_ => None,
	}
}

/// The language of a template expression before lowering: `sql "…"`, or the bare tag of a flat `execute sql "…"`
fn language_of(node: &Node) -> Option<Language> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => tag_of(&items[0]),
		other => tag_of(other),
	}
}

/// `"…"` or `("…")`; anything computed is refused
fn literal_text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Text(text) => Some(text.clone()),
		Node::Char(c) => Some(c.to_string()),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => literal_text(&items[0]),
		_ => None,
	}
}

fn not_literal(language: Language, tag: &Node, argument: &Node) -> Diagnostic {
	let tag_name = language.tag();
	let got = if *argument == Node::Empty { "nothing".to_string() } else { argument.serialize() };
	Diagnostic::at(tag, format!(
		"{tag_name} takes a literal template, got {got}: values go into holes as parameters, never into the {tag_name} text"))
		.fix(format!("{tag_name} \"… $value …\""))
}
