use std::cell::Cell;
use std::thread::LocalKey;
use std::{env, fs};
#[cfg(feature = "native")]
use wasmtime::{Engine, Store};

#[cfg(feature = "native")]
pub use warp_runtime::engine::deterministic_config;
pub use warp_runtime::fuel::{DEFAULT_FUEL, FUEL_VARIABLE};

thread_local! {
	static FUEL_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
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

/// Fuel budget of the next runs on this thread: `with_fuel`, else `WARP_FUEL`, else `DEFAULT_FUEL`
pub fn fuel_budget() -> u64 {
	FUEL_OVERRIDE.with(Cell::get).or_else(warp_runtime::fuel::fuel_from_environment).unwrap_or(DEFAULT_FUEL)
}

/// Set the fuel budget for every later run on this thread (the CLI's `--fuel`)
pub fn set_fuel_budget(steps: u64) {
	FUEL_OVERRIDE.with(|budget| budget.set(Some(steps)));
}

/// Run `body` with a different fuel budget on this thread, then restore the previous one
pub fn with_fuel<R>(steps: u64, body: impl FnOnce() -> R) -> R {
	scoped(&FUEL_OVERRIDE, Some(steps), body)
}

/// A store for a `gc_engine` with the current fuel budget
#[cfg(feature = "native")]
pub fn fueled_store<T>(engine: &Engine, data: T) -> Store<T> {
	warp_runtime::engine::fueled_store(engine, data, fuel_budget())
}

/// The body at `url`, empty (and reported on stderr) when the download fails
#[cfg(feature = "native")]
pub fn fetch(url: &str) -> String {
	crate::extensions::utils::download(url)
}

/// The engine of a program that starts tasks (tasks.rs): gc_engine's settings plus epoch interruption, the check
/// points where a task stops or pauses
#[cfg(feature = "native")]
pub fn task_engine() -> Engine {
	let mut config = warp_runtime::engine::fueled_config();
	config.epoch_interruption(true);
	Engine::new(&config).expect("Failed to create WASM engine")
}

/// The WASM engine with GC, function references, canonical NaNs and fuel metering, one per process: every run shares
/// it, so a module compiled once runs again without compiling (run/module_cache.rs); stores and their fuel stay per
/// run (`fueled_store`). Programs that start tasks get an engine of their own (task_engine): their epoch ticks would
/// reach every store of a shared engine.
#[cfg(feature = "native")]
pub fn gc_engine() -> Engine {
	// todo move to wasm / runner / ... ?
	static SHARED: std::sync::OnceLock<Engine> = std::sync::OnceLock::new();
	SHARED
		.get_or_init(|| Engine::new(&warp_runtime::engine::fueled_config()).expect("Failed to create WASM engine"))
		.clone()
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
