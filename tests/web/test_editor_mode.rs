//! The playground editor's warp mode (web/playground/codemirror-warp-mode.js, card playground-editor) without a built
//! keywords.js: index.html's fallback gives empty keyword lists, and no rule may then match the empty string, else
//! CodeMirror stops on every edit ("Mode warp failed to advance stream")
use std::process::Command;

const MODE: &str = include_str!("../../web/playground/codemirror-warp-mode.js");
/// the mode's rules under a stub CodeMirror, the rules matching nothing at the start of each line
const EMPTY_MATCHES: &str = r#"
const lines = ["x = 1", "if a then b", "// note", "use math"];
const empty = [];
for (const rules of Object.values(defined).filter(Array.isArray))
	for (const { regex } of rules)
		for (const line of lines) {
			const found = new RegExp(regex.source, regex.flags.replace("g", "")).exec(line);
			if (found && found.index === 0 && found[0] === "") empty.push(`${regex} on ${line}`);
		}
console.log(JSON.stringify(empty));
"#;

#[test]
fn the_mode_advances_without_built_keywords() {
	let stub = "const KEYWORDS = {hard: [], soft: []};\nlet defined;\nconst CodeMirror = { defineSimpleMode: (name, states) => { defined = states; } };\n";
	let output = Command::new("node").arg("-e").arg(format!("{stub}{MODE}\n{EMPTY_MATCHES}")).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
}
