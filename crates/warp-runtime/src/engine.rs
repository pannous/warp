use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasmtime::{Config, Engine, Store};

/// The GC heap a run starts with: reserved, committed lazily by the OS; a heap starting empty collects (walking every
/// frame of a deep recursion) at each of its many small growths
const GC_HEAP_INITIAL_BYTES: u64 = 1 << 30;
/// The main thread's stack kept for the host when wasm gets the rest (`give_main_stack_to_wasm`): the compiler and host
/// words run below the wasm frames
const HOST_STACK_BYTES: usize = 16 << 20;

/// The wasm call stack of engines made from now on; 0 keeps wasmtime's default (512 KiB), safe on any thread
static WASM_STACK_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Engine settings every warp run shares: GC and typed function references. NaNs are canonicalized where they are
/// observed (floats::canonical_nan, P119), not after every float operation, which doubled the cost of float loops.
/// Machine code compiled with these settings (`warp compile --aot`, `warp build`) loads into any engine made
/// from them, with or without a compiler.
pub fn deterministic_config() -> Config {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	config.gc_heap_initial_size(GC_HEAP_INITIAL_BYTES);
	// warp runs core modules, no components: the machine code then also loads into a runtime built without the
	// component model (the warp-runtime stub), where it is off anyway
	#[cfg(feature = "compiler")]
	config.wasm_component_model(false);
	config.concurrency_support(false);
	let wasm_stack = wasm_stack_bytes();
	if wasm_stack > 0 {
		// wasmtime wants an async stack at least as large; warp makes no async calls, so none is allocated
		config.async_stack_size(wasm_stack).max_wasm_stack(wasm_stack);
	}
	config
}

/// The wasm call stack engines get, 0 for wasmtime's default: a thread running wasm of such an engine needs a stack
/// this large besides its own frames (the self-hosted stage workers, src/compiler/self_hosted.rs)
pub fn wasm_stack_bytes() -> usize {
	WASM_STACK_BYTES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Called on the main thread before any engine is made: wasm runs on the main thread and may use its stack but
/// HOST_STACK_BYTES, a million frames instead of about 15000 (card deep-recursion). Other threads (tests, tasks)
/// keep the default, which fits their smaller stacks.
pub fn give_main_stack_to_wasm() {
	let wasm_stack = main_stack_bytes().saturating_sub(HOST_STACK_BYTES);
	if wasm_stack > 512 << 10 {
		WASM_STACK_BYTES.store(wasm_stack, std::sync::atomic::Ordering::Relaxed);
	}
}

#[cfg(target_os = "macos")]
fn main_stack_bytes() -> usize {
	// SAFETY: asks about the calling thread only
	unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn main_stack_bytes() -> usize {
	let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
	// SAFETY: fills the limit it is given
	let known = unsafe { libc::getrlimit(libc::RLIMIT_STACK, &mut limit) } == 0;
	match known {
		true if limit.rlim_cur != libc::RLIM_INFINITY => limit.rlim_cur as usize,
		_ => 0,
	}
}

#[cfg(not(unix))]
fn main_stack_bytes() -> usize {
	0
}

/// deterministic_config with epoch interruption, where a run checks its time limit (and a task whether to stop), and
/// with fuel metering when `fuel`: the settings of warp's engines. Epoch checks cost a few percent, fuel about four
/// times on calls, so only runs that ask for it count steps (decision fuel-metering).
pub fn metered_config(fuel: bool) -> Config {
	let mut config = deterministic_config();
	config.epoch_interruption(true);
	config.consume_fuel(fuel);
	config
}

/// How often a ticking engine's epoch advances: a run passes its time limit by at most about this much
pub const EPOCH_TICK: Duration = Duration::from_millis(10);

/// `engine` with a thread advancing its epoch every EPOCH_TICK for the rest of the process: its stores check their
/// time limit at each tick. For engines that live as long as the process (shared or standalone).
pub fn ticking(engine: Engine) -> Engine {
	let ticked = engine.clone();
	std::thread::spawn(move || loop {
		std::thread::sleep(EPOCH_TICK);
		ticked.increment_epoch();
	});
	engine
}

/// A store whose run stops: with `fuel` steps on an engine that meters fuel, else after `time` of computing in one turn
/// (each call from the host into the program: main, a frame, an event handler, so a game or a server runs for as long as
/// it likes); its memories and GC heap capped (`cap_memory`)
pub fn limited_store<T: 'static>(engine: &Engine, data: T, fuel: u64, time: Duration) -> Store<T> {
	checked_store(engine, data, fuel, time, || Ok(()))
}

/// limited_store that also runs `checkpoint` at every epoch tick (a task's stop and pause), failing the run when it
/// fails
pub fn checked_store<T: 'static>(
	engine: &Engine,
	data: T,
	fuel: u64,
	time: Duration,
	mut checkpoint: impl FnMut() -> wasmtime::Result<()> + Send + Sync + 'static,
) -> Store<T> {
	let mut store = Store::new(engine, data);
	let mut clock = store.set_fuel(fuel).is_err().then(|| ComputingClock::new(time));
	if let Some(clock) = &clock {
		let new_turn = clock.new_turn.clone();
		store.call_hook(move |_, hook| {
			if matches!(hook, wasmtime::CallHook::CallingWasm) {
				new_turn.store(true, Ordering::Relaxed);
			}
			Ok(())
		});
	}
	store.epoch_deadline_callback(move |_| {
		checkpoint()?;
		if let Some(clock) = clock.as_mut() {
			clock.check()?;
		}
		Ok(wasmtime::UpdateDeadline::Continue(1))
	});
	store.set_epoch_deadline(1);
	cap_memory(&mut store);
	store
}

/// The failure of a run past its time limit
#[derive(Debug)]
pub struct OutOfTime(pub Duration);

impl std::fmt::Display for OutOfTime {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let variable = crate::fuel::TIME_LIMIT_VARIABLE;
		write!(f, "out of time after {:?} of computing in one turn (give it more with {variable}=<seconds>)", self.0)
	}
}

impl std::error::Error for OutOfTime {}

/// The computing time of a turn, counted from its first check: the CPU time of the thread running it
struct ComputingClock {
	limit: Duration,
	started: Option<Duration>,
	/// set when the host calls into the program: the next check starts counting again
	new_turn: Arc<AtomicBool>,
}

impl ComputingClock {
	fn new(limit: Duration) -> Self {
		ComputingClock { limit, started: None, new_turn: Arc::default() }
	}

	fn check(&mut self) -> wasmtime::Result<()> {
		let now = thread_cpu_time();
		if self.new_turn.swap(false, Ordering::Relaxed) {
			self.started = None;
		}
		match now.saturating_sub(*self.started.get_or_insert(now)) > self.limit {
			true => Err(OutOfTime(self.limit).into()),
			false => Ok(()),
		}
	}
}

#[cfg(unix)]
fn thread_cpu_time() -> Duration {
	let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
	// SAFETY: fills the timespec it is given, about the calling thread
	unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
	Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

/// Without a thread clock: the time since the process's first check, waiting included
#[cfg(not(unix))]
fn thread_cpu_time() -> Duration {
	static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
	START.get_or_init(std::time::Instant::now).elapsed()
}

/// The size any one memory or GC heap of a run may grow to: a runaway program stops with an error instead of
/// swapping the machine to death (a 154 GB process took the Mac down, 2026-10-09)
pub const DEFAULT_MEMORY_CAP_MB: usize = 2048;
/// Environment variable that overrides `DEFAULT_MEMORY_CAP_MB`
pub const MEMORY_CAP_VARIABLE: &str = "WARP_MEMORY_CAP_MB";
const BYTES_PER_MB: usize = 1 << 20;

/// The cap in bytes: `WARP_MEMORY_CAP_MB` if it is set to a number, else the default
pub fn memory_cap_bytes() -> usize {
	static CAP: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
	*CAP.get_or_init(|| {
		let megabytes = std::env::var(MEMORY_CAP_VARIABLE).ok().and_then(|value| value.trim().parse().ok());
		megabytes.unwrap_or(DEFAULT_MEMORY_CAP_MB).saturating_mul(BYTES_PER_MB)
	})
}

/// Every growth of the store's linear memories and its GC heap (wasmtime grows the GC heap's memory through the
/// same limiter) past `memory_cap_bytes` fails the run with an error naming the cap
pub fn cap_memory<T>(store: &mut Store<T>) {
	// a zero-sized limiter: leaking its box allocates nothing
	store.limiter(|_| Box::leak(Box::new(MemoryCap)));
}

struct MemoryCap;

impl wasmtime::ResourceLimiter for MemoryCap {
	fn memory_growing(&mut self, _current: usize, desired: usize, maximum: Option<usize>) -> wasmtime::Result<bool> {
		let cap = memory_cap_bytes();
		if desired > cap {
			let refusal = format!(
				"memory cap: growing a memory to {} MB passes the cap of {} MB ({MEMORY_CAP_VARIABLE})",
				desired / BYTES_PER_MB,
				cap / BYTES_PER_MB
			);
			// also on stderr: wasmtime drops this error for the GC heap and traps with "GC heap out of memory"
			eprintln!("{refusal}");
			wasmtime::bail!(refusal);
		}
		Ok(maximum.is_none_or(|maximum| desired <= maximum))
	}

	fn table_growing(&mut self, _current: usize, desired: usize, maximum: Option<usize>) -> wasmtime::Result<bool> {
		Ok(maximum.is_none_or(|maximum| desired <= maximum))
	}
}
