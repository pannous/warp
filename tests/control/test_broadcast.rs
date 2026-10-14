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
	use std::io::BufRead;
	// the listener says when it listens: a fixed head start was too short under load (card broadcast-test)
	let mut listener = crate::common::warp_command()
		.args(["--no-ask", "on message from \"test-other\" { print \"heard \" + event; exit }\nprint \"listening\"\nsleep(30000 ms)\nprint \"nothing\""])
		.stdout(std::process::Stdio::piped())
		.spawn()
		.expect("the listener starts");
	let mut lines =
		std::io::BufReader::new(listener.stdout.take().expect("the listener's output")).lines().map_while(Result::ok);
	assert_eq!(lines.next().as_deref(), Some("listening"));
	printed("broadcast \"ping\" on \"test-other\"");
	assert_eq!(lines.next().as_deref(), Some("heard ping"));
	listener.wait().expect("the listener ends");
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
