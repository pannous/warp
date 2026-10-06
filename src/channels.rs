//! Channels between programs (wiki/signal.md, notes/system_signals.md): `broadcast value on "chat"` sends the value as
//! wasp text to every program on this machine listening with `on message from "chat" {…}`. A channel is the directory
//! `/tmp/warp-channels-<user>/<channel>` (the temp directory off unix), a named event's (`chat/stop the machine`, P129b)
//! the directory `stop_the_machine` inside it; each listener binds a Unix datagram socket there (its file removed when the run's
//! listeners are forgotten), a broadcast sends one datagram to each socket in it and removes those nobody reads any more.
//! A listener keeps what arrived until the program asks: channel_pending(id) reads the socket without waiting,
//! channel_next(id) gives the oldest message.
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use crate::system_signals::EVENT_SEPARATOR;

const CHANNELS_DIRECTORY: &str = "warp-channels";
#[cfg(unix)]
const SHORT_TEMP_DIRECTORY: &str = "/tmp";
const SOCKET_EXTENSION: &str = "sock";
/// The largest message: a datagram must fit in one read
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[cfg(unix)]
struct Listener {
	socket: std::os::unix::net::UnixDatagram,
	path: PathBuf,
	received: VecDeque<String>,
}

#[cfg(unix)]
impl Drop for Listener {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}

#[cfg(unix)]
thread_local! {
	/// the listeners of the run on this thread, by the id the program gave each
	static LISTENERS: RefCell<HashMap<i64, Listener>> = RefCell::new(HashMap::new());
}

/// The directory of a channel, a named event's inside its channel's; a name is kept to letters, digits, `-` and `_`
fn directory(channel: &str) -> PathBuf {
	let kept = |name: &str| -> String { name.chars().map(|letter| if letter.is_alphanumeric() || letter == '-' || letter == '_' { letter } else { '_' }).collect() };
	channel.split(EVENT_SEPARATOR).fold(channels_root(), |directory, name| directory.join(kept(name)))
}

/// `/tmp/warp-channels-<user>` on unix: a socket path has at most 104 bytes, and macOS's own temp directory alone takes
/// about 50 of them; elsewhere the temp directory
fn channels_root() -> PathBuf {
	#[cfg(unix)]
	{
		let user = std::env::var("USER").or_else(|_| std::env::var("LOGNAME")).unwrap_or_default();
		PathBuf::from(SHORT_TEMP_DIRECTORY).join(format!("{CHANNELS_DIRECTORY}-{user}"))
	}
	#[cfg(not(unix))]
	std::env::temp_dir().join(CHANNELS_DIRECTORY)
}

/// A new run on this thread starts without listeners
pub fn forget_listeners() {
	#[cfg(unix)]
	LISTENERS.with(|listeners| listeners.borrow_mut().clear());
}

/// `channel_listen(id, channel)`: listen on the channel from now on
pub fn listen(id: i64, channel: &str) -> Result<(), String> {
	#[cfg(unix)]
	{
		let directory = directory(channel);
		std::fs::create_dir_all(&directory).map_err(|problem| format!("on message from {channel:?}: {problem}"))?;
		let path = directory.join(format!("{}-{id}.{SOCKET_EXTENSION}", std::process::id()));
		let _ = std::fs::remove_file(&path);
		let socket = std::os::unix::net::UnixDatagram::bind(&path).map_err(|problem| format!("on message from {channel:?}: {problem}"))?;
		socket.set_nonblocking(true).map_err(|problem| problem.to_string())?;
		LISTENERS.with(|listeners| listeners.borrow_mut().insert(id, Listener { socket, path, received: VecDeque::new() }));
		Ok(())
	}
	#[cfg(not(unix))]
	Err(format!("on message from {channel:?}: channels need Unix sockets, not here yet"))
}

/// `channel_pending(id)`: how many messages wait, after reading what arrived
pub fn pending(id: i64) -> i64 {
	#[cfg(unix)]
	return LISTENERS.with(|listeners| {
		let mut listeners = listeners.borrow_mut();
		let Some(listener) = listeners.get_mut(&id) else { return 0 };
		let mut buffer = vec![0u8; MAX_MESSAGE_BYTES];
		while let Ok(length) = listener.socket.recv(&mut buffer) {
			listener.received.push_back(String::from_utf8_lossy(&buffer[..length]).into_owned());
		}
		listener.received.len() as i64
	});
	#[cfg(not(unix))]
	0
}

/// `channel_next(id)`: the oldest message waiting, as wasp text
pub fn next(id: i64) -> Option<String> {
	#[cfg(unix)]
	return LISTENERS.with(|listeners| listeners.borrow_mut().get_mut(&id)?.received.pop_front());
	#[cfg(not(unix))]
	None
}

/// `channel_send(channel, message)`: the message to every listener of the channel; one nobody reads any more is removed
pub fn send(channel: &str, message: &str) -> Result<(), String> {
	if message.len() > MAX_MESSAGE_BYTES {
		return Err(format!("broadcast on {channel:?}: a message of {} bytes is longer than {MAX_MESSAGE_BYTES}", message.len()));
	}
	#[cfg(unix)]
	{
		let Ok(entries) = std::fs::read_dir(directory(channel)) else { return Ok(()) };
		let sender = std::os::unix::net::UnixDatagram::unbound().map_err(|problem| problem.to_string())?;
		for path in entries.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == SOCKET_EXTENSION)) {
			if let Err(problem) = sender.send_to(message.as_bytes(), &path) {
				if matches!(problem.kind(), std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound) {
					let _ = std::fs::remove_file(&path);
				}
			}
		}
		Ok(())
	}
	#[cfg(not(unix))]
	Err(format!("broadcast on {channel:?}: channels need Unix sockets, not here yet"))
}
