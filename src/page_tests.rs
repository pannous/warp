//! Page tests written in warp (card web-testing, notes/web_framework.md step 14), run on headless pages (headless.rs):
//!
//! ```warp
//! def Counter() { count = 0; div{ button{ on click { count += 1 } "Add" } p{ count } } }
//! test "counter" {
//!     render Counter()
//!     click "Add"
//!     check text is "Add1"
//! }
//! ```
//!
//! A program with a top-level `test "…" { … }` block that renders is run by its tests: the code outside the tests is
//! each test's setup, `render markup` shows that markup as the page, `click "label"` and `fill "label" with "text"` act
//! on it, `check condition` must hold with `text` and `html` naming what the page shows, and any other line is setup
//! of that test. They run under `warp test` and the playground's Run (pipeline::for_tests); a plain run skips them (P209,
//! lowering/test_blocks.rs). The value is "✓ n tests passed", or an Error with a ✗ line for each failed test and why,
//! then "m of n failed" (P210). Everything is taken from the source as written (Node::serialize would not give back
//! every program).

use crate::extensions::numbers::Number;
use crate::headless::Page;
use crate::node::{error, Node};
use std::ops::Range;

const TEST_WORD: &str = "test";
const RENDER_WORD: &str = "render";
const CLICK_WORD: &str = "click";
const FILL_WORD: &str = "fill";
const WITH_WORD: &str = "with";
const CHECK_WORD: &str = "check";
const TEXT_WORD: &str = "text";
const HTML_WORD: &str = "html";
const STEP_WORDS: [&str; 4] = [RENDER_WORD, CLICK_WORD, FILL_WORD, CHECK_WORD];

/// The outcome of the program's page tests, when it has any
pub fn answer(code: &str) -> Option<Node> {
	if !crate::pipeline::is_for_tests() || !code.contains(TEST_WORD) || !code.contains(RENDER_WORD) {
		return None;
	}
	let ranges = test_ranges(code);
	let tests: Vec<(String, Vec<&str>)> = ranges.iter().filter_map(|range| test_parts(&code[range.clone()])).collect();
	if !tests.iter().any(|(_, steps)| steps.iter().any(|step| step_word(step) == Some(RENDER_WORD))) {
		return None;
	}
	let mut setup = code.to_string();
	ranges.iter().rev().for_each(|range| setup.replace_range(range.clone(), ""));
	let failures: Vec<String> = tests.iter().filter_map(|(name, steps)| run_test(&setup, steps).err().map(|problem| format!("✗ test \"{name}\": {problem}"))).collect();
	Some(match failures.is_empty() {
		true => Node::Text(format!("✓ {} passed", plural(tests.len(), TEST_WORD))),
		false => error(&format!("{}\n{} of {} failed", failures.join("\n"), failures.len(), tests.len())),
	})
}

fn plural(count: usize, word: &str) -> String {
	format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

/// Where the top-level `test "…" { … }` blocks are: from a line starting with `test "` to its block's closing brace
fn test_ranges(code: &str) -> Vec<Range<usize>> {
	let opening = format!("{TEST_WORD} \"");
	let starts = code.match_indices(&opening).map(|(at, _)| at).filter(|&at| at == 0 || code[..at].ends_with('\n'));
	starts.filter_map(|start| {
		let open = start + code[start..].find('{')?;
		Some(start..open + matching_close(&code[open..])? + 1)
	}).collect()
}

/// `test "counter" { … }`: its name and the source of each step
fn test_parts(test: &str) -> Option<(String, Vec<&str>)> {
	let name_start = TEST_WORD.len() + 2;
	let name_end = name_start + test[name_start..].find('"')?;
	let open = test.find('{')?;
	let body = &test[open + 1..open + matching_close(&test[open..])?];
	Some((test[name_start..name_end].to_string(), statements(body)))
}

/// Walks `code` outside quotes, giving each character with the bracket depth before it
fn unquoted(code: &str, mut visit: impl FnMut(usize, char, i32) -> bool) {
	let (mut depth, mut quote) = (0, None);
	for (at, character) in code.char_indices() {
		match (quote, character) {
			(Some(closing), _) if character == closing => quote = None,
			(Some(_), _) => {}
			(None, '"' | '\'') => quote = Some(character),
			(None, _) => {
				if !visit(at, character, depth) {
					return;
				}
				match character {
					'{' | '(' | '[' => depth += 1,
					'}' | ')' | ']' => depth -= 1,
					_ => {}
				}
			}
		}
	}
}

/// The offset of the bracket closing the one `code` starts with
fn matching_close(code: &str) -> Option<usize> {
	let mut close = None;
	unquoted(code, |at, character, depth| {
		let closes = depth == 1 && matches!(character, '}' | ')' | ']');
		if closes {
			close = Some(at);
		}
		!closes
	});
	close
}

/// The statements of a block's source: its lines and `;`-separated parts outside brackets and quotes
fn statements(body: &str) -> Vec<&str> {
	let mut cuts = vec![];
	unquoted(body, |at, character, depth| {
		if depth == 0 && matches!(character, '\n' | ';') {
			cuts.push(at);
		}
		true
	});
	let bounds = [0].into_iter().chain(cuts.iter().map(|cut| cut + 1)).zip(cuts.iter().copied().chain([body.len()]));
	bounds.map(|(start, end)| body[start..end].trim()).filter(|statement| !statement.is_empty()).collect()
}

/// The step word a statement starts with: render, click, fill, check
fn step_word(statement: &str) -> Option<&'static str> {
	let word = statement.split(|character: char| !character.is_alphanumeric() && character != '_').next()?;
	STEP_WORDS.into_iter().find(|step| *step == word)
}

/// The statement after its step word
fn after_word<'a>(statement: &'a str, word: &str) -> &'a str {
	statement[word.len()..].trim()
}

/// The texts a step names: `fill "name" with "Ada"` → name, Ada
fn texts(statement: &str) -> Vec<String> {
	match crate::warp_parser::parse(statement).drop_meta() {
		Node::List(items, _, _) => items.iter().filter_map(|item| match item.drop_meta() {
			Node::Text(text) => Some(text.clone()),
			_ => None,
		}).collect(),
		_ => vec![],
	}
}

fn run_test(setup: &str, steps: &[&str]) -> Result<(), String> {
	let mut setup = setup.to_string();
	let mut page: Option<Page> = None;
	for step in steps {
		match (step_word(step), texts(step).as_slice()) {
			(Some(RENDER_WORD), _) => {
				let markup = after_word(step, RENDER_WORD);
				page = Some(Page::render(&format!("{setup}\n{markup}")).map_err(|failure| format!("render {markup} failed: {}", failure.serialize()))?);
			}
			(Some(CLICK_WORD), [label]) => rendered(&mut page, step)?.click(label)?,
			(Some(FILL_WORD), [label, text]) if step.contains(WITH_WORD) => rendered(&mut page, step)?.type_into(label, text)?,
			(Some(CHECK_WORD), _) => check(rendered(&mut page, step)?, after_word(step, CHECK_WORD))?,
			(Some(word), _) => return Err(format!("{step}: write {word} as in the steps render markup, click \"label\", fill \"label\" with \"text\", check condition")),
			(None, _) => setup = format!("{setup}\n{step}"),
		}
	}
	Ok(())
}

fn rendered<'a>(page: &'a mut Option<Page>, step: &str) -> Result<&'a mut Page, String> {
	page.as_mut().ok_or_else(|| format!("{step} before render"))
}

/// `check text is "Add1"`: the condition with `text` and `html` bound to what the page shows
fn check(page: &Page, condition: &str) -> Result<(), String> {
	let (text, html) = (quoted(&page.text()), quoted(&page.html()));
	match crate::pipeline::eval(&format!("{TEXT_WORD} = {text}\n{HTML_WORD} = {html}\n{condition}")).drop_meta() {
		Node::True => Ok(()),
		Node::Number(Number::Int(truth)) if *truth != 0 => Ok(()),
		verdict => Err(format!("check {condition} gave {}; the page shows {text}", verdict.serialize())),
	}
}

/// A warp text literal of `text`
fn quoted(text: &str) -> String {
	format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
