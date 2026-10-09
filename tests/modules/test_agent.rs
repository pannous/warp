//! card g_X_F0: `agent "prompt"` (lib/agent.warp, loaded when called) sends the prompt to the Anthropic Messages API
//! with the key in ANTHROPIC_API_KEY and gives the answer's text. Without a key only the failures are checked: the
//! API itself answers a wrong key
use crate::common::fails_with;
use std::sync::Mutex;

const KEY: &str = "ANTHROPIC_API_KEY";
/// The tests here change the process's environment: one at a time
static ENVIRONMENT: Mutex<()> = Mutex::new(());

/// `run` with the key variable set to `key` (unset for None), the variable restored afterwards
fn with_key(key: Option<&str>, run: impl FnOnce()) {
	let _alone = ENVIRONMENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let kept = std::env::var(KEY).ok();
	match key {
		Some(key) => std::env::set_var(KEY, key),
		None => std::env::remove_var(KEY),
	}
	let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
	match kept {
		Some(kept) => std::env::set_var(KEY, kept),
		None => std::env::remove_var(KEY),
	}
	outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}

#[test]
fn agent_without_a_key_names_the_variable() {
	with_key(None, || fails_with("agent \"say hi\"", "ANTHROPIC_API_KEY"));
}

// card g_n6vg: the failure says where the key goes, in a terminal and in the playground
#[test]
fn agent_without_a_key_says_where_to_set_it() {
	with_key(None, || fails_with("agent \"say hi\"", "export ANTHROPIC_API_KEY="));
	with_key(None, || fails_with("agent \"say hi\"", "⋯ menu"));
}

#[test]
fn agent_with_a_wrong_key_fails_with_the_api_reason() {
	with_key(Some("not-a-key"), || fails_with("agent \"say hi\"", "invalid x-api-key"));
}

#[test]
fn agent_answers_a_prompt_when_a_key_is_set() {
	let _alone = ENVIRONMENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if std::env::var(KEY).is_err() {
		eprintln!("note: {KEY} unset, agent's answer not checked");
		return;
	}
	let answer = warp::wasm_emitter::eval("agent \"Answer with the single word yes, nothing else\"").serialize().to_lowercase();
	assert!(answer.contains("yes"), "{answer}");
}
