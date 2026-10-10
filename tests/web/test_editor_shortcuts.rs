//! The playground editor's shortcuts (web/playground/shortcuts.js, card keyboard-shortcuts): Cmd-/ toggles warp's line
//! comments, here on the lines themselves (commentEdits) under node
use std::process::Command;

const SHORTCUTS: &str = include_str!("../../web/playground/shortcuts.js");
/// commentEdits applied to the lines of the JSON text, the lines that result
const APPLIED: &str = r#"
const lines = JSON.parse(process.argv[1]);
for (const { index, ch, removed, inserted } of commentEdits(lines)) lines[index] = lines[index].slice(0, ch) + inserted + lines[index].slice(ch + removed);
console.log(JSON.stringify(lines));
"#;

fn toggled(lines: &[&str]) -> Vec<String> {
	let output = Command::new("node").arg("-e").arg(format!("{SHORTCUTS}\n{APPLIED}")).arg(serde_json::to_string(lines).unwrap()).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("lines")
}

#[test]
fn cmd_slash_comments_lines_out_and_back_in() {
	assert_eq!(toggled(&["x = 1"]), ["// x = 1"]);
	assert_eq!(toggled(&["// x = 1"]), ["x = 1"]);
	assert_eq!(toggled(&["//x"]), ["x"]);
	// after the shallowest indentation, blank lines alone
	assert_eq!(toggled(&["if a {", "\tb", "", "}"]), ["// if a {", "// \tb", "", "// }"]);
	assert_eq!(toggled(&["\tf()", "\t\tg()"]), ["\t// f()", "\t// \tg()"]);
	assert_eq!(toggled(&["\t// f()", "\t// \tg()"]), ["\tf()", "\t\tg()"]);
	// some commented, some not: all commented
	assert_eq!(toggled(&["// a", "b"]), ["// // a", "// b"]);
	assert_eq!(toggled(&[""]), ["// "]);
}

/// moveLines on the lines of the JSON text with the selection [anchor line, anchor ch, head line, head ch], in an editor
/// that keeps lines and a selection like CodeMirror's: the lines and the selection after it
const MOVED: &str = r#"
const [lines, [anchorLine, anchorCh, headLine, headCh], step] = JSON.parse(process.argv[1]);
let anchor = { line: anchorLine, ch: anchorCh }, head = { line: headLine, ch: headCh };
const before = (one, other) => one.line < other.line || (one.line === other.line && one.ch <= other.ch);
const editor = {
	getCursor: end => ({ anchor, head, from: before(anchor, head) ? anchor : head, to: before(anchor, head) ? head : anchor })[end],
	getLine: line => lines[line],
	lineCount: () => lines.length,
	replaceRange: (text, from, to) => lines.splice(from.line, to.line - from.line + 1, ...(lines[from.line].slice(0, from.ch) + text + lines[to.line].slice(to.ch)).split("\n")),
	setSelection: (newAnchor, newHead) => { anchor = newAnchor; head = newHead; },
	operation: change => change(),
};
moveLines(editor, step);
console.log(JSON.stringify([lines, [anchor.line, anchor.ch, head.line, head.ch]]));
"#;

fn moved(lines: &[&str], selection: [usize; 4], step: i32) -> (Vec<String>, [usize; 4]) {
	let input = serde_json::to_string(&(lines, selection, step)).unwrap();
	let output = Command::new("node").arg("-e").arg(format!("{SHORTCUTS}\n{MOVED}")).arg(input).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("lines and selection")
}

// card keyboard-shortcut: Cmd-Ctrl-Up/Down (Alt-Up/Down elsewhere) move the selected lines, else the cursor's line
#[test]
fn cmd_ctrl_arrows_move_lines() {
	assert_eq!(moved(&["a", "b", "c"], [1, 1, 1, 1], -1), (vec!["b".into(), "a".into(), "c".into()], [0, 1, 0, 1]));
	assert_eq!(moved(&["a", "b", "c"], [1, 0, 1, 0], 1), (vec!["a".into(), "c".into(), "b".into()], [2, 0, 2, 0]));
	// a selection over two lines moves both, ending at the start of the next line leaves that one
	assert_eq!(moved(&["a", "b", "c", "d"], [1, 0, 3, 0], -1), (vec!["b".into(), "c".into(), "a".into(), "d".into()], [0, 0, 2, 0]));
	assert_eq!(moved(&["a", "b", "c", "d"], [2, 1, 0, 0], 1), (vec!["d".into(), "a".into(), "b".into(), "c".into()], [3, 1, 1, 0]));
	// at the edge nothing moves
	assert_eq!(moved(&["a", "b"], [0, 0, 0, 0], -1), (vec!["a".into(), "b".into()], [0, 0, 0, 0]));
	assert_eq!(moved(&["a", "b"], [1, 0, 1, 1], 1), (vec!["a".into(), "b".into()], [1, 0, 1, 1]));
}
