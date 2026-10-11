//! The playground's gray continuation is code only, usually one line (web/playground/assistant.js terseContinuation,
//! card completion-terse): Claude's explanations, results and comments are dropped before it shows, under node
use std::process::Command;

const ASSISTANT: &str = include_str!("../../web/playground/assistant.js");
const SHOWN: &str = r#"
const [answer, before] = JSON.parse(process.argv[1]);
console.log(JSON.stringify(terseContinuation(answer, isInComment(before))));
"#;

/// what the editor shows of Claude's `answer` at a cursor after `before`
fn shown(answer: &str, before: &str) -> String {
	let input = serde_json::to_string(&[answer, before]).unwrap();
	let output = Command::new("node").arg("-e").arg(format!("{ASSISTANT}\n{SHOWN}")).arg(input).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("text")
}

#[test]
fn an_explaining_answer_shows_only_its_code() {
	// the answer the user saw inserted after `[1 2 3] where it > 1`
	let rambling = "Hmm, where is a comprehension filter, not a postfix…\n.filter(it > 1)\nWait, …\n=> [2 3]";
	assert_eq!(shown(rambling, "[1 2 3] where it > 1"), ".filter(it > 1)");
	assert_eq!(shown("Here is the code:\n```warp\nfib(n-1) + fib(n-2)\n```\nThis recurses twice.", "fib(n) := "), "fib(n-1) + fib(n-2)");
	assert_eq!(shown("This returns the doubled list, since map applies it to each item.", "xs.map"), "");
	let nothing = "Nothing to insert: `[1 2 3] where it > 1` already evaluates to `[2 3]`, so the program is complete as written.";
	assert_eq!(shown(nothing, "[1 2 3] where it > 1"), "");
	assert_eq!(shown(" + `hello` + name", "greeting = "), " + `hello` + name");
}

#[test]
fn a_continuation_is_one_line_unless_it_opens_a_block() {
	assert_eq!(shown(" + 1\nprint x\nprint y", "x = 1"), " + 1");
	assert_eq!(shown(" {\n\tprint x\n}\nprint y", "for x in xs"), " {\n\tprint x\n}");
	assert_eq!(shown("print x for x in xs", ""), "print x for x in xs");
}

#[test]
fn comments_only_inside_a_comment() {
	assert_eq!(shown(" * 2 // doubles it", "x = y"), " * 2");
	assert_eq!(shown("// the next step\nx + 1", "y = "), "x + 1");
	assert_eq!(shown(" fetch \"https://x.com/a\"", "page ="), " fetch \"https://x.com/a\"");
	assert_eq!(shown(" the squares of the list\nsquares = xs.map(it * it)", "// next:"), " the squares of the list");
	assert_eq!(shown(" * 2 // doubles it", "x = \"a//b\" + y"), " * 2");
}
