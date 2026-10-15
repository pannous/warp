// key names in any case (user, 2026-10-15): compared with `key` or `held_key` itself, Escape, escape, "escape",
// "ESCAPE" and "Escape" all name Escape (lowering/collections/key_names.rs; lowercase words: card if-key); a single
// character keeps its case, "a" and "A" being different presses. Other comparisons in a handler stay as written
use crate::is;

/// How many of the presses the handler comparing `key == compared` counts
fn counted(compared: &str, pressed: &[&str]) -> String {
	let presses: String = pressed.iter().map(|key| format!("emit key{{key:\"{key}\"}}\n")).collect();
	format!("n = 0\non key {{ if key == {compared} {{ n += 1 }} }}\n{presses}n")
}

#[test]
fn key_words_and_texts_name_keys_in_any_case() {
	for compared in ["ESC", "Escape", "escape", "\"escape\"", "\"ESCAPE\"", "\"Escape\""] {
		is!(&counted(compared, &["Escape", "q"]), 1);
	}
	is!(&counted("\"arrowup\"", &["ArrowUp"]), 1);
	is!(&counted("\"f1\"", &["F1"]), 1);
}

#[test]
fn a_single_character_keeps_its_case() {
	is!(&counted("\"a\"", &["a", "A"]), 1);
}

#[test]
fn other_comparisons_in_a_handler_stay_as_written() {
	is!("seen = \"\"\non key { mode = \"up\"\n seen = if mode == \"up\" then \"same\" else \"differs\" }\nemit key{key:\"q\"}\nseen", "same");
	is!("seen = \"\"\non key { left = 3\n seen = if 3 == left then \"same\" else \"differs\" }\nemit key{key:\"q\"}\nseen", "same");
}
