//! System signals (notes/system_signals.md): handlers of events from outside the program, exported functions the
//! runtime calls at check points: `signal_poll()` (at the start and end of main and at each loop start of a program
//! with such handlers) and `sleep`, which wakes for them. `on interrupt {…}` is `on·interrupt`, run after a ctrl-c:
//! the first poll installs the SIGINT handler, which only sets a flag; a second ctrl-c within a second, or before the
//! handler ran, ends the run at once (exit code 130), so a handler that ignores ctrl-c never makes a program
//! unstoppable. `on every 5 seconds {…}` is `on·every·0`, started by `signal_every(0, 5000)` where the handler is
//! declared. `on file "notes.txt" change {…}` is `on·file·0`, started by `signal_watch(0, "notes.txt")`: a timer of
//! FILE_CHECK_PERIOD that fires only when the file's modification time or size changed (it appeared, was written, or went).
//! A run that allows it (`warp run`, built executables) stays after main while a timer or watch lives.
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use crate::host_words::{EXIT_HANDLER, FILE_HANDLER_PREFIX, INTERRUPT_HANDLER, SHARED_HANDLER, TIMER_HANDLER_PREFIX};
use std::path::PathBuf;
use std::time::SystemTime;
use wasmtime::{AsContextMut, Func, Instance, Result, Store, Val};

const NEW_EMPTY: &str = "new_empty";
const NEW_INT: &str = "new_int";
/// How often a watched file's modification time is read: polling, no file-system events yet
const FILE_CHECK_PERIOD: Duration = Duration::from_millis(100);
/// How often a sleeping program with listeners on shared values (on·shared) looks at them
const SHARED_CHECK_PERIOD: Duration = Duration::from_millis(10);

struct Timer {
	handler: String,
	repeat: Repeat,
	/// None once a one-shot timer fired
	due: Option<Instant>,
	/// a watched file and its stamp when last read: the handler runs only when that changed
	watched: Option<(PathBuf, Option<FileStamp>)>,
}

#[derive(Clone, Copy)]
enum Repeat {
	Every(Duration),
	/// a local time of day on the weekdays of the mask (bit 0 Sunday), every week or only `once`
	Clock { minute_of_day: i64, weekdays: i64, once: bool },
}

impl Repeat {
	/// The first time from `now`: a clock is read anew each time, so a change of the local time (summer time) counts
	fn first(self, now: Instant) -> Instant {
		match self {
			Repeat::Every(period) => now + period,
			Repeat::Clock { minute_of_day, weekdays, .. } => {
				let (weekday, second_of_day) = local_time();
				now + Duration::from_secs(seconds_until_on(minute_of_day, weekdays, weekday, second_of_day) as u64)
			}
		}
	}

	/// The time after the one `due` fired at `now` (one call however late); None after a one-shot
	fn after(self, due: Instant, now: Instant) -> Option<Instant> {
		match self {
			Repeat::Every(period) => Some((due + period).max(now)),
			Repeat::Clock { once: true, .. } => None,
			Repeat::Clock { .. } => Some(self.first(now)),
		}
	}

	/// `every 5 seconds`, `every monday at 9:00`, `at 9:00`
	fn spoken(self) -> String {
		match self {
			Repeat::Every(period) => format!("every {}", spoken(period)),
			Repeat::Clock { minute_of_day, once: true, .. } => format!("at {}", clock_time(minute_of_day)),
			Repeat::Clock { minute_of_day, weekdays, once: false } => format!("every {} at {}", spoken_days(weekdays), clock_time(minute_of_day)),
		}
	}
}

const WEEKDAY_NAMES: [&str; 7] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
pub const EVERY_DAY: i64 = 0b111_1111;

fn spoken_days(weekdays: i64) -> String {
	if weekdays & EVERY_DAY == EVERY_DAY {
		return "day".to_string();
	}
	WEEKDAY_NAMES.iter().enumerate().filter(|(day, _)| weekdays & (1 << day) != 0).map(|(_, name)| *name).collect::<Vec<_>>().join(" and ")
}

fn clock_time(minute_of_day: i64) -> String {
	format!("{}:{:02}", minute_of_day / 60, minute_of_day % 60)
}

fn add_timer(handler: String, repeat: Repeat, watched: Option<(PathBuf, Option<FileStamp>)>) {
	let timer = Timer { handler, repeat, due: Some(repeat.first(Instant::now())), watched };
	TIMERS.with(|timers| timers.borrow_mut().push(timer));
}

thread_local! {
	/// the timers of the run on this thread
	static TIMERS: RefCell<Vec<Timer>> = const { RefCell::new(Vec::new()) };
}
static STAYING_ALLOWED: AtomicBool = AtomicBool::new(false);

thread_local! {
	/// the code of the `exit` that ended the last run on this thread
	static EXIT_CODE: Cell<Option<i32>> = const { Cell::new(None) };
}

/// `exit(code)` (P121): the error that unwinds the run; the runner ends the run with ø and keeps the code
#[derive(Debug)]
pub struct ExitRequest(pub i32);

impl std::fmt::Display for ExitRequest {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "exit({})", self.0)
	}
}

impl std::error::Error for ExitRequest {}

/// Whether the run ended by `exit`: the code is kept for take_exit_code
pub fn ended_by_exit<R>(outcome: &Result<R>) -> bool {
	let code = outcome.as_ref().err().and_then(|error| error.downcast_ref::<ExitRequest>()).map(|request| request.0);
	code.inspect(|code| EXIT_CODE.with(|kept| kept.set(Some(*code)))).is_some()
}

/// The code of the `exit` that ended the last run on this thread (once)
pub fn take_exit_code() -> Option<i32> {
	EXIT_CODE.with(Cell::take)
}

/// A new run on this thread starts without timers
pub fn forget_timers() {
	TIMERS.with(|timers| timers.borrow_mut().clear());
}

/// `signal_every(id, milliseconds)`: the handler on·every·id runs every that many milliseconds from now on
pub fn start_timer(id: i64, milliseconds: i64) {
	add_timer(format!("{TIMER_HANDLER_PREFIX}{id}"), Repeat::Every(Duration::from_millis(milliseconds.max(1) as u64)), None);
}

/// `signal_daily(id, minute_of_day, weekdays)`: the handler on·every·id runs at that local time of day on the
/// weekdays of the mask (bit 0 Sunday … bit 6 Saturday)
pub fn start_daily_timer(id: i64, minute_of_day: i64, weekdays: i64) {
	add_timer(format!("{TIMER_HANDLER_PREFIX}{id}"), Repeat::Clock { minute_of_day, weekdays, once: false }, None);
}

/// `signal_at(id, minute_of_day)`: the handler on·every·id runs once, at the next such local time of day
pub fn start_one_shot_timer(id: i64, minute_of_day: i64) {
	add_timer(format!("{TIMER_HANDLER_PREFIX}{id}"), Repeat::Clock { minute_of_day, weekdays: EVERY_DAY, once: true }, None);
}

const DAY: Duration = Duration::from_secs(24 * 3600);
const WEEK_DAYS: i64 = 7;

/// Seconds from `second_of_day` to the next `minute_of_day` (a whole day when it is now)
pub fn seconds_until(minute_of_day: i64, second_of_day: i64) -> i64 {
	seconds_until_on(minute_of_day, EVERY_DAY, 0, second_of_day)
}

/// Seconds from `second_of_day` of `weekday` (0 Sunday) to the next `minute_of_day` on a weekday of the mask (a whole
/// week when that is now and the mask has only this day); an empty mask is every day
pub fn seconds_until_on(minute_of_day: i64, weekdays: i64, weekday: i64, second_of_day: i64) -> i64 {
	let day = DAY.as_secs() as i64;
	let weekdays = if weekdays & EVERY_DAY == 0 { EVERY_DAY } else { weekdays };
	let today = minute_of_day * 60 - second_of_day;
	(0..=WEEK_DAYS)
		.map(|days_ahead| (days_ahead, today + days_ahead * day))
		.find(|(days_ahead, seconds)| *seconds > 0 && weekdays & (1 << ((weekday + days_ahead) % WEEK_DAYS)) != 0)
		.map_or(WEEK_DAYS * day, |(_, seconds)| seconds)
}

/// The local weekday (0 Sunday) and seconds since local midnight (UTC where the platform gives no time zone)
fn local_time() -> (i64, i64) {
	let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
	#[cfg(unix)]
	{
		let time = now as libc::time_t;
		let mut local: libc::tm = unsafe { std::mem::zeroed() };
		if !unsafe { libc::localtime_r(&time, &mut local) }.is_null() {
			return (local.tm_wday as i64, (local.tm_hour * 3600 + local.tm_min * 60 + local.tm_sec) as i64);
		}
	}
	let day = DAY.as_secs() as i64;
	const THURSDAY: i64 = 4; // 1970-01-01
	((now.div_euclid(day) + THURSDAY) % WEEK_DAYS, now.rem_euclid(day))
}

/// `signal_watch(id, path)`: the handler on·file·id runs when the file changes from now on
pub fn watch_file(id: i64, path: String) {
	let path = PathBuf::from(path);
	let seen = stamp(&path);
	add_timer(format!("{FILE_HANDLER_PREFIX}{id}"), Repeat::Every(FILE_CHECK_PERIOD), Some((path, seen)));
}

/// Modification time and size: a rewrite within one tick of a coarse clock still changes the size, mostly
type FileStamp = (SystemTime, u64);

fn stamp(path: &PathBuf) -> Option<FileStamp> {
	let metadata = std::fs::metadata(path).ok()?;
	Some((metadata.modified().ok()?, metadata.len()))
}

fn next_due() -> Option<Instant> {
	TIMERS.with(|timers| timers.borrow().iter().filter_map(|timer| timer.due).min())
}

/// The handlers to run now: on·interrupt after a ctrl-c (watched only by a program with that handler: any other
/// ends at a ctrl-c as usual), then each due timer's (rescheduled; one call however late)
fn due_handlers(handles_interrupt: bool) -> Vec<String> {
	let mut due = vec![];
	if handles_interrupt && interrupt::take() {
		due.push(INTERRUPT_HANDLER.to_string());
	}
	let now = Instant::now();
	TIMERS.with(|timers| {
		let mut timers = timers.borrow_mut();
		for timer in timers.iter_mut() {
			let Some(when) = timer.due.filter(|when| *when <= now) else { continue };
			timer.due = timer.repeat.after(when, now);
			if let Some((path, seen)) = &mut timer.watched {
				let current = stamp(path);
				if current == *seen {
					continue;
				}
				*seen = current;
			}
			due.push(timer.handler.clone());
		}
		timers.retain(|timer| timer.due.is_some());
	});
	due
}

/// Run the due handlers, then the checks of the listeners on shared values (each check point looks at them);
/// `export` finds a function of the instance by name
pub fn run_due_handlers<S: AsContextMut>(store: &mut S, mut export: impl FnMut(&mut S, &str) -> Option<Func>) -> Result<()> {
	let handles_interrupt = export(store, INTERRUPT_HANDLER).is_some();
	let shared_checks = export(store, SHARED_HANDLER).map(|_| SHARED_HANDLER.to_string());
	for name in due_handlers(handles_interrupt).into_iter().chain(shared_checks) {
		let Some(handler) = export(store, &name) else { continue };
		call_handler(store, handler, None, &mut export)?;
	}
	Ok(())
}

fn call_handler<S: AsContextMut>(store: &mut S, handler: Func, event: Option<i64>, export: &mut impl FnMut(&mut S, &str) -> Option<Func>) -> Result<()> {
	let mut arguments = vec![];
	// a handler that reads `event` gets it (the exit code of `on exit`, P134), as a number or a Node; else ø
	if let Some(parameter) = handler.ty(&*store).params().next() {
		arguments.push(match (parameter, event) {
			(wasmtime::ValType::I64, event) => Val::I64(event.unwrap_or(0)),
			(_, Some(event)) => node_of(store, export, NEW_INT, &[Val::I64(event)])?,
			(_, None) => node_of(store, export, NEW_EMPTY, &[])?,
		});
	}
	let mut results = vec![Val::I64(0); handler.ty(&*store).results().len()];
	handler.call(&mut *store, &arguments, &mut results)
}

/// The Node a constructor export of the program builds (`new_empty()`, `new_int(n)`)
fn node_of<S: AsContextMut>(store: &mut S, export: &mut impl FnMut(&mut S, &str) -> Option<Func>, constructor: &str, arguments: &[Val]) -> Result<Val> {
	let function = export(store, constructor).ok_or_else(|| wasmtime::Error::msg(format!("no {constructor}")))?;
	let mut built = [Val::AnyRef(None)];
	function.call(&mut *store, arguments, &mut built)?;
	Ok(built[0])
}

/// `on exit {…}`: the run ends (main returned, the timers stopped, or `exit(code)`), so on·exit runs once; an `exit`
/// inside it ends it with that code (ended_by_exit keeps it)
pub fn run_exit_handler<S: AsContextMut>(store: &mut S, mut export: impl FnMut(&mut S, &str) -> Option<Func>) -> Result<()> {
	let Some(handler) = export(store, EXIT_HANDLER) else { return Ok(()) };
	let code = EXIT_CODE.with(Cell::get).unwrap_or(0);
	call_handler(store, handler, Some(code.into()), &mut export)
}

/// The outcome of a run and its exit handler: the handler runs after a normal end and after `exit(code)` (whose code
/// stays, unless the handler exits with its own), never after a failure
pub fn with_exit_handler<S: AsContextMut, R>(outcome: Result<R>, store: &mut S, export: impl FnMut(&mut S, &str) -> Option<Func>) -> Result<Option<R>> {
	let exited = ended_by_exit(&outcome);
	let value = match outcome {
		Ok(value) => Some(value),
		Err(_) if exited => None,
		Err(error) => return Err(error),
	};
	let handled = run_exit_handler(store, export);
	if ended_by_exit(&handled) {
		return Ok(None);
	}
	handled?;
	Ok(value)
}

/// Sleep for `duration`, running the handlers that come due on the way; a ctrl-c ends the sleep early
pub fn sleep_with_handlers<S: AsContextMut>(store: &mut S, duration: Duration, mut export: impl FnMut(&mut S, &str) -> Option<Func>) -> Result<()> {
	let end = Instant::now() + duration;
	let checks_shared = export(store, SHARED_HANDLER).is_some();
	loop {
		let wake = next_due().map_or(end, |due| due.min(end));
		let wake = if checks_shared { wake.min(Instant::now() + SHARED_CHECK_PERIOD) } else { wake };
		let interrupted = interrupt::wait_until(wake);
		run_due_handlers(store, &mut export)?;
		if interrupted || Instant::now() >= end {
			return Ok(());
		}
	}
}

/// `warp run` and built executables: a program may stay after main while it listens
pub fn allow_staying() {
	STAYING_ALLOWED.store(true, Ordering::SeqCst);
}

/// `5 seconds`, `1 second`, `200 ms`
fn spoken(period: Duration) -> String {
	match (period.as_millis(), period.as_millis() % 1000) {
		(1000, _) => "1 second".to_string(),
		(milliseconds, 0) => format!("{} seconds", milliseconds / 1000),
		(milliseconds, _) => format!("{milliseconds} ms"),
	}
}

/// After main: while a timer lives (and the run allows it), wait for the next due handler and run it
pub fn stay_while_listening<T>(store: &mut Store<T>, instance: &Instance) -> Result<()> {
	if !STAYING_ALLOWED.load(Ordering::SeqCst) || next_due().is_none() {
		return Ok(());
	}
	let listening: Vec<String> = TIMERS.with(|timers| timers.borrow().iter().map(|timer| match &timer.watched {
		Some((path, _)) => format!("file {} change", path.display()),
		None => timer.repeat.spoken(),
	}).collect());
	eprintln!("listening: {} (ctrl-c to stop)", listening.join(", "));
	while let Some(due) = next_due() {
		interrupt::wait_until(due);
		run_due_handlers(store, |store, name| instance.get_func(&mut *store, name))?;
	}
	Ok(())
}

#[cfg(unix)]
mod interrupt {
	use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
	use std::sync::Once;
	use std::time::{Duration, Instant};

	const EXIT_INTERRUPTED: i32 = 130; // 128 + SIGINT, what a shell reports for a ctrl-c
	/// a second ctrl-c this soon after the one before ends the run
	const HARD_EXIT_WITHIN_MILLISECONDS: i64 = 1000;
	/// How often a sleep of a watching program looks for a ctrl-c
	const SLEEP_SLICE: Duration = Duration::from_millis(10);
	static INTERRUPTED: AtomicBool = AtomicBool::new(false);
	/// when the last ctrl-c came, monotonic milliseconds (0: none yet)
	static LAST_SIGNAL: AtomicI64 = AtomicI64::new(0);
	static WATCHING: Once = Once::new();

	/// Sleep until `wake`; a watching program wakes early at a ctrl-c: whether one came
	pub fn wait_until(wake: Instant) -> bool {
		if !WATCHING.is_completed() {
			std::thread::sleep(wake.saturating_duration_since(Instant::now()));
			return false;
		}
		while !INTERRUPTED.load(Ordering::SeqCst) {
			let left = wake.saturating_duration_since(Instant::now());
			if left.is_zero() {
				return false;
			}
			std::thread::sleep(left.min(SLEEP_SLICE));
		}
		true
	}

	/// Whether a ctrl-c came since the last call; the first call starts watching
	pub fn take() -> bool {
		WATCHING.call_once(|| unsafe {
			libc::signal(libc::SIGINT, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
		});
		INTERRUPTED.swap(false, Ordering::SeqCst)
	}

	/// async-signal-safe (atomics, clock_gettime, _exit): a flag, or the exit of a second ctrl-c within a second or
	/// one the program did not take yet
	extern "C" fn on_signal(_: libc::c_int) {
		let now = monotonic_milliseconds();
		let last = LAST_SIGNAL.swap(now, Ordering::SeqCst);
		let soon_after = last != 0 && now - last < HARD_EXIT_WITHIN_MILLISECONDS;
		if INTERRUPTED.swap(true, Ordering::SeqCst) || soon_after {
			unsafe { libc::_exit(EXIT_INTERRUPTED) }
		}
	}

	fn monotonic_milliseconds() -> i64 {
		let mut now = libc::timespec { tv_sec: 0, tv_nsec: 0 };
		unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) };
		now.tv_sec as i64 * 1000 + now.tv_nsec as i64 / 1_000_000
	}
}

#[cfg(not(unix))]
mod interrupt {
	use std::time::Instant;

	/// not yet on this platform (card signals-system-windows): ctrl-c ends the run as before
	pub fn wait_until(wake: Instant) -> bool {
		std::thread::sleep(wake.saturating_duration_since(Instant::now()));
		false
	}

	pub fn take() -> bool {
		false
	}
}
