// Named events across programs and `send` (P129, P129b, notes/system_signals.md): `broadcast stop the machine{…} on
// "chat"` reaches `on stop the machine from "chat" {…}` in every program on this machine, `event` being the data;
// `send value to "chat"` is `broadcast value on "chat"`; `send "file system full"` sends that named event without data
// on the channel "warp", heard by `on "file system full" {…}`
#[cfg(all(feature = "native", unix))]
fn printed(code: &str) -> String {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_program_hears_its_own_named_event() {
	assert_eq!(printed("on stop the machine from \"test-named\" { print \"stop \" + event.reason }\nbroadcast stop the machine{reason: \"heat\"} on \"test-named\"\nsleep(200 ms)"), "stop heat\n");
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_named_event_is_no_message_and_no_other_event() {
	let code = "on message from \"test-named-only\" { print \"message\" }\non stop from \"test-named-only\" { print \"stop\" }\non go from \"test-named-only\" { print \"go\" }\nbroadcast stop{} on \"test-named-only\"\nsleep(200 ms)";
	assert_eq!(printed(code), "stop\n");
}

#[test]
#[cfg(all(feature = "native", unix))]
fn send_to_is_broadcast_on() {
	assert_eq!(printed("on message from \"test-send\" { print \"got \" + event }\nsend 21 to \"test-send\"\nsleep(200 ms)"), "got 21\n");
	assert_eq!(printed("on alarm from \"test-send-named\" { print \"alarm \" + event.level }\nsend alarm{level: 3} to \"test-send-named\"\nsleep(200 ms)"), "alarm 3\n");
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_text_alone_sends_a_named_event_without_data() {
	assert_eq!(printed("on \"test disk full\" { print \"full\" }\nsend \"test disk full\"\nsleep(200 ms)"), "full\n");
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_named_event_reaches_another_program() {
	let listener = crate::common::warp_command()
		.args(["--no-ask", "on stop the machine from \"test-named-other\" { print \"heard \" + event.reason; exit }\nsleep(3000 ms)\nprint \"nothing\""])
		.stdout(std::process::Stdio::piped())
		.spawn()
		.expect("the listener starts");
	std::thread::sleep(std::time::Duration::from_millis(800));
	printed("broadcast stop the machine{reason: \"done\"} on \"test-named-other\"");
	let output = listener.wait_with_output().expect("the listener ends");
	assert_eq!(String::from_utf8_lossy(&output.stdout).lines().next(), Some("heard done"));
}

#[test]
fn a_send_is_a_statement_of_no_value() {
	crate::is!("send 1 to \"test-nobody\"; 5", 5);
	crate::is!("send stop the machine{reason: \"x\"} to \"test-nobody\"; 5", 5);
}

#[test]
fn raise_stays_inside_the_program() {
	crate::is!("n = 1; on stop the machine { n = 2 }; raise stop the machine; n", 2);
}
