// A channel named by a ws:// or wss:// address is a WebSocket (card web-websocket, notes/web_framework.md "web-apis"):
// `on message from "ws://…" {…}` connects and hears what the server sends, `broadcast value on "ws://…"` sends to it; a
// text goes as it is, any other value as JSON, and an arriving JSON object or array is data, any other message a text.
// The server here is a real one on this machine that answers each message: a JSON object `o` with {"got": o}, any
// other text `t` with "echo t", so only a message that went through it and came back passes

/// A WebSocket server on a free local port that answers each message; its address
fn echo_server() -> String {
	let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
	let address = format!("ws://{}", listener.local_addr().expect("bound"));
	std::thread::spawn(move || {
		for stream in listener.incoming().flatten() {
			std::thread::spawn(move || {
				let Ok(mut socket) = tungstenite::accept(stream) else { return };
				while let Ok(tungstenite::Message::Text(text)) = socket.read() {
					let answer = if text.starts_with('{') { format!("{{\"got\": {text}}}") } else { format!("echo {text}") };
					if socket.send(tungstenite::Message::text(answer)).is_err() {
						return;
					}
				}
			});
		}
	});
	address
}

#[test]
fn a_web_socket_is_a_channel() {
	let server = echo_server();
	let code = format!("on message from \"{server}\" {{ print \"got \" + event; exit }}\nbroadcast \"hi\" on \"{server}\"\nsleep(3000 ms)\nprint \"nothing\"");
	assert_eq!(crate::common::printed(&code).lines().find(|line| !line.starts_with('»')), Some("got echo hi"));
}

#[test]
fn data_goes_as_json() {
	let server = echo_server();
	let code = format!("on message from \"{server}\" {{ print event.got.n + 1; exit }}\nbroadcast {{n: 41}} on \"{server}\"\nsleep(3000 ms)");
	assert_eq!(crate::common::printed(&code).lines().find(|line| !line.starts_with('»')), Some("42"));
}

#[test]
fn an_unreachable_server_is_a_loud_error() {
	crate::common::fails_with("on message from \"ws://127.0.0.1:9\" { print event }", "ws://127.0.0.1:9");
}

