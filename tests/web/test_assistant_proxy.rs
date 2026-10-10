//! The playground assistant's proxy for visitors without a key (web/assistant/, card assistant-proxy) is no open relay:
//! web/assistant/test_assistant.mjs runs it under wrangler dev against the real Anthropic API (skipped without a key)
use std::process::Command;

#[test]
fn the_assistant_proxy_answers_only_the_playground() {
	let output = Command::new("node").arg("web/assistant/test_assistant.mjs").output().expect("node runs");
	let report = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
	assert!(output.status.success(), "{report}");
	assert!(report.contains("all checks passed") || report.contains("SKIPPED"), "{report}");
}
