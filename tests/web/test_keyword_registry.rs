//! Card where-infix: the editor colors every keyword a pass gives meaning, with no list to keep by hand.
//! web/playground/keywords.py gathers soft_keywords.rs's lists and every `const …_KEYWORD: &str` in src/
use std::process::Command;

const KEYWORDS_SCRIPT: &str = "web/playground/keywords.py";

fn registered_keywords() -> serde_json::Value {
	let output = Command::new("python3").arg(KEYWORDS_SCRIPT).current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("python3 runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("keywords.py prints JSON")
}

#[test]
fn keywords_of_passes_are_highlighted() {
	let keywords = registered_keywords();
	let soft: Vec<&str> = keywords["soft"].as_array().unwrap().iter().filter_map(|word| word.as_str()).collect();
	for word in ["where", "loop", "when", "finally", "emit", "try"] {
		assert!(soft.contains(&word), "{word} is not highlighted: {soft:?}");
	}
	let hard = keywords["hard"].as_array().unwrap();
	assert!(hard.iter().any(|word| word == "if"));
	assert!(!soft.contains(&"if"), "a hard keyword is not listed as soft too");
}
