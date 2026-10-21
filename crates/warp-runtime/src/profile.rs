//! WARP_PROFILE=<file>: where a run spends its time, sampled at each epoch tick with wasmtime's GuestProfiler and
//! written as Firefox Profiler JSON when the run's store drops (open it at https://profiler.firefox.com). Samples are
//! taken at function entries and loop headers of the wasm, host calls show as markers. Engines counting fuel don't
//! tick, so their runs give no samples.
use wasmtime::{AsContext, CallHook, Engine, Module};

/// Environment variable naming the profile file; runs after the first in a process (stage programs, tasks) write
/// `<file stem>-<n>.<extension>` beside it
pub const PROFILE_VARIABLE: &str = "WARP_PROFILE";

#[cfg(feature = "profiling")]
pub use sampled::Profile;

#[cfg(feature = "profiling")]
mod sampled {
	use super::*;
	use std::path::{Path, PathBuf};
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::{Arc, Mutex};
	use std::time::Duration;
	use wasmtime::GuestProfiler;

	/// The profile of one run, shared by its store's epoch callback and call hook
	#[derive(Clone)]
	pub struct Profile(Arc<Mutex<RunProfile>>);

	struct RunProfile {
		profiler: Option<GuestProfiler>,
		file: PathBuf,
		/// the thread's CPU time at the last sample
		sampled_at: Duration,
	}

	impl Profile {
		/// A profile of `module`'s run when WARP_PROFILE names a file
		pub fn of(engine: &Engine, module: &Module) -> Option<Profile> {
			let file = profile_file(&std::env::var(PROFILE_VARIABLE).ok().filter(|file| !file.is_empty())?);
			let name = module.name().unwrap_or("main").to_string();
			let interval = crate::engine::EPOCH_TICK;
			match GuestProfiler::new(engine, &name, interval, [(name.clone(), module.clone())]) {
				Ok(profiler) => {
					let sampled_at = crate::engine::thread_cpu_time();
					Some(Profile(Arc::new(Mutex::new(RunProfile { profiler: Some(profiler), file, sampled_at }))))
				}
				Err(failure) => {
					eprintln!("{PROFILE_VARIABLE}: no profile of {name}: {failure}");
					None
				}
			}
		}

		pub fn sample(&self, store: impl AsContext) {
			let mut run = self.0.lock().unwrap();
			let now = crate::engine::thread_cpu_time();
			let delta = now.saturating_sub(run.sampled_at);
			run.sampled_at = now;
			if let Some(profiler) = run.profiler.as_mut() {
				profiler.sample(store, delta);
			}
		}

		pub fn call_hook(&self, store: impl AsContext, hook: CallHook) {
			if let Some(profiler) = self.0.lock().unwrap().profiler.as_mut() {
				profiler.call_hook(store, hook);
			}
		}
	}

	impl Drop for RunProfile {
		fn drop(&mut self) {
			let Some(profiler) = self.profiler.take() else { return };
			let written = std::fs::File::create(&self.file)
				.map_err(wasmtime::Error::from)
				.and_then(|file| profiler.finish(std::io::BufWriter::new(file)));
			match written {
				Ok(()) => eprintln!("profile: {} (open it at https://profiler.firefox.com)", self.file.display()),
				Err(failure) => eprintln!("{PROFILE_VARIABLE}: {} not written: {failure}", self.file.display()),
			}
		}
	}

	/// The file of this process's next profile: `file` first, then `<stem>-2.<extension>`, …
	fn profile_file(file: &str) -> PathBuf {
		static PROFILES: AtomicUsize = AtomicUsize::new(0);
		let number = PROFILES.fetch_add(1, Ordering::Relaxed) + 1;
		let file = Path::new(file);
		if number == 1 {
			return file.to_path_buf();
		}
		let stem = file.file_stem().unwrap_or_default().to_string_lossy();
		let numbered = match file.extension() {
			Some(extension) => format!("{stem}-{number}.{}", extension.to_string_lossy()),
			None => format!("{stem}-{number}"),
		};
		file.with_file_name(numbered)
	}
}

/// Without the profiling feature no run is profiled
#[cfg(not(feature = "profiling"))]
#[derive(Clone)]
pub enum Profile {}

#[cfg(not(feature = "profiling"))]
impl Profile {
	pub fn of(_engine: &Engine, _module: &Module) -> Option<Profile> {
		if std::env::var_os(PROFILE_VARIABLE).is_some() {
			eprintln!("{PROFILE_VARIABLE}: this build has no profiler (warp-runtime feature profiling)");
		}
		None
	}

	pub fn sample(&self, _store: impl AsContext) {
		match *self {}
	}

	pub fn call_hook(&self, _store: impl AsContext, _hook: CallHook) {
		match *self {}
	}
}
