//! Tests in a program (card test-soft, P209, P210): `test C` is a soft check that records whether C holds and goes on;
//! `test "name" { … }` is a named test whose failure (a failed `check`, any error) is caught per block. They run only
//! under `warp test` (pipeline::for_tests); a plain run skips them. Under test, each failure prints a ✗ line, and the
//! program's value is "✓ 12 tests passed", or the Error "2 of 12 failed". Page tests (`test "…" { render … }`) run
//! in page_tests.rs.

use crate::lowering::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::wasp_parser::parse;

const TEST_WORD: &str = "test";
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
	let written = statements(&program);
	if !written.iter().any(|statement| test_of(statement).is_some()) || crate::soft_keywords::program_names(&program, TEST_WORD) {
		return program;
	}
	match crate::pipeline::is_for_tests() {
		true => sequence([statements(&template(PROLOGUE)), written.into_iter().flat_map(run_test).collect(), statements(&template(SUMMARY))].concat()),
		false => sequence(written.into_iter().filter(|statement| test_of(statement).is_none()).collect()),
	}
}

enum Test<'a> {
	Soft(Node),
	Named(&'a str, &'a Node),
}

/// `test C` or `test "name" { … }`; C may be a phrase of several parts: `test switch 3 {…} == "three"`
fn test_of(statement: &Node) -> Option<Test<'_>> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else { return None };
	let (head, rest) = items.split_first()?;
	if !matches!(head.drop_meta(), Node::Symbol(word) if word == TEST_WORD) {
		return None;
	}
	if let [name, body] = rest {
		if let (Node::Text(name), Node::List(_, Bracket::Curly, _)) = (name.drop_meta(), body.drop_meta()) {
			return Some(Test::Named(name, body));
		}
	}
	match rest {
		[] => None,
		[condition] => Some(Test::Soft(condition.clone())),
		phrase => Some(Test::Soft(Node::List(phrase.to_vec(), Bracket::None, Separator::Space))),
	}
}

/// The statements a test runs as: it counts, and a failure prints its ✗ line
fn run_test(statement: Node) -> Vec<Node> {
	let lowered = match test_of(&statement) {
		Some(Test::Soft(condition)) => {
			let serialized = || format!("{TEST_WORD} {}", condition.serialize().trim());
			let written = Node::Text(crate::diagnostic::written_statement(&statement).unwrap_or_else(serialized));
			substitute(substitute(template(SOFT_CHECK), CONDITION, &condition), NAME, &written)
		}
		Some(Test::Named(name, body)) => {
			let body = Node::List(statements(body).into_iter().flat_map(run_test).collect(), Bracket::Curly, Separator::Newline);
			substitute(substitute(template(NAMED_TEST), BODY, &body), NAME, &Node::Text(format!("\"{name}\"")))
		}
		None => return vec![statement],
	};
	statements(&lowered)
}

/// A template's source with the hidden names, which a program cannot write
fn template(source: &str) -> Node {
	[COUNT, FAILED, CAUGHT].iter().fold(parse(source), |node, (placeholder, hidden)| substitute(node, placeholder, &Node::Symbol(hidden.to_string())))
}

fn statements(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Curly, Separator::Newline | Separator::Semicolon) => items.clone(),
		Node::List(items, Bracket::Curly, _) => items.clone(),
		_ => vec![node.clone()],
	}
}

fn sequence(statements: Vec<Node>) -> Node {
	match statements.len() {
		0 => Node::Empty,
		_ => Node::List(statements, Bracket::None, Separator::Newline),
	}
}
