// The clipboard (card system-clipboard, notes/system_signals.md): `on clipboard change {…}` listens to its change
// count, which reads no content (macOS asks the user before a program reads another one's copy); `clipboard` is its
// text, read where the program reads it. These tests never read the content.
use crate::is;

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program that needs a module").serialize()
}

#[test]
fn a_clipboard_listener_watches_the_change_count() {
	let code = lowered("on clipboard change { print \"copied\" }");
	assert!(code.contains("clipboard count") && code.contains("on·shared") && code.contains("signal_every"), "{code}");
	assert!(!code.contains("clipboard_text"), "listening reads no content: {code}");
}

#[test]
fn reading_the_clipboard_is_one_host_call() {
	assert!(lowered("print clipboard").contains("(clipboard_text)"));
	is!("clipboard = 3; clipboard", 3);
}

#[test]
#[cfg(all(feature = "native", target_os = "macos"))]
fn the_change_count_is_read_without_the_content() {
	is!("n = 1; on clipboard change { n = 2 }; n", 1);
	is!("c = clipboard count; c > 0", true);
}
