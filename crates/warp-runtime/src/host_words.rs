//! The words of the program's environment, imported from the "host" module and called like C functions:
//! `sleep(ms)` pauses, `random()` is a float in [0, 1), `random_below(n)` an int in 0..n, `clock()` the milliseconds
//! since the Unix epoch. A program that defines a word of the same name keeps its own. warp adds the words that need
//! its compiler, the network or threads (host.rs); these need nothing.
//! `signal_poll()` and `signal_every(id, ms)` serve the handlers of system signals (system_signals.rs).
pub const HOST_LIBRARY: &str = "host";
pub const SLEEP: &str = "sleep";
pub const RANDOM: &str = "random";
pub const RANDOM_BELOW: &str = "random_below";
pub const CLOCK: &str = "clock";
pub const SIGNAL_POLL: &str = "signal_poll";
pub const SIGNAL_EVERY: &str = "signal_every";
/// `signal_daily(id, minute_of_day, weekdays)`: `on every day at 9:00 {…}`, the timer on·every·id at that local time
/// on the weekdays of the mask (bit 0 Sunday … bit 6 Saturday, 127 every day)
pub const SIGNAL_DAILY: &str = "signal_daily";
/// `signal_at(id, minute_of_day)`: `at 9:00 {…}`, on·every·id once at the next such local time
pub const SIGNAL_AT: &str = "signal_at";
pub const SIGNAL_WATCH: &str = "signal_watch";
/// `system_value(name)`: the machine's `battery` percent, `charging`, `online`, `dark mode` (system_values.rs)
pub const SYSTEM_VALUE: &str = "system_value";
pub const BATTERY: &str = "battery";
pub const CHARGING: &str = "charging";
pub const ONLINE: &str = "online";
pub const DARK_MODE: &str = "dark mode";
/// The system values, and whether each is a yes/no value
/// How often the clipboard changed since the machine started (macOS NSPasteboard changeCount): `on clipboard change`
pub const CLIPBOARD_COUNT: &str = "clipboard count";
/// The pointer over the page's canvas, in canvas pixels, and whether a button is down (card drawing-frames)
pub const MOUSE_X: &str = "mouse_x";
pub const MOUSE_Y: &str = "mouse_y";
pub const MOUSE_DOWN: &str = "mouse_down";
pub const SYSTEM_VALUES: [(&str, bool); 8] = [(BATTERY, false), (CHARGING, true), (ONLINE, true), (DARK_MODE, true), (CLIPBOARD_COUNT, false),
	(MOUSE_X, false), (MOUSE_Y, false), (MOUSE_DOWN, true)];
/// `clipboard`: the clipboard's text, read only when the program reads it (host.rs clipboard_text)
pub const CLIPBOARD: &str = "clipboard";
pub const CLIPBOARD_TEXT: &str = "clipboard_text";
/// `on·file·0`: the handler of the first `on file "x" change {…}`
pub const FILE_HANDLER_PREFIX: &str = "on·file·";
/// `exit(code)` ends the run, not the process (P121, system_signals.rs ExitRequest)
pub const EXIT: &str = "exit";
/// The exported handler of `on interrupt {…}` (src/lowering/event_signals.rs)
pub const INTERRUPT_HANDLER: &str = "on·interrupt";
/// The exported handler of `on exit {…}`, run once as the run ends (system_signals.rs run_exit_handler)
pub const EXIT_HANDLER: &str = "on·exit";
/// `on·every·0`: the handler of the first `on every … {…}`
pub const TIMER_HANDLER_PREFIX: &str = "on·every·";
/// `on·fetch·0`: the handler run when the reply of the first `users := fetch url` arrived (src/fetches.rs)
pub const FETCH_HANDLER_PREFIX: &str = "on·fetch·";
/// The checks of the listeners on shared values (P106), polled at every check point (lowering/signal_values.rs)
pub const SHARED_HANDLER: &str = "on·shared";
/// The words link_host_words provides
pub const BASIC_HOST_WORDS: [&str; 11] = [SLEEP, RANDOM, RANDOM_BELOW, CLOCK, SIGNAL_POLL, SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT, SIGNAL_WATCH, SYSTEM_VALUE, EXIT];

#[cfg(feature = "engine")]
pub use linking::*;

#[cfg(feature = "engine")]
mod linking {
	use super::*;
	use std::time::Duration;
	use crate::system_signals;
	use wasmtime::{Caller, Func, Linker, Result};

	/// Link sleep, random, random_below and clock, for a store of any state
	pub fn link_host_words<T: 'static>(linker: &mut Linker<T>) -> Result<()> {
		system_signals::forget_timers(); // each run links anew, on its own thread
		linker.func_wrap(HOST_LIBRARY, SLEEP, |mut caller: Caller<'_, T>, milliseconds: i64| {
			system_signals::sleep_with_handlers(&mut caller, Duration::from_millis(milliseconds.max(0) as u64), exported)
		})?;
		// the top 53 bits make a uniform f64 in [0, 1)
		linker.func_wrap(HOST_LIBRARY, RANDOM, || (next_random() >> 11) as f64 / (1u64 << 53) as f64)?;
		linker.func_wrap(HOST_LIBRARY, RANDOM_BELOW, |bound: i64| if bound <= 0 { 0 } else { (next_random() % bound as u64) as i64 })?;
		linker.func_wrap(HOST_LIBRARY, CLOCK, milliseconds_since_epoch)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_POLL, |mut caller: Caller<'_, T>| system_signals::run_due_handlers(&mut caller, exported))?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_EVERY, system_signals::start_timer)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_DAILY, system_signals::start_daily_timer)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_AT, system_signals::start_one_shot_timer)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_WATCH, |mut caller: Caller<'_, T>, id: i64, path: i32| -> Result<()> {
			let path = c_string(&mut caller, path)?;
			system_signals::watch_file(id, path);
			Ok(())
		})?;
		linker.func_wrap(HOST_LIBRARY, SYSTEM_VALUE, |mut caller: Caller<'_, T>, name: i32| -> Result<i64> {
			let name = c_string(&mut caller, name)?;
			crate::system_values::read(&name).map_err(wasmtime::Error::msg)
		})?;
		linker.func_wrap(HOST_LIBRARY, EXIT, |code: i64| -> Result<()> { Err(wasmtime::Error::new(system_signals::ExitRequest(code as i32))) })?;
		Ok(())
	}

	/// The NUL-terminated text a text literal passes as an i32 (the emitter's C strings)
	fn c_string<T>(caller: &mut Caller<'_, T>, pointer: i32) -> Result<String> {
		let memory = caller.get_export("memory").and_then(|export| export.into_memory()).ok_or_else(|| wasmtime::Error::msg("no memory export"))?;
		let bytes = memory.data(&*caller);
		let start = (pointer as usize).min(bytes.len());
		let end = bytes[start..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |length| start + length);
		Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
	}

	fn exported<T>(caller: &mut Caller<'_, T>, name: &str) -> Option<Func> {
		caller.get_export(name).and_then(|export| export.into_func())
	}

	/// xorshift64*, seeded from the clock once per process: random enough for games and samples, not for secrets
	fn next_random() -> u64 {
		use std::sync::atomic::{AtomicU64, Ordering};
		static STATE: AtomicU64 = AtomicU64::new(0);
		let mut x = STATE.load(Ordering::Relaxed);
		if x == 0 {
			x = (milliseconds_since_epoch() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
		}
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		STATE.store(x, Ordering::Relaxed);
		x.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	fn milliseconds_since_epoch() -> i64 {
		std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as i64)
	}
}
