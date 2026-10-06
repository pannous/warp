// Broadcast and channel listeners (wiki/signal.md, notes/system_signals.md): `broadcast value on "chat"` sends a value to
// every program on this machine listening on that channel with `on message from "chat" {…}`, `event` being the value;
// without a channel name both use the channel "warp". Natively a channel is a directory of Unix datagram sockets
#[cfg(all(feature = "native", unix))]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_program_hears_its_own_broadcast() {
	assert_eq!(printed("on message from \"test-own\" { print \"got \" + event.text }\nbroadcast {text: \"hi\"} on \"test-own\"\nsleep(200 ms)"), "got hi\n");
	assert_eq!(printed("on message { print \"got \" + event }\nbroadcast 21\nsleep(200 ms)"), "got 21\n");
}

#[test]
#[cfg(all(feature = "native", unix))]
fn a_broadcast_reaches_another_program() {
	let listener = crate::common::warp_command()
		.args(["--no-ask", "on message from \"test-other\" { print \"heard \" + event; exit }\nsleep(3000 ms)\nprint \"nothing\""])
		.stdout(std::process::Stdio::piped())
		.spawn()
		.expect("the listener starts");
	std::thread::sleep(std::time::Duration::from_millis(800));
	printed("broadcast \"ping\" on \"test-other\"");
	let output = listener.wait_with_output().expect("the listener ends");
	assert_eq!(String::from_utf8_lossy(&output.stdout).lines().next(), Some("heard ping"));
}

#[test]
#[cfg(all(feature = "native", unix))]
fn nobody_listening_is_no_error() {
	assert_eq!(printed("broadcast 1 on \"test-nobody\"\nprint \"sent\""), "sent\n");
}

#[test]
fn a_broadcast_is_a_statement_of_no_value() {
	crate::is!("broadcast {text: \"hi\"} on \"test-nobody\"; 5", 5);
}
