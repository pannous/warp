//! The browser platform from wasp without glue (card web-apis, notes/web_framework.md "web-apis"): `notify "text"` is a
//! desktop notification (the browser's Notification in the playground, natively osascript / notify-send). Only the
//! compiled call is checked here: a test run shows no notification
use crate::common::imports_of;
use crate::is;

#[test]
fn notify_is_a_host_word() {
	let notifies = |code: &str| imports_of(code).contains(&("host".to_string(), "notify".to_string()));
	assert!(notifies("notify \"done\""));
	assert!(notifies("n = 3\nnotify \"left: \" + n"));
	assert!(!notifies("notify(x) := x + 1; notify(2)"));
	is!("notify(x) := x + 1; notify(2)", 3); // a program's own notify stays its own
}

