//! The words of the program's environment, imported from the "host" module and called like C functions:
//! `sleep(ms)` pauses, `random()` is a float in [0, 1), `random_below(n)` an int in 0..n, `clock()` the milliseconds
//! since the Unix epoch. A program that defines a word of the same name keeps its own. warp adds the words that need
//! its compiler, the network or threads (host.rs); these need nothing.
//! `signal_poll()`, called at each loop start of a program with `on interrupt {…}`, runs that handler (the exported
//! `on·interrupt`) after a ctrl-c (notes/system_signals.md): the first poll installs the SIGINT handler, which only sets
//! a flag; a second ctrl-c within a second ends the run at once (exit code 130), so a handler that ignores ctrl-c
//! never makes a program unstoppable.
pub const HOST_LIBRARY: &str = "host";
pub const SLEEP: &str = "sleep";
pub const RANDOM: &str = "random";
pub const RANDOM_BELOW: &str = "random_below";
pub const CLOCK: &str = "clock";
pub const SIGNAL_POLL: &str = "signal_poll";
/// The exported handler of `on interrupt {…}` (src/lowering/event_signals.rs)
pub const INTERRUPT_HANDLER: &str = "on·interrupt";
/// The words link_host_words provides
pub const BASIC_HOST_WORDS: [&str; 5] = [SLEEP, RANDOM, RANDOM_BELOW, CLOCK, SIGNAL_POLL];

#[cfg(feature = "engine")]
pub use linking::*;

#[cfg(feature = "engine")]
mod linking {
	use super::*;
	use std::time::Duration;
	use wasmtime::{Caller, Linker, Result, Val};

	/// Link sleep, random, random_below and clock, for a store of any state
	pub fn link_host_words<T: 'static>(linker: &mut Linker<T>) -> Result<()> {
		linker.func_wrap(HOST_LIBRARY, SLEEP, |milliseconds: i64| std::thread::sleep(Duration::from_millis(milliseconds.max(0) as u64)))?;
		// the top 53 bits make a uniform f64 in [0, 1)
		linker.func_wrap(HOST_LIBRARY, RANDOM, || (next_random() >> 11) as f64 / (1u64 << 53) as f64)?;
		linker.func_wrap(HOST_LIBRARY, RANDOM_BELOW, |bound: i64| if bound <= 0 { 0 } else { (next_random() % bound as u64) as i64 })?;
		linker.func_wrap(HOST_LIBRARY, CLOCK, milliseconds_since_epoch)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_POLL, |caller: Caller<'_, T>| run_interrupt_handler(caller))?;
		Ok(())
	}

	/// After a ctrl-c the program's on·interrupt runs, with ø as `event` when it reads one
	fn run_interrupt_handler<T>(mut caller: Caller<'_, T>) -> Result<()> {
		if !interrupt::take() {
			return Ok(());
		}
		let Some(handler) = caller.get_export(INTERRUPT_HANDLER).and_then(|export| export.into_func()) else { return Ok(()) };
		let mut arguments = vec![];
		if handler.ty(&caller).params().len() == 1 {
			let empty = caller.get_export("new_empty").and_then(|export| export.into_func()).ok_or_else(|| wasmtime::Error::msg("no new_empty"))?;
			let mut built = [Val::AnyRef(None)];
			empty.call(&mut caller, &[], &mut built)?;
			arguments.push(built[0]);
		}
		let mut results = vec![Val::I64(0); handler.ty(&caller).results().len()];
		handler.call(&mut caller, &arguments, &mut results)
	}

	#[cfg(unix)]
	mod interrupt {
		use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
		use std::sync::Once;

		const EXIT_INTERRUPTED: i32 = 130; // 128 + SIGINT, what a shell reports for a ctrl-c
		/// a second ctrl-c this soon after the one before ends the run
		const HARD_EXIT_WITHIN_MILLISECONDS: i64 = 1000;
		static INTERRUPTED: AtomicBool = AtomicBool::new(false);
		/// when the last ctrl-c came, monotonic milliseconds (0: none yet)
		static LAST_SIGNAL: AtomicI64 = AtomicI64::new(0);
		static WATCHING: Once = Once::new();

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
		/// not yet on this platform: ctrl-c ends the run as before
		pub fn take() -> bool {
			false
		}
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
