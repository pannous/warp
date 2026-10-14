//! Card key-app: `on key {…}` of a program shown by `warp app` (and of any built site) hears the page's key presses,
//! named as KeyboardEvent.key, and `exit` in a handler closes the app with its code (samples/hello_div.warp: `on key
//! { exit }`); `--check` loads it in a hidden window, its own script presses the key
use std::path::PathBuf;

const PRESSES_Q: &str =
	"script{ \"setTimeout(() => document.dispatchEvent(new KeyboardEvent('keydown', {key: 'q'})), 100)\" }";

pub(crate) fn checked(name: &str, code: &str) -> std::process::Output {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("test_app_keys");
	std::fs::create_dir_all(&folder).unwrap();
	let page = folder.join(format!("{name}.warp"));
	std::fs::write(&page, code).unwrap();
	crate::common::warp_command().args(["app", "--check"]).arg(&page).env("WARP_NO_WINDOW", "1").output().unwrap()
}

fn checked_text(code: &str) -> String {
	let output = checked("keys", code);
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn on_key_hears_the_app_pages_key_presses() {
	let text = checked_text(&format!(
		"pressed = \"none\"\non key {{ pressed = event.key }}\ndiv{{ p{{ \"pressed \" + pressed }} {PRESSES_Q} }}"
	));
	assert!(text.contains("pressed q"), "{text}");
}

#[test]
fn exit_in_a_key_handler_ends_the_app_with_its_code() {
	let output = checked("exits", &format!("on key {{ exit 3 }}\ndiv{{ p{{ \"press a key\" }} {PRESSES_Q} }}"));
	assert_eq!(output.status.code(), Some(3), "{}", String::from_utf8_lossy(&output.stdout));
}
