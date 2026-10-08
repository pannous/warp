//! Async data (card web-async, notes/web_framework.md "Async data"): `users := fetch "/api/users"` (lowering/fetch_signals.rs)
//! calls fetch_start(id, url), which fetches on a thread of its own while the program goes on; the handler on·fetch·id
//! runs at the first check point after the reply arrived (warp-runtime system_signals await_ready) and takes it with
//! fetch_reply(id): [value, ø] with the JSON parsed (any other body the text), or [ø, failure].
use crate::node::{Bracket, Node, Separator};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

type Reply = Result<String, String>;
/// The URL and, for a POST, its body
pub type Request = (String, Option<String>);

const THREAD_ENDED: &str = "fetch failed: the fetching thread ended";

enum Fetch {
	Pending(Receiver<Reply>),
	Arrived(Reply),
}

thread_local! {
	/// the fetches of the run on this thread, by the id the program gave each; a fetch started anew replaces its own
	static FETCHES: RefCell<HashMap<i64, Fetch>> = RefCell::new(HashMap::new());
}

/// Start fetching `url` (POSTing the body when there is one); the handler on·fetch·id runs once the reply arrived (a
/// reply still on its way is dropped)
pub fn start(id: i64, (url, body): Request, timeout: Duration) {
	let (sender, receiver) = channel();
	let awaited = format!("fetch {url}");
	std::thread::spawn(move || sender.send(match body {
		Some(body) => crate::extensions::utils::post_within(&url, &body, timeout).map_err(|reason| format!("fetch {url} failed: {reason}")),
		None => crate::host::fetch(&url, timeout),
	}));
	FETCHES.with(|fetches| fetches.borrow_mut().insert(id, Fetch::Pending(receiver)));
	let handler = format!("{}{id}", crate::host::FETCH_HANDLER_PREFIX);
	warp_runtime::system_signals::await_ready(handler, awaited, move || arrived(id));
}

/// Whether the reply of fetch `id` is here, kept for fetch_reply
fn arrived(id: i64) -> bool {
	FETCHES.with(|fetches| {
		let mut fetches = fetches.borrow_mut();
		let Some(Fetch::Pending(receiver)) = fetches.get(&id) else { return fetches.contains_key(&id) };
		let reply = match receiver.try_recv() {
			Ok(reply) => reply,
			Err(std::sync::mpsc::TryRecvError::Empty) => return false,
			Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(THREAD_ENDED.to_string()),
		};
		fetches.insert(id, Fetch::Arrived(reply));
		true
	})
}

/// [value, error] of fetch `id`, waiting for it when it has not arrived yet; [ø, failure] of an id never started
pub fn reply(id: i64) -> Node {
	let fetch = FETCHES.with(|fetches| fetches.borrow_mut().remove(&id));
	let reply = match fetch {
		Some(Fetch::Arrived(reply)) => reply,
		Some(Fetch::Pending(receiver)) => receiver.recv().unwrap_or_else(|_| Err(THREAD_ENDED.to_string())),
		None => Err(format!("fetch {id} was never started")),
	};
	let (value, error) = match reply {
		Ok(body) => (crate::web_server::value_of_body(body), Node::Empty),
		Err(failure) => (Node::Empty, Node::Text(failure)),
	};
	Node::List(vec![value, error], Bracket::Square, Separator::Colon)
}
