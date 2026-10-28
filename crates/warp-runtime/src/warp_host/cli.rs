//! warp-host's command line as warp-rust's: `--fuel <steps>` and WARP_TIME_LIMIT stop a program's run with warp-rust's
//! errors, refusals go to WARP_EMIT_REFUSALS, `version` names the host and the compilers it would use
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use wasmtime::{Engine, Store};

/// The steps `--fuel <steps>` gives a program's run (warp-rust's src/cli/flags.rs; underscores allowed)
static FUEL: OnceLock<u64> = OnceLock::new();
/// The file each refusal goes to as a line, as warp-rust's self_hosted.rs writes it
const REFUSALS_FILE: &str = "WARP_EMIT_REFUSALS";
const OUTSIDE_THE_CORE: &str = "outside the core: ";
/// The custom section a compiler's build carries: its lib/, lib/'s fingerprint, warp's sources, lib/'s commit
const BUILD_SECTION: &str = "warp.build";
const COMMIT_PART: usize = 3;
const SHOWN_COMMIT_LENGTH: usize = 10;
/// How often the time limit's watch looks at the run's computing time
const WATCH_TICK: Duration = Duration::from_millis(10);

/// `--fuel <steps>`: the program's run counts steps instead of running freely
pub fn set_fuel(steps: Option<String>) -> Result<(), String> {
	let parsed = steps.as_deref().and_then(|steps| steps.replace('_', "").parse::<u64>().ok());
	let steps = parsed.ok_or("--fuel needs a number of steps, e.g. warp-host --fuel 100000000000 app.warp")?;
	FUEL.set(steps).map_err(|_| "--fuel is given once".to_string())
}

/// The engine settings of a program's run: fuel metering when `--fuel` asks for it
pub fn configure(config: &mut wasmtime::Config) {
	config.consume_fuel(FUEL.get().is_some());
}

/// The engine the compiler runs on: the program's, unless that one counts fuel (the steps are the program's, and
/// metering would make the compile four times slower)
pub fn compiling_engine(engine: &Engine) -> Engine {
	static UNMETERED: OnceLock<Engine> = OnceLock::new();
	if FUEL.get().is_none() {
		return engine.clone();
	}
	UNMETERED
		.get_or_init(|| {
			let mut config = warp_runtime::engine::deterministic_config();
			config.async_stack_size(crate::WASM_STACK).max_wasm_stack(crate::WASM_STACK);
			Engine::new(&config).unwrap_or_else(|_| engine.clone())
		})
		.clone()
}

/// A new store's fuel, the steps --fuel gives, so what runs before main (a start function, an instantiated module's
/// own) has its steps too: a store of a metered engine starts with none
pub fn fuel<T>(store: &mut Store<T>) {
	if let Some(steps) = FUEL.get() {
		let _ = store.set_fuel(*steps);
	}
}

/// The limits of a run about to start: its fuel, and its time when WARP_TIME_LIMIT sets one (warp-rust's limit of a
/// turn's computing, the CPU time of the thread running it); the watch ends when the returned guard drops
pub fn limit<T>(store: &mut Store<T>) -> Limit {
	fuel(store);
	let done = Arc::new(AtomicBool::new(false));
	if let Some(limit) = warp_runtime::fuel::time_limit_from_environment() {
		let clock = thread_clock();
		let watched = done.clone();
		std::thread::spawn(move || {
			let started = clock();
			while !watched.load(Ordering::Relaxed) {
				std::thread::sleep(WATCH_TICK);
				if clock().saturating_sub(started) > limit {
					stop(&out_of_time(limit));
				}
			}
		});
	}
	Limit(done)
}

pub struct Limit(Arc<AtomicBool>);

impl Drop for Limit {
	fn drop(&mut self) {
		self.0.store(true, Ordering::Relaxed);
	}
}

/// A run that ran out of fuel ends the process with warp-rust's error; any other failure returns
pub fn stop_when_out_of_fuel(error: &wasmtime::Error) {
	if let (Some(wasmtime::Trap::OutOfFuel), Some(steps)) = (error.downcast_ref::<wasmtime::Trap>(), FUEL.get()) {
		stop(&format!(
			"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or warp fuel <steps>)",
			warp_runtime::fuel::FUEL_VARIABLE
		));
	}
}

fn out_of_time(limit: Duration) -> String {
	format!(
		"out of time after {} s of computing in one turn: the program may not terminate (give it more with {}=<seconds>, or count steps with warp fuel <steps>)",
		limit.as_secs_f64(),
		warp_runtime::fuel::TIME_LIMIT_VARIABLE
	)
}

/// The run's error as its only value, as warp-rust ends a run past its limit
fn stop(message: &str) -> ! {
	use std::io::Write;
	println!("{}", crate::Node::Error(message.as_bytes().to_vec()).shown());
	let _ = std::io::stdout().flush();
	std::process::exit(1)
}

/// The computing time of the calling thread, readable from another thread: its Mach thread's user and system time
#[cfg(target_vendor = "apple")]
fn thread_clock() -> impl Fn() -> Duration + Send + 'static {
	// SAFETY: the Mach port of the calling thread, which stays valid while the thread runs
	let thread = unsafe { libc::pthread_mach_thread_np(libc::pthread_self()) };
	let started = std::time::Instant::now();
	move || {
		// SAFETY: a plain C struct, all zeros a valid value
		let mut info: libc::thread_basic_info = unsafe { std::mem::zeroed() };
		let mut count = libc::THREAD_BASIC_INFO_COUNT;
		let info_out = &mut info as *mut libc::thread_basic_info as libc::thread_info_t;
		// SAFETY: fills `info`, of the size `count` gives, about the thread found above
		let read = unsafe { libc::thread_info(thread, libc::THREAD_BASIC_INFO as u32, info_out, &mut count) };
		match read == libc::KERN_SUCCESS {
			true => [info.user_time, info.system_time]
				.iter()
				.map(|time| Duration::new(time.seconds as u64, time.microseconds as u32 * 1000))
				.sum(),
			false => started.elapsed(),
		}
	}
}

/// The computing time of the calling thread, readable from another thread
#[cfg(all(unix, not(target_vendor = "apple")))]
fn thread_clock() -> impl Fn() -> Duration + Send + 'static {
	let mut clock: libc::clockid_t = 0;
	// SAFETY: asks for the CPU clock of the calling thread, written into `clock`
	let found = unsafe { libc::pthread_getcpuclockid(libc::pthread_self(), &mut clock) } == 0;
	let started = std::time::Instant::now();
	move || {
		let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
		// SAFETY: fills the timespec it is given, about the clock found above
		if found && unsafe { libc::clock_gettime(clock, &mut time) } == 0 {
			Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
		} else {
			started.elapsed()
		}
	}
}

/// Without a thread clock: the time since the run started, waiting included
#[cfg(not(unix))]
fn thread_clock() -> impl Fn() -> Duration + Send + 'static {
	let started = std::time::Instant::now();
	move || started.elapsed()
}

/// A refusal's lines in the file WARP_EMIT_REFUSALS names, when set: a run over many programs ranks what keeps them
/// outside the core
pub fn record_refusal(message: &str) {
	use std::io::Write;
	let Some(file) = std::env::var_os(REFUSALS_FILE) else { return };
	let lines: String =
		message.lines().filter(|line| line.starts_with(OUTSIDE_THE_CORE)).map(|line| format!("{line}\n")).collect();
	if lines.is_empty() {
		return;
	}
	let appended = std::fs::OpenOptions::new().create(true).append(true).open(file);
	let _ = appended.and_then(|mut file| file.write_all(lines.as_bytes()));
}

/// `warp-host version`: the host, and the compilers a program of the current folder compiles with, as `warp version`
/// names them (`warp.wasm of <commit>, warp-self.wasm missing`)
pub fn version() -> i32 {
	println!("🌀 warp-host {}", env!("CARGO_PKG_VERSION"));
	let here = std::env::current_dir().unwrap_or_default().join("program.warp");
	let lib = crate::library_for(&here);
	let stamp = |name: &str| {
		let file = lib.as_deref().and_then(|lib| crate::compiler_file(lib, name));
		let bytes = file.as_ref().and_then(|file| std::fs::read(file).ok());
		match bytes {
			Some(bytes) => match build_commit(&bytes) {
				commit if commit.is_empty() => format!("{name} of no commit"),
				commit => format!("{name} of {}", &commit[..commit.len().min(SHOWN_COMMIT_LENGTH)]),
			},
			None => format!("{name} missing (warp rebuild {})", if name == "warp.wasm" { "pure" } else { "self" }),
		}
	};
	println!("{}, {}", stamp("warp.wasm"), stamp("warp-self.wasm"));
	0
}

/// The commit a compiler's build section names, "" when it names none
fn build_commit(bytes: &[u8]) -> String {
	let section = crate::ffi::custom_sections(bytes, BUILD_SECTION).next().unwrap_or_default();
	String::from_utf8_lossy(section).split('\n').nth(COMMIT_PART).unwrap_or_default().to_string()
}
