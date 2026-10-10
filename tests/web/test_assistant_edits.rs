//! The chat's code into the editor as edits (web/playground/assistant.js, card put-editor): a diff changes the lines
//! it names, a whole program replaces only the lines that differ; here the edits applied to a text as CodeMirror's
//! replaceRange would, under node
use std::process::Command;

const ASSISTANT: &str = include_str!("../../web/playground/assistant.js");
/// answerEdits of [text, language, code] applied last first through editRange, the text that results (or the failure)
const APPLIED: &str = r#"
let [text, language, code] = JSON.parse(process.argv[1]);
try {
	for (const edit of [...answerEdits(text, language, code)].reverse()) {
		const lines = text.split("\n");
		const { start, end, text: inserted } = editRange(edit, lines.length, line => lines[line].length);
		const offset = ({ line, ch }) => lines.slice(0, line).reduce((sum, before) => sum + before.length + 1, 0) + ch;
		text = text.slice(0, offset(start)) + inserted + text.slice(offset(end));
	}
	console.log(JSON.stringify(text));
} catch (failure) {
	console.log(JSON.stringify("failed: " + failure.message));
}
"#;

fn applied(text: &str, language: &str, code: &str) -> String {
	let input = serde_json::to_string(&[text, language, code]).unwrap();
	let output = Command::new("node").arg("-e").arg(format!("{ASSISTANT}\n{APPLIED}")).arg(input).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("text")
}

const PROGRAM: &str = "x = 1\ny = 2\nprint x + y\n";

#[test]
fn a_diff_changes_only_the_lines_it_names() {
	let diff = "--- a\n+++ b\n@@ -2,2 +2,2 @@\n y = 2\n-print x + y\n+print x * y\n";
	assert_eq!(applied(PROGRAM, "diff", diff), "x = 1\ny = 2\nprint x * y\n");
	// several hunks, lines added at the start and the end, the @@ line numbers only a hint
	let diff = "@@ -1,1 +1,2 @@\n+// sum\n x = 1\n@@ -9,1 +10,2 @@\n print x + y\n+print \"done\"\n";
	assert_eq!(applied(PROGRAM, "diff", diff), "// sum\nx = 1\ny = 2\nprint x + y\nprint \"done\"\n");
	// lines removed, the empty context line without its space
	assert_eq!(applied("a\n\nb\nc", "diff", "@@ -2,3 +2,2 @@\n\n b\n-c\n"), "a\n\nb");
	assert_eq!(applied("a\nb", "diff", "@@ -1,2 +1,1 @@\n-a\n b\n"), "b");
}

#[test]
fn a_diff_that_no_longer_fits_says_so() {
	let failure = applied(PROGRAM, "diff", "@@ -1,1 +1,1 @@\n-z = 9\n+z = 10\n");
	assert!(failure.starts_with("failed: ") && failure.contains("z = 9"), "{failure}");
}

#[test]
fn a_whole_program_replaces_only_the_lines_that_differ() {
	assert_eq!(applied(PROGRAM, "warp", "x = 1\ny = 3\nprint x + y\n"), "x = 1\ny = 3\nprint x + y\n");
	assert_eq!(applied(PROGRAM, "warp", "x = 1\n"), "x = 1\n");
	assert_eq!(applied("", "warp", "print 1\n"), "print 1");
	assert_eq!(applied(PROGRAM, "", "x = 0\ny = 2\nprint x + y\nprint 7\n"), "x = 0\ny = 2\nprint x + y\nprint 7\n");
}
