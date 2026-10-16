use std::cell::Cell;
use std::thread::LocalKey;
use std::time::Duration;
use std::{env, fs};
#[cfg(feature = "native")]
use wasmtime::{Engine, Store};

#[cfg(feature = "native")]
pub use warp_runtime::engine::deterministic_config;
pub use warp_runtime::fuel::{DEFAULT_FUEL, DEFAULT_TIME_LIMIT, FUEL_VARIABLE, TIME_LIMIT_VARIABLE};

thread_local! {
	static FUEL_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
	static TIME_LIMIT_OVERRIDE: Cell<Option<Duration>> = const { Cell::new(None) };
}

/// A thread-local cell whose value can be swapped: `Cell` and `RefCell`
pub trait Swappable<T> {
	fn swap_value(&self, value: T) -> T;
}

impl<T> Swappable<T> for Cell<T> {
	fn swap_value(&self, value: T) -> T {
		self.replace(value)
	}
}

impl<T> Swappable<T> for std::cell::RefCell<T> {
	fn swap_value(&self, value: T) -> T {
		self.replace(value)
	}
}

/// `run` with the thread-local `key` holding `value`, then the value it held before, also when `run` panics
pub fn scoped<C: Swappable<T>, T: 'static, R>(key: &'static LocalKey<C>, value: T, run: impl FnOnce() -> R) -> R {
	struct Restore<C: Swappable<T> + 'static, T: 'static> {
		key: &'static LocalKey<C>,
		outer: Option<T>,
	}
	impl<C: Swappable<T>, T> Drop for Restore<C, T> {
		fn drop(&mut self) {
			if let Some(outer) = self.outer.take() {
				// try_with: a thread's locals may already be gone when it unwinds at exit
				let _ = self.key.try_with(|cell| cell.swap_value(outer));
			}
		}
	}
	let outer = key.with(|cell| cell.swap_value(value));
	let _restore = Restore { key, outer: Some(outer) };
	run()
}

/// Fuel the next runs on this thread ask for: `with_fuel` or `warp fuel <steps>`, else `WARP_FUEL`; none runs them
/// with the time limit
pub fn requested_fuel() -> Option<u64> {
	FUEL_OVERRIDE.with(Cell::get).or_else(warp_runtime::fuel::fuel_from_environment)
}

/// Fuel budget of runs that count fuel on this thread: the requested fuel, else `DEFAULT_FUEL` (`use fuel`)
pub fn fuel_budget() -> u64 {
	requested_fuel().unwrap_or(DEFAULT_FUEL)
}

/// Set the fuel budget for every later run on this thread (the CLI's `warp fuel <steps>`)
pub fn set_fuel_budget(steps: u64) {
	FUEL_OVERRIDE.with(|budget| budget.set(Some(steps)));
}

/// Run `body` with a different fuel budget on this thread, then restore the previous one: its runs count fuel
pub fn with_fuel<R>(steps: u64, body: impl FnOnce() -> R) -> R {
	scoped(&FUEL_OVERRIDE, Some(steps), body)
}

/// Computing time of runs without fuel on this thread: `with_time_limit`, else `WARP_TIME_LIMIT`, else
/// `DEFAULT_TIME_LIMIT`
pub fn time_limit() -> Duration {
	(TIME_LIMIT_OVERRIDE.with(Cell::get))
		.or_else(warp_runtime::fuel::time_limit_from_environment)
		.unwrap_or(DEFAULT_TIME_LIMIT)
}

/// Run `body` with a different time limit on this thread, then restore the previous one
pub fn with_time_limit<R>(limit: Duration, body: impl FnOnce() -> R) -> R {
	scoped(&TIME_LIMIT_OVERRIDE, Some(limit), body)
}

/// A store for `engine` that stops its run: with the fuel budget on an engine counting fuel, else at the time limit
#[cfg(feature = "native")]
pub fn limited_store<T: 'static>(engine: &Engine, data: T) -> Store<T> {
	warp_runtime::engine::limited_store(engine, data, fuel_budget(), time_limit())
}

/// limited_store under the name of the time when every run counted fuel
#[cfg(feature = "native")]
pub use limited_store as fueled_store;

/// The body at `url`, empty (and reported on stderr) when the download fails
#[cfg(feature = "native")]
pub fn fetch(url: &str) -> String {
	crate::extensions::utils::download(url)
}

/// The engine of a program that starts tasks (tasks.rs), counting fuel when the runs ask for it (`requested_fuel`)
#[cfg(feature = "native")]
pub fn task_engine() -> Engine {
	metered_task_engine(requested_fuel().is_some())
}

/// The engine of a program that starts tasks, with fuel metering when `fuel`: an engine of its own, whose epoch the
/// program's task table advances while it runs (a shared engine's ticks would reach every store)
#[cfg(feature = "native")]
pub fn metered_task_engine(fuel: bool) -> Engine {
	Engine::new(&warp_runtime::engine::metered_config(fuel)).expect("Failed to create WASM engine")
}

/// The WASM engine with GC, function references and epoch interruption, one per process: every run shares it, so a
/// module compiled once runs again without compiling (run/module_cache.rs); stores and their limits stay per run
/// (`limited_store`). Runs that ask for fuel (`requested_fuel`) get the shared engine counting fuel instead.
#[cfg(feature = "native")]
pub fn gc_engine() -> Engine {
	shared_engine(requested_fuel().is_some())
}

/// The shared engine counting fuel or the shared one checking the time (its epoch ticking all along)
#[cfg(feature = "native")]
pub fn shared_engine(fuel: bool) -> Engine {
	use std::sync::OnceLock;
	static TIMED: OnceLock<Engine> = OnceLock::new();
	static FUELED: OnceLock<Engine> = OnceLock::new();
	let new_engine = || Engine::new(&warp_runtime::engine::metered_config(fuel)).expect("Failed to create WASM engine");
	match fuel {
		true => FUELED.get_or_init(new_engine).clone(),
		false => TIMED.get_or_init(|| warp_runtime::engine::ticking(new_engine())).clone(),
	}
}

pub fn show_type_name<T>(_: &T) {
	use std::any::type_name;
	// println!("{}", type_name_of_val(*json!({"name": "Alice"})));
	println!("{}", type_name::<T>());
}

pub fn dirs_home() -> std::path::PathBuf {
	env::var("HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// `~` and `~/…` with the home folder in place of `~`, as the shell expands them; other paths as they are
pub fn home_expanded(path: &str) -> String {
	match path.strip_prefix('~') {
		Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{}{rest}", dirs_home().display()),
		_ => path.to_string(),
	}
}

pub fn load_file(path: &str) -> String {
	fs::read_to_string(path).unwrap_or_else(|_| {
		eprintln!("Error: Could not read file '{}'", path);
		String::new()
	})
}

pub fn file_exists(path: &str) -> bool {
	std::path::Path::new(path).exists()
}

pub fn extract_after(s: &str, sep: &str) -> String {
	s.split_once(sep).map(|(_, after)| after.to_string()).unwrap_or_default()
}
