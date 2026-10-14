//! Tests in a program (card test-soft, P209, P210): `test C` is a soft check that records whether C holds and goes on;
//! `test "name" { … }` is a named test whose failure (a failed `check`, any error) is caught per block. They run only
//! under `warp test` (pipeline::for_tests); a plain run skips them. Under test, each failure prints a ✗ line, and the
//! program's value is "✓ 12 tests passed", or the Error "2 of 12 failed". Page tests (`test "…" { render … }`) run
//! in page_tests.rs. `expect 30 10+20` is the soft check `test 10+20 == 30` (card expect-make).

use crate::lowering::library_words::substitute;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;

const TEST_WORD: &str = "test";
const EXPECT_WORD: &str = "expect";
const COUNT: (&str, &str) = ("tests_run_", "tests·run");
const FAILED: (&str, &str) = ("tests_failed_", "tests·failed");
const CAUGHT: (&str, &str) = ("test_error_", "test·error");
const CONDITION: &str = "condition_";
const BODY: &str = "body_";
const NAME: &str = "name_";
const PROLOGUE: &str = "tests_run_ = 0; tests_failed_ = 0";
const SOFT_CHECK: &str = r#"tests_run_ += 1; if not (condition_) then { tests_failed_ += 1; print("✗ " + name_) }"#;
const NAMED_TEST: &str = r#"tests_run_ += 1; try { body_; 0 } catch test_error_ { tests_failed_ += 1; print("✗ " + name_ + ": " + str(test_error_)) }"#;
const SUMMARY: &str = r#"if tests_failed_ > 0 then raise(str(tests_failed_) + " of " + str(tests_run_) + " failed"); "✓ " + str(tests_run_) + (if tests_run_ == 1 then " test" else " tests") + " passed""#;

pub fn lower(program: Node) -> Node {
	// soft keywords: a program naming test or expect itself keeps that word
	let words: Vec<&str> = [TEST_WORD, EXPECT_WORD]
		.into_iter()
		.filter(|word| !crate::soft_keywords::program_names(&program, word))
		.collect();
	let written = program.body_statements();
	if !written.iter().any(|statement| test_of(statement, &words).is_some()) {
		return program;
	}
	match crate::pipeline::compile_mode().for_tests {
		true => sequence(
			[
				template(PROLOGUE).body_statements(),
				written.into_iter().flat_map(|statement| run_test(statement, &words)).collect(),
				template(SUMMARY).body_statements(),
			]
			.concat(),
		),
		false => sequence(written.into_iter().filter(|statement| test_of(statement, &words).is_none()).collect()),
	}
}

enum Test<'a> {
	Soft(Node),
	Named(&'a str, &'a Node),
}

/// `test C` or `test "name" { … }`; C may be a phrase of several parts: `test switch 3 {…} == "three"`;
/// `expect V C` is `test C == V`
fn test_of<'a>(statement: &'a Node, words: &[&str]) -> Option<Test<'a>> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let (head, rest) = items.split_first()?;
	if !words.iter().any(|word| head.is_symbol(word)) {
		return None;
	}
	if head.is_symbol(EXPECT_WORD) {
		let (expected, actual) = rest.split_first()?;
		return Some(Test::Soft(Node::Key(Box::new(phrase(actual)?), Op::Eq, Box::new(expected.clone()))));
	}
	if let [name, body] = rest {
		if let (Node::Text(name), Node::List(_, Bracket::Curly, _)) = (name.drop_meta(), body.drop_meta()) {
			return Some(Test::Named(name, body));
		}
	}
	phrase(rest).map(Test::Soft)
}

/// The parts of a phrase as one node: the part itself, or a phrase of several
fn phrase(parts: &[Node]) -> Option<Node> {
	match parts {
		[] => None,
		[part] => Some(part.clone()),
		parts => Some(Node::List(parts.to_vec(), Bracket::None, Separator::Space)),
	}
}

/// The statements a test runs as: it counts, and a failure prints its ✗ line
fn run_test(statement: Node, words: &[&str]) -> Vec<Node> {
	let lowered = match test_of(&statement, words) {
		Some(Test::Soft(condition)) => {
			let serialized = || format!("{TEST_WORD} {}", condition.serialize().trim());
			let written =
				Node::Text(crate::errors::diagnostic::written_statement(&statement).unwrap_or_else(serialized));
			substitute(substitute(template(SOFT_CHECK), CONDITION, &condition), NAME, &written)
		}
		Some(Test::Named(name, body)) => {
			let body = Node::List(
				body.body_statements().into_iter().flat_map(|statement| run_test(statement, words)).collect(),
				Bracket::Curly,
				Separator::Newline,
			);
			substitute(substitute(template(NAMED_TEST), BODY, &body), NAME, &Node::Text(format!("\"{name}\"")))
		}
		None => return vec![statement],
	};
	lowered.body_statements()
}

/// A template's source with the hidden names, which a program cannot write
fn template(source: &str) -> Node {
	[COUNT, FAILED, CAUGHT].iter().fold(
		crate::compiler::meta::parsing_file(crate::compiler::meta::GENERATED_FILE, || parse(source)),
		|node, (placeholder, hidden)| substitute(node, placeholder, &symbol(hidden)),
	)
}

fn sequence(statements: Vec<Node>) -> Node {
	match statements.len() {
		0 => Node::Empty,
		_ => Node::List(statements, Bracket::None, Separator::Newline),
	}
}
