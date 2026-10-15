// key words and compared texts name keys in any case (user, 2026-10-15): ESC, Esc, escape, "escape" and "Escape" are
// all Escape (lowering/collections/key_names.rs); a single character stays itself, "a" and "A" being different presses
use crate::is;

/// How many of the presses the handler comparing `key == compared` counts
fn counted(compared: &str, pressed: &[&str]) -> String {
	let presses: String = pressed.iter().map(|key| format!("emit key{{key:\"{key}\"}}\n")).collect();
	format!("n = 0\non key {{ if key == {compared} {{ n += 1 }} }}\n{presses}n")
}

#[test]
fn key_words_and_texts_name_keys_in_any_case() {
	for compared in ["ESC", "Esc", "escape", "Escape", "\"escape\"", "\"ESCAPE\"", "\"Escape\""] {
		is!(&counted(compared, &["Escape", "q"]), 1);
	}
	is!(&counted("up", &["ArrowUp"]), 1);
	is!(&counted("\"arrowup\"", &["ArrowUp"]), 1);
	is!(&counted("f1", &["F1"]), 1);
}

#[test]
fn a_single_character_keeps_its_case() {
	is!(&counted("\"a\"", &["a", "A"]), 1);
}
