//! Tests in a program (card test-soft, P209, P210): `test C` is a soft check that records whether C holds and goes on;
//! `test "name" { … }` is a named test whose failure (a failed `check`, any error) is caught per block. They run only
//! under `warp test` (pipeline::for_tests); a plain run skips them. Under test, each failure prints a ✗ line, and the
//! program's value is "✓ 12 tests passed", or the Error "2 of 12 failed". Page tests (`test "…" { render … }`) run
//! in page_tests.rs. `expect 30 10+20` is the soft check `test 10+20 == 30` (card expect-make).

use crate::lowering::library_words::substitute;
use crate::lowering::nodes::key;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;

const TEST_WORD: &str = "test";
const EXPECT_WORD: &str = "expect";
/// Words whose top-level calls count as tests under `warp test` and print their own ✗ line (lib/prelude.warp `is!`)
const CHECK_WORDS: [&str; 1] = ["is!"];
/// The name of a map of test pairs: `test_pairs = { '1+1' 2 }`
const TEST_PAIRS: &str = "test_pairs";
const COUNT: (&str, &str) = ("tests_run_", "tests·run");
const FAILED: (&str, &str) = ("tests_failed_", "tests·failed");
const CAUGHT: (&str, &str) = ("test_error_", "test·error");
const CONDITION: &str = "condition_";
const BODY: &str = "body_";
const NAME: &str = "name_";
const PROLOGUE: &str = "tests_run_ = 0; tests_failed_ = 0";
const SOFT_CHECK: &str = r#"tests_run_ += 1; if not (condition_) then { tests_failed_ += 1; print("✗ " + name_) }"#;
const NAMED_TEST: &str = r#"tests_run_ += 1; try { body_; 0 } catch test_error_ { tests_failed_ += 1; print("✗ " + name_ + ": " + str(test_error_)) }"#;
const CHECK: &str = r#"tests_run_ += 1; try { if not (condition_) then tests_failed_ += 1 } catch test_error_ { tests_failed_ += 1; print("✗ " + name_ + ": " + str(test_error_)) }"#;
const SUMMARY: &str = r#"if tests_failed_ > 0 then raise(str(tests_failed_) + " of " + str(tests_run_) + " failed"); "✓ " + str(tests_run_) + (if tests_run_ == 1 then " test" else " tests") + " passed""#;

pub fn lower(program: Node) -> Node {
	let program = match crate::pipeline::compile_mode().for_tests {
		true => pairs_checked(program),
		false => program,
	};
	// soft keywords: a program taking test or expect itself keeps that word
	let names = crate::program_names::ProgramNames::of(&program);
	let words: Vec<&str> = [TEST_WORD, EXPECT_WORD].into_iter().filter(|word| !names.takes(word)).collect();
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
		false => sequence(written.into_iter().filter(|statement| !is_skipped_test(statement, &words)).collect()),
	}
}

enum Test<'a> {
	Soft(Node),
	/// `is! "3+3" 6`: a check word's call, run as it is also outside `warp test`
	Check(&'a Node),
	Named(&'a str, &'a Node),
}

/// `test C` or `test "name" { … }`; C may be a phrase of several parts: `test switch 3 {…} == "three"`;
/// `expect V C` is `test C == V`
fn test_of<'a>(statement: &'a Node, words: &[&str]) -> Option<Test<'a>> {
	if is_check(statement) {
		return Some(Test::Check(statement));
	}
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let (head, rest) = items.split_first()?;
	if !words.iter().any(|word| head.is_symbol(word)) {
		return None;
	}
	if head.is_symbol(EXPECT_WORD) {
		let (expected, actual) = rest.split_first()?;
		return Some(Test::Soft(key(phrase(actual)?, Op::Eq, expected.clone())));
	}
	if let [name, body] = rest {
		if let (Node::Text(name), Node::List(_, Bracket::Curly, _)) = (name.drop_meta(), body.drop_meta()) {
			return Some(Test::Named(name, body));
		}
	}
	phrase(rest).map(Test::Soft)
}

/// A file of test pairs (decision warp-is-tests): code and its expected value a line each, `'1+1' 2`, bare or as the
/// map `test_pairs = { … }`; each pair runs as `is!(code, expected)`. Any other program stays as it is
fn pairs_checked(program: Node) -> Node {
	let statements = program.body_statements();
	let named_pairs = match statements.as_slice() {
		[single] => match single.drop_meta() {
			Node::Key(name, Op::Assign, map) if name.is_symbol(TEST_PAIRS) => Some(map.body_statements()),
			_ => None,
		},
		_ => None,
	};
	// texts alone ("hello", the guide's first program) are a value, not a file of tests: one pair must expect something
	if named_pairs.is_none() && !statements.iter().any(|statement| pair_parts(statement).is_some()) {
		return program;
	}
	let pairs = named_pairs.unwrap_or(statements);
	let checks: Option<Vec<Node>> = pairs.iter().map(pair_check).collect();
	match checks {
		Some(checks) if !checks.is_empty() => sequence(checks),
		_ => program,
	}
}

/// `'1+1' 2` as `is!("1+1", 2)`; a code alone expects ø
fn pair_check(pair: &Node) -> Option<Node> {
	let (code, expected) = pair_parts(pair).unwrap_or((pair.drop_meta(), Node::Empty));
	let code = match code.drop_meta() {
		Node::Text(code) => code.clone(),
		Node::Char(code) => code.to_string(),
		_ => return None,
	};
	Some(Node::List(vec![symbol(CHECK_WORDS[0]), Node::Text(code), expected], Bracket::Round, Separator::None))
}

/// `'1+1' 2`: the code and the value it expects
fn pair_parts(pair: &Node) -> Option<(&Node, Node)> {
	match pair.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Square, Separator::Space) if items.len() == 2 => {
			Some((&items[0], items[1].clone()))
		}
		_ => None,
	}
}

/// `is! "3+3" 6`, `is!("3+3", 6)`: a call of a check word
fn is_check(statement: &Node) -> bool {
	let Node::List(items, Bracket::None | Bracket::Round, _) = statement.drop_meta() else {
		return false;
	};
	items.first().is_some_and(|head| CHECK_WORDS.iter().any(|word| head.is_symbol(word)))
}

/// A test a plain run skips: all but the checks, which are calls of their own
fn is_skipped_test(statement: &Node, words: &[&str]) -> bool {
	matches!(test_of(statement, words), Some(Test::Soft(_) | Test::Named(..)))
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
		// as serialized: the written statement ends at the first `;`, also one inside the code text
		Some(Test::Check(check)) => {
			let written = Node::Text(check.serialize().trim().to_string());
			substitute(substitute(template(CHECK), CONDITION, check), NAME, &written)
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
