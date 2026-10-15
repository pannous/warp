//! Named patterns (card patterns-values, tree-patterns step 4, user decision: the keyword): `pattern Pair = [_, _]`
//! names a pattern. The definition leaves the program and the name stands for its pattern where a pattern goes:
//! `x is Pair`, an arm `Pair => …` or `Pair: …`, and `x is Pair and Short`, which tests x against both. A named pattern
//! binds its names like the written one (`pattern Point = [x, y]; match p { Point => x + y }`). A plain assignment is
//! always a value; one whose right side can only be a pattern (`Pair = [_, _]`) is the error whose fix is the keyword.

use super::nodes::is_kinded_type_word;
use super::nodes::{children_rewritten, key};
use super::switch::{is_list_pattern, is_map_pattern, is_switch_head, map_field, rest_pattern, split_switch, WILDCARD};
use super::tree_walks::{FIND_WORD, REPLACE_WORD};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

const PATTERN_WORD: &str = "pattern";

type Patterns = HashMap<String, Node>;

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else {
		return program;
	};
	if let Some(error) = statements.iter().find_map(pattern_without_keyword) {
		return error;
	}
	let (definitions, statements): (Vec<Node>, Vec<Node>) =
		statements.into_iter().partition(|statement| pattern_definition(statement).is_some());
	if definitions.is_empty() {
		return Node::List(statements, bracket, separator);
	}
	let patterns: Patterns = definitions.iter().filter_map(pattern_definition).collect();
	let statements = statements.into_iter().map(|statement| substituted(statement, &patterns)).collect();
	Node::List(statements, bracket, separator)
}

/// `pattern Pair = [_, _]`: the name and its pattern
fn pattern_definition(statement: &Node) -> Option<(String, Node)> {
	let Node::List(parts, _, _) = statement.drop_meta() else {
		return None;
	};
	let [keyword, definition] = parts.as_slice() else {
		return None;
	};
	match definition.drop_meta() {
		Node::Key(name, Op::Assign, pattern) if keyword.is_symbol(PATTERN_WORD) => {
			Some((name.symbol_name()?.to_string(), pattern.as_ref().clone()))
		}
		_ => None,
	}
}

/// `Pair = [_, _]` assigns what only a pattern can be: the error whose fix is `pattern Pair = [_, _]`
fn pattern_without_keyword(statement: &Node) -> Option<Node> {
	match statement.drop_meta() {
		Node::Key(name, Op::Assign, pattern) if name.symbol_name().is_some() && is_open(pattern) => {
			let name = name.name();
			Some(
				Diagnostic::at(
					statement,
					format!("{name} is assigned a pattern, which is no value: name it with the keyword pattern"),
				)
				.fix(format!("{PATTERN_WORD} {name} = …"))
				.into_error(),
			)
		}
		_ => None,
	}
}

/// A list or map literal no value could be: among its items (at any depth of nested list and map literals) a `_`, an
/// unnamed rest `..` or an item pattern. A `_` elsewhere stays what it is, an argument placeholder in `add(1, _)`.
fn is_open(pattern: &Node) -> bool {
	(is_list_pattern(pattern) || is_map_pattern(pattern)) && literal_items(pattern).iter().any(is_open_part)
}

fn is_open_part(part: &Node) -> bool {
	// a named rest `...xs` may be a spread of the list xs
	rest_pattern(part) == Some(None)
		|| part.is_symbol(WILDCARD)
		|| matches!(part.drop_meta(), Node::Key(each, Op::Range | Op::To, end)
			if end.is_empty_node() && matches!(each.drop_meta(), Node::List(..)))
		|| matches!(part.drop_meta(), Node::Key(_, Op::Colon, value) if is_open_part(value))
		|| is_open(part)
}

fn literal_items(literal: &Node) -> &[Node] {
	match literal.drop_meta() {
		Node::List(items, _, _) => items,
		_ => &[],
	}
}

fn substituted(node: Node, patterns: &Patterns) -> Node {
	let node = children_rewritten(node, |child| substituted(child, patterns));
	match node {
		Node::Key(subject, Op::Eq, pattern) => Node::Key(subject, Op::Eq, Box::new(resolved(*pattern, patterns))),
		Node::Key(subject, Op::As, target) => match pattern_named(&target, patterns).and_then(schema_fields) {
			Some(fields) => cast_to_schema(*subject, &target.name(), &fields),
			None => Node::Key(subject, Op::As, target),
		},
		// `x is Pair and Short`: Short tests the same subject
		Node::Key(tests, op @ (Op::And | Op::Or), pattern) => {
			match (tested_subject(&tests), pattern_named(&pattern, patterns)) {
				(Some(subject), Some(pattern)) => {
					let test = key(subject.clone(), Op::Eq, pattern.clone());
					Node::Key(tests, op, Box::new(test))
				}
				_ => Node::Key(tests, op, pattern),
			}
		}
		Node::List(items, bracket, separator) => match split_switch(&items) {
			Some((head, _, _)) if is_switch_head(head) => {
				let items = items.into_iter().map(|item| with_case_keys(item, patterns)).collect();
				Node::List(items, bracket, separator)
			}
			_ => Node::List(with_found_patterns(items, patterns), bracket, separator),
		},
		other => other,
	}
}

/// `page find Image`, `xs replace Pair with …`: the pattern after the word find or replace
fn with_found_patterns(items: Vec<Node>, patterns: &Patterns) -> Vec<Node> {
	let mut after_walk = false;
	items
		.into_iter()
		.map(|item| {
			let item = if after_walk { resolved(item, patterns) } else { item };
			after_walk = ends_with_walk_word(&item);
			item
		})
		.collect()
}

/// `find`, or the nested words `[xs, replace]` of `xs replace Pair with …` in a function body
fn ends_with_walk_word(item: &Node) -> bool {
	match item.drop_meta() {
		Node::List(words, Bracket::None, _) => words.last().is_some_and(ends_with_walk_word),
		word => word.is_symbol(FIND_WORD) || word.is_symbol(REPLACE_WORD),
	}
}

/// The subject of the first test of `x is A and B`
fn tested_subject(tests: &Node) -> Option<&Node> {
	match tests.drop_meta() {
		Node::Key(subject, Op::Eq, _) => Some(subject),
		Node::Key(first, Op::And | Op::Or, _) => tested_subject(first),
		_ => None,
	}
}

fn pattern_named<'a>(node: &Node, patterns: &'a Patterns) -> Option<&'a Node> {
	patterns.get(node.drop_meta().symbol_name()?)
}

fn resolved(node: Node, patterns: &Patterns) -> Node {
	pattern_named(&node, patterns).cloned().unwrap_or(node)
}

/// A field of a schema: its name, its type word and whether it may be missing (`debug?: bool`)
struct SchemaField {
	name: String,
	type_word: String,
	optional: bool,
}

/// The fields of a map pattern whose every field is a type: a schema, which `as` converts to
fn schema_fields(pattern: &Node) -> Option<Vec<SchemaField>> {
	if !is_map_pattern(pattern) {
		return None;
	}
	literal_items(pattern)
		.iter()
		.map(|part| {
			let (name, type_word) = map_field(part)?;
			is_kinded_type_word(type_word).then(|| SchemaField {
				name,
				type_word: type_word.name(),
				optional: crate::type_kinds::optional_field(part).is_some(),
			})
		})
		.collect()
}

/// `x as Config`: a copy of x with each field converted by `as` where it does not have its type yet, else the error
/// that names the field; a missing optional field stays missing (card loaded-config)
fn cast_to_schema(subject: Node, schema: &str, fields: &[SchemaField]) -> Node {
	let copy = format!("as_{schema}");
	let converted: String = fields
		.iter()
		.map(|SchemaField { name, type_word, optional }| {
			let field = format!("{copy}.{name}");
			let article = if type_word.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
			let shown = format!(r#"(if {field} is text then "\"" + {field} + "\"" else "\({field})")"#);
			let failure = format!(r#""{name} of {schema} takes {article} {type_word}, got " + {shown}"#);
			let conversion = format!(
				"if not ({field} is {type_word}) {{ {field} = try {{ {field} as {type_word} }} catch problem {{ raise {failure} }} }}"
			);
			match optional {
				true => format!("if ({copy} has {name}) {{ {conversion} }}\n"),
				false => format!(
					"if not ({copy} has {name}) {{ raise \"{name} of {schema} is missing\" }}\n{conversion}\n"
				),
			}
		})
		.collect();
	let copied = key(Node::Symbol(copy.clone()), Op::Assign, crate::lowering::nodes::call("copy", vec![subject]));
	let mut statements = vec![copied];
	statements.extend(crate::warp_parser::parse_generated(&format!("{converted}{copy}")).as_items());
	Node::List(statements, Bracket::Round, crate::node::Separator::Semicolon)
}

/// The colon arms `Pair: …` of a switch's case block, which as an item may also hold its subject
fn with_case_keys(item: Node, patterns: &Patterns) -> Node {
	match item {
		Node::Key(arm, Op::Colon, body) => Node::Key(Box::new(resolved(*arm, patterns)), Op::Colon, body),
		Node::List(items, bracket, separator) => {
			Node::List(items.into_iter().map(|item| with_case_keys(item, patterns)).collect(), bracket, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_case_keys(node, patterns)),
		other => other,
	}
}
