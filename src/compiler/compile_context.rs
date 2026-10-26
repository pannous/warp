//! What the run on this thread is about (card cleanup-compile-context; on `pure` only what the harness keeps, card
//! rust-bootstrap): one thread-local. A field set for one run is `scoped`; one written for later readers is written with
//! `change`. Closures given to `read` and `change` only touch fields, or the context would be borrowed twice.
use crate::errors::diagnostic::WarningMode;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Default)]
pub struct CompileContext {
	/// the program runs sandboxed (`warp --sandbox`)
	pub sandboxed: bool,
	/// the program may read and write beyond the working directory
	pub beyond_working_directory: bool,
	/// the source of the program, for the positions of its messages
	pub source: String,
	pub warning_mode: WarningMode,
	/// the run counts fuel (WARP_FUEL)
	pub counts_fuel: bool,
	/// the file whose source is in `source`, for messages
	pub parsed_file: Option<String>,
	/// `use strict`: warnings are errors
	pub says_strict: bool,
	/// comments kept as meta by the parse
	pub comments_as_meta: bool,
	/// the program reflects on its own code
	pub reflection_mode: bool,
	/// the program's own file: its folder is where its `use`s and relative paths are looked for
	pub program_file: Option<PathBuf>,
	/// the folder the program's relative paths mean, when not the process's
	pub working_folder: Option<PathBuf>,
	/// the module files the program read
	pub module_files: Vec<PathBuf>,
}

thread_local! {
	static CONTEXT: RefCell<CompileContext> = RefCell::new(CompileContext::default());
}

pub fn read<R>(look: impl FnOnce(&CompileContext) -> R) -> R {
	CONTEXT.with(|context| look(&context.borrow()))
}

pub fn change<R>(write: impl FnOnce(&mut CompileContext) -> R) -> R {
	CONTEXT.with(|context| write(&mut context.borrow_mut()))
}

/// A context field set for a while: dropping it puts back the value the field held before
struct Restore<T: 'static> {
	field: fn(&mut CompileContext) -> &mut T,
	outer: Option<T>,
}

impl<T> Restore<T> {
	fn set(field: fn(&mut CompileContext) -> &mut T, value: T) -> Self {
		let outer = change(|context| std::mem::replace(field(context), value));
		Restore { field, outer: Some(outer) }
	}
}

impl<T> Drop for Restore<T> {
	fn drop(&mut self) {
		if let Some(outer) = self.outer.take() {
			// try_with: a thread's locals may already be gone when it unwinds at exit
			let _ = CONTEXT.try_with(|context| *(self.field)(&mut context.borrow_mut()) = outer);
		}
	}
}

/// A switch of the context, such as the one a pragma turns on
pub type Flag = fn(&mut CompileContext) -> &mut bool;

/// `run` with the context's `field` holding `value`, then the value it held before, also when `run` panics
pub fn scoped<T: 'static, R>(field: fn(&mut CompileContext) -> &mut T, value: T, run: impl FnOnce() -> R) -> R {
	let _restore = Restore::set(field, value);
	run()
}

/// `run` with each flag holding its value, then the values they held before, also when `run` panics
pub fn scoped_flags<R>(flags: &[(Flag, bool)], run: impl FnOnce() -> R) -> R {
	let _restores: Vec<Restore<bool>> = flags.iter().map(|&(field, value)| Restore::set(field, value)).collect();
	run()
}
