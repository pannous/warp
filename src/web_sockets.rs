//! A channel named by a ws:// or wss:// address is a WebSocket (card web-websocket, notes/web_framework.md "web-apis"):
//! `on message from "wss://…" {…}` connects at once (an unreachable server is a loud error there) and hears what the
//! server sends, `broadcast value on "wss://…"` sends to it over the same connection, or a new one. A thread owns each
//! connection: it reads with a short timeout and writes what the program sent in between. An arriving JSON object or
//! array is data, any other message a text (as an HTTP body, web_server.rs value_of_body).
//! Several listeners on one address share its connection, so each message reaches one of them.
use crate::node::Node;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message;

const SOCKET_SCHEMES: [&str; 2] = ["ws://", "wss://"];
/// How long the connection's thread waits for a message before it looks for one to send
const READ_TIMEOUT: Duration = Duration::from_millis(20);

struct Connection {
	outgoing: Sender<String>,
	incoming: Receiver<String>,
	received: VecDeque<String>,
}

thread_local! {
	/// the connections of the run on this thread, by address
	static CONNECTIONS: RefCell<HashMap<String, Connection>> = RefCell::new(HashMap::new());
	/// the address each socket listener (by the id the program gave it) hears
	static LISTENERS: RefCell<HashMap<i64, String>> = RefCell::new(HashMap::new());
}

pub fn is_socket_address(channel: &str) -> bool {
	SOCKET_SCHEMES.iter().any(|scheme| channel.starts_with(scheme))
}

/// A new run on this thread starts without connections (their threads end with their channels)
pub fn forget() {
	LISTENERS.with(|listeners| listeners.borrow_mut().clear());
	CONNECTIONS.with(|connections| connections.borrow_mut().clear());
}

pub fn listen(id: i64, address: &str) -> Result<(), String> {
	connected(address, |_| ()).map_err(|problem| format!("on message from {address:?}: {problem}"))?;
	LISTENERS.with(|listeners| listeners.borrow_mut().insert(id, address.to_string()));
	Ok(())
}

/// How many messages wait for listener `id`, None when it hears no WebSocket
pub fn pending(id: i64) -> Option<i64> {
	let address = LISTENERS.with(|listeners| listeners.borrow().get(&id).cloned())?;
	connected(&address, |connection| {
		connection.received.extend(connection.incoming.try_iter());
		connection.received.len() as i64
	}).ok()
}

/// The oldest message for listener `id` as a value (ø when none waits), None when it hears no WebSocket
pub fn next(id: i64) -> Option<Node> {
	let address = LISTENERS.with(|listeners| listeners.borrow().get(&id).cloned())?;
	let message = connected(&address, |connection| connection.received.pop_front()).ok().flatten();
	Some(message.map_or(Node::Empty, crate::web_server::value_of_body))
}

/// A text as it is, any other value as JSON
pub fn send(address: &str, message: &Node) -> Result<(), String> {
	let text = match message.drop_meta() {
		Node::Text(text) => text.clone(),
		other => other.to_json_compact().map_err(|problem| format!("broadcast on {address:?}: {problem}"))?,
	};
	connected(address, |connection| connection.outgoing.send(text))
		.and_then(|sent| sent.map_err(|_| "the connection is closed".to_string()))
		.map_err(|problem| format!("broadcast on {address:?}: {problem}"))
}

/// `action` on the connection to `address`, connecting first when there is none
fn connected<T>(address: &str, action: impl FnOnce(&mut Connection) -> T) -> Result<T, String> {
	CONNECTIONS.with(|connections| {
		let mut connections = connections.borrow_mut();
		if !connections.contains_key(address) {
			connections.insert(address.to_string(), connect(address)?);
		}
		Ok(action(connections.get_mut(address).expect("connected")))
	})
}

fn connect(address: &str) -> Result<Connection, String> {
	let (mut socket, _) = tungstenite::connect(address).map_err(|problem| problem.to_string())?;
	let stream = match socket.get_mut() {
		MaybeTlsStream::Plain(stream) => stream,
		MaybeTlsStream::Rustls(stream) => stream.get_mut(),
		_ => return Err("an unknown kind of stream".to_string()),
	};
	stream.set_read_timeout(Some(READ_TIMEOUT)).map_err(|problem| problem.to_string())?;
	let (outgoing, to_send) = channel::<String>();
	let (arrived, incoming) = channel();
	std::thread::spawn(move || loop {
		match socket.read() {
			Ok(Message::Text(text)) => {
				if arrived.send(text.to_string()).is_err() {
					return;
				}
			}
			Ok(Message::Close(_)) => return,
			Ok(_) => {}
			Err(tungstenite::Error::Io(problem)) if matches!(problem.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
			Err(_) => return,
		}
		loop {
			match to_send.try_recv() {
				Ok(text) => {
					if socket.send(Message::text(text)).is_err() {
						return;
					}
				}
				Err(std::sync::mpsc::TryRecvError::Empty) => break,
				Err(std::sync::mpsc::TryRecvError::Disconnected) => return, // the program's run ended
			}
		}
	});
	Ok(Connection { outgoing, incoming, received: VecDeque::new() })
}

