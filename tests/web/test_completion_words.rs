//! The playground editor's word completion sees the whole program (web/playground/completion.js, card
//! code-completion-should): after `use math`, `dir(ma` offers math before the keywords and the tour's words; Enter after
//! a word that is whole already is a new line, else it takes the chosen word (card completion-must); Claude's gray
//! continuation reads the whole program, cut around the cursor only past what the proxy takes (assistant.js codeAround)
use std::process::Command;

/// assistant.js MOST_CONTEXT_CHARACTERS
const MOST_CONTEXT_CHARACTERS: usize = 20000;
const COMPLETION: &str = include_str!("../../web/playground/completion.js");
const ASSISTANT: &str = include_str!("../../web/playground/assistant.js");
/// the page's globals completion.js reads: keywords.js (build.sh), examples.js and samples.js, here small ones
const PAGE: &str = r#"
globalThis.KEYWORDS = { hard: ["match", "if"], soft: ["print"], modules: ["math", "file"] };
globalThis.EXAMPLES = { tour: { code: "map map map main max" } };
globalThis.SAMPLES = {};
const editor = code => ({ getValue: () => code });
"#;

fn evaluated(expression: &str) -> serde_json::Value {
	let script = format!("{PAGE}\n{COMPLETION}\n{ASSISTANT}\nconsole.log(JSON.stringify({expression}));");
	let output = Command::new("node").arg("-e").arg(script).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("JSON")
}

#[test]
fn the_programs_own_words_come_first() {
	let offered = evaluated(r#"wordChoices(editor("use math\ndir(ma"), "ma", "dir(ma").map(choice => choice.text)"#);
	assert_eq!(offered, serde_json::json!(["math", "match", "map", "main", "max"]));
}

#[test]
fn enter_after_a_whole_word_is_a_new_line() {
	let whole = evaluated(r#"[isWholeWord(editor("print"), "print"), isWholeWord(editor("xs = 1\nxs"), "xs"), isWholeWord(editor("xs = 1\nx"), "x")]"#);
	assert_eq!(whole, serde_json::json!([true, true, false]));
}

#[test]
fn claude_reads_the_whole_program_unless_it_is_too_long() {
	assert_eq!(evaluated(r#"codeAround("use math\ndir(m", ")")"#), "use math\ndir(m‸)");
	let cut = evaluated(r#"(code => [code.length, code.indexOf("‸")])(codeAround("a".repeat(30000), "b".repeat(30000)))"#);
	assert_eq!(cut, serde_json::json!([MOST_CONTEXT_CHARACTERS, 15000])); // three quarters before the cursor
}
