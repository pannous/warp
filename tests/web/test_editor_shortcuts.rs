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
