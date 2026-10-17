//! Channels between the tasks of one program: `channel()`, put, take and close, and the host words that reach them
use super::*;

pub(crate) const CHANNEL_NEW: &str = crate::host::CHANNEL_WORDS[0];
pub(crate) const CHANNEL_PUT: &str = crate::host::CHANNEL_WORDS[1];
pub(crate) const CHANNEL_TAKE: &str = crate::host::CHANNEL_WORDS[2];
pub(crate) const CHANNEL_MORE: &str = crate::host::CHANNEL_WORDS[3];
pub(crate) const CHANNEL_CLOSE: &str = crate::host::CHANNEL_WORDS[4];
/// How often a waiting send or receive looks whether it waits forever (the program alone, or ended)
pub(crate) const CHANNEL_CHECK: Duration = Duration::from_millis(20);

/// One local channel (P155): the values offered and not yet taken, in order; a send waits until the receives counted
/// past its own number
#[derive(Default)]
pub(super) struct Channel {
	offered: std::collections::VecDeque<TaskValue>,
	sent: u64,
	taken: u64,
	closed: bool,
}

/// The local channels of a run (`ch = channel()`, notes/channels.md): Go's unbuffered channels between its tasks
#[derive(Default)]
pub(super) struct Channels {
	channels: Mutex<HashMap<i64, Channel>>,
	changed: Condvar,
	next_id: AtomicI64,
	/// the program ended: a task still waiting on a channel stops (as Go drops its goroutines)
	ended: std::sync::atomic::AtomicBool,
}

impl Channels {
	fn open(&self) -> i64 {
		let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
		self.channels.lock().expect("channels").insert(id, Channel::default());
		id
	}

	/// Wait on channel `id` until `ready` gives an answer; `alone` says whether nothing else could ever change it
	fn wait<T>(
		&self,
		id: i64,
		alone: &dyn Fn() -> bool,
		waiting: &str,
		mut ready: impl FnMut(&mut Channel) -> Option<Result<T, String>>,
	) -> Result<T, String> {
		let mut channels = self.channels.lock().expect("channels");
		loop {
			let channel = channels.get_mut(&id).ok_or_else(|| format!("no channel {id}"))?;
			if let Some(answer) = ready(channel) {
				self.changed.notify_all();
				return answer;
			}
			if self.ended.load(Ordering::Relaxed) {
				return Err(STOPPED.to_string());
			}
			if alone() {
				return Err(format!("{waiting} waits forever: no task is left to answer it"));
			}
			channels = self.changed.wait_timeout(channels, CHANNEL_CHECK).expect("channels").0;
		}
	}

	/// `ch.send(v)`: offers v, then waits until a receiver took it
	fn put(&self, id: i64, value: TaskValue, alone: &dyn Fn() -> bool) -> Result<(), String> {
		let mut number = None;
		self.wait(id, alone, "ch.send", |channel| {
			if channel.closed && number.is_none() {
				return Some(Err("send on a closed channel".to_string()));
			}
			let mine = *number.get_or_insert_with(|| {
				channel.offered.push_back(value.clone());
				channel.sent += 1;
				channel.sent - 1
			});
			(channel.taken > mine).then_some(Ok(()))
		})
	}

	/// `ch.receive()`: the oldest value offered, waiting for one; None once the channel is closed and empty
	fn take(&self, id: i64, alone: &dyn Fn() -> bool) -> Result<Option<TaskValue>, String> {
		self.wait(id, alone, "ch.receive()", |channel| match channel.offered.pop_front() {
			Some(value) => {
				channel.taken += 1;
				Some(Ok(Some(value)))
			}
			None => channel.closed.then_some(Ok(None)),
		})
	}

	/// `for v in ch`: whether another value comes, waiting until one is offered or the channel is closed
	fn more(&self, id: i64, alone: &dyn Fn() -> bool) -> Result<bool, String> {
		self.wait(id, alone, "for … in ch", |channel| match (channel.offered.is_empty(), channel.closed) {
			(false, _) => Some(Ok(true)),
			(true, closed) => closed.then_some(Ok(false)),
		})
	}

	fn close(&self, id: i64) -> Result<(), String> {
		let mut channels = self.channels.lock().expect("channels");
		let channel = channels.get_mut(&id).ok_or_else(|| format!("no channel {id}"))?;
		if std::mem::replace(&mut channel.closed, true) {
			return Err("close of a closed channel".to_string());
		}
		self.changed.notify_all();
		Ok(())
	}

	pub(super) fn end(&self) {
		let _channels = self.channels.lock().expect("channels");
		self.ended.store(true, Ordering::Relaxed);
		self.changed.notify_all();
	}
}

impl TaskTable {
	/// The local channel words (Channels); the program waiting on a channel while no task runs any more waits forever:
	/// an error, as Go's "all goroutines are asleep"
	pub(super) fn link_channels(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		let opener = self.clone();
		linker.func_wrap(HOST_LIBRARY, CHANNEL_NEW, move || -> i64 { opener.channels.open() })?;
		let sender = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_PUT,
			move |mut caller: Caller<'_, HostState>, id: i64, value: HostNode| -> wasmtime::Result<()> {
				let value = read_in(&mut caller, value).map_err(host_error)?;
				let alone = sender.alone_in(&caller);
				sender.channels.put(id, value, &alone).map_err(task_failure)
			},
		)?;
		let receiver = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_TAKE,
			move |mut caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<HostNode> {
				let alone = receiver.alone_in(&caller);
				let value = receiver.channels.take(id, &alone).map_err(task_failure)?.unwrap_or(TaskValue::Empty);
				build_in(&mut caller, &value)
			},
		)?;
		let looper = self.clone();
		linker.func_wrap(
			HOST_LIBRARY,
			CHANNEL_MORE,
			move |caller: Caller<'_, HostState>, id: i64| -> wasmtime::Result<i64> {
				let alone = looper.alone_in(&caller);
				looper.channels.more(id, &alone).map(i64::from).map_err(task_failure)
			},
		)?;
		let closer = self.clone();
		linker.func_wrap(HOST_LIBRARY, CHANNEL_CLOSE, move |id: i64| -> wasmtime::Result<()> {
			closer.channels.close(id).map_err(task_failure)
		})?;
		Ok(())
	}
}
