//! A standalone executable is the warp-runtime stub carrying a program's machine code:
//! `<executable><machine code, deflated><its length, u64 little-endian><TRAILER_MAGIC><zeros to a multiple of 16>`, then, in a
//! Mach-O executable, its code signature: the machine code lies inside the __LINKEDIT segment, so the executable signs
//! and verifies cleanly (`codesign --verify --strict`, macho.rs). At start the executable looks for the trailer where
//! its program would end and runs what it carries.
use crate::c_calls::CHandles;
use crate::engine::{limited_store, metered_config, ticking};
use crate::fuel::{time_limit_from_environment, DEFAULT_FUEL};
use crate::host_words::{FOREIGN_CALL, HANDLER_PREFIX};
use crate::macho;
use crate::system_signals::{NEW_EMPTY, NEW_INT};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;
use std::time::Duration;
use wasmtime::{Engine, Instance, Linker, Module, Result, Store, Val};

const TRAILER_MAGIC: &[u8; 8] = b"WRPCwasm";
const TRAILER_BYTES: usize = 16;
/// The carried machine code is deflated: most of it is padding to page boundaries and tables (180 KB → 45 KB)
const COMPRESSION_LEVEL: u8 = 9;
/// The carried program ends on this boundary, where codesign puts the signature
const ALIGNMENT: usize = 16;
/// The exported functions a program starts at, in order of preference
pub const ENTRY_POINTS: [&str; 2] = ["main", "_start"];
/// The memory print and the host words read texts from
const MEMORY_EXPORT: &str = "memory";
/// An app carrying a site loads it in a hidden window and prints its text (tests), as `warp app --check`
#[cfg(all(feature = "webview", target_os = "macos"))]
const CHECK_FLAG: &str = "--check";

/// Whether the stub looks up this export of a program: its entry point, memory, handlers and the constructors of their
/// `event`. warp build drops every other export, and the functions only those reached (tree shaking, card g_gFs8)
pub fn stub_calls_export(name: &str) -> bool {
	ENTRY_POINTS.contains(&name)
		|| [MEMORY_EXPORT, NEW_EMPTY, NEW_INT].contains(&name)
		|| name.starts_with(HANDLER_PREFIX)
}

/// The exports the stub looks up of a program calling python or js: those of every program, and what the answers are
/// built with (gc_values.rs)
pub fn stub_calls_export_with_foreign_calls(name: &str) -> bool {
	stub_calls_export(name) || crate::gc_values::BUILDERS.contains(&name) || name == crate::gc_values::TEXT_HEAP
}

/// The engine an executable's machine code is compiled for (warp build) and run with (the stub): no address map, the
/// table from machine code to wasm offsets (an eighth of the machine code); a trap names the functions without them.
/// An executable counts no fuel; it stops only at a time limit WARP_TIME_LIMIT sets.
pub fn standalone_engine() -> Result<Engine> {
	let mut config = metered_config(false);
	config.generate_address_map(false);
	Engine::new(&config)
}

/// `runtime` (an executable, with or without a program) carrying `machine_code` instead. A Mach-O executable comes
/// back unsigned: sign it (`codesign --sign -`, which `warp build` runs), else macOS refuses to start it.
pub fn with_machine_code(runtime: &[u8], machine_code: &[u8]) -> Vec<u8> {
	let mut executable = without_machine_code(runtime);
	let compressed = miniz_oxide::deflate::compress_to_vec(machine_code, COMPRESSION_LEVEL);
	executable.extend_from_slice(&compressed);
	executable.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
	executable.extend_from_slice(TRAILER_MAGIC);
	executable.resize(executable.len().next_multiple_of(ALIGNMENT), 0);
	macho::extend_last_segment_to_end(&mut executable);
	executable
}

/// `runtime` carrying a page's site: an app showing it in the system's web view (card build-links); sign it as above
pub fn with_site(runtime: &[u8], site: &[crate::site_files::SiteFile]) -> Vec<u8> {
	with_machine_code(runtime, &crate::site_files::packed(site))
}

/// The executable without its code signature and without the program it carries
pub fn without_machine_code(executable: &[u8]) -> Vec<u8> {
	let mut unsigned = macho::unsigned(executable);
	if let Some(carried) = carried_range(&unsigned) {
		unsigned.truncate(carried.start);
	}
	unsigned
}

/// Where the program in `executable` lies: it ends with the trailer, before the code signature if there is one
fn carried_range(executable: &[u8]) -> Option<Range<usize>> {
	let end = macho::signature_offset(executable).unwrap_or(executable.len());
	let (trailer_start, length) = trailer_before(executable.get(..end)?)?;
	Some(trailer_start.checked_sub(length)?..trailer_start)
}

/// The trailer at the end of `bytes` (after its zero padding): where it starts, and the length of the program it closes
fn trailer_before(bytes: &[u8]) -> Option<(usize, usize)> {
	let padding = bytes.iter().rev().take(ALIGNMENT - 1).take_while(|&&byte| byte == 0).count();
	let trailer_start = (bytes.len() - padding).checked_sub(TRAILER_BYTES)?;
	let trailer: &[u8; TRAILER_BYTES] = bytes[trailer_start..trailer_start + TRAILER_BYTES].try_into().ok()?;
	(&trailer[8..] == TRAILER_MAGIC)
		.then(|| (trailer_start, u64::from_le_bytes(trailer[..8].try_into().unwrap()) as usize))
}

/// The machine code the executable at `path` carries, reading only its load commands and end when it carries none
pub fn embedded_machine_code(path: &Path) -> Option<Vec<u8>> {
	let mut file = File::open(path).ok()?;
	let size = file.metadata().ok()?.len() as usize;
	let header = read_at(&mut file, 0, size.min(macho::MAX_HEADER_BYTES))?;
	let end = macho::signature_offset(&header).unwrap_or(size);
	let tail_start = end.checked_sub(TRAILER_BYTES + ALIGNMENT)?;
	let (trailer_start, length) = trailer_before(&read_at(&mut file, tail_start, end - tail_start)?)?;
	let compressed = read_at(&mut file, (tail_start + trailer_start).checked_sub(length)?, length)?;
	miniz_oxide::inflate::decompress_to_vec(&compressed).ok()
}

fn read_at(file: &mut File, offset: usize, length: usize) -> Option<Vec<u8>> {
	file.seek(SeekFrom::Start(offset as u64)).ok()?;
	let mut bytes = vec![0u8; length];
	file.read_exact(&mut bytes).ok()?;
	Some(bytes)
}

/// When this executable carries a program: run it and give the exit code; a page's site it shows (card build-links)
pub fn run_carried_program() -> Option<i32> {
	let executable = std::env::current_exe().ok()?;
	let machine_code = embedded_machine_code(&executable)?;
	if let Some(site) = crate::site_files::unpacked(&machine_code) {
		return Some(show_site(&executable, site));
	}
	Some(match run(&machine_code) {
		Ok(()) => 0,
		Err(failure) => {
			eprintln!("{failure:#}");
			1
		}
	})
}

/// The carried site in a web view window titled as the executable; `--check` prints its text from a hidden one
#[cfg(all(feature = "webview", target_os = "macos"))]
fn show_site(executable: &Path, site: Vec<crate::site_files::SiteFile>) -> i32 {
	let title = executable.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	let check = std::env::args().any(|argument| argument == CHECK_FLAG);
	match crate::webview::show(&title, check, Box::new(move |path| crate::site_files::served(&site, path))) {
		Ok(()) => 0,
		Err(failure) => {
			eprintln!("{failure}");
			1
		}
	}
}

#[cfg(not(all(feature = "webview", target_os = "macos")))]
fn show_site(_: &Path, _: Vec<crate::site_files::SiteFile>) -> i32 {
	eprintln!("this app shows a page, and this warp-runtime has no web view (macOS only for now)");
	1
}

/// Run machine code compiled with warp's settings (`warp build`): its host words, libm, libc, print and the C functions
/// it carries are linked, its result is what it printed (build makes the program print its value)
pub fn run(carried: &[u8]) -> Result<()> {
	let (c_functions, machine_code) = crate::c_signatures::split_c_functions(carried).map_err(wasmtime::Error::msg)?;
	// an executable is the user's app (a game, a server): it runs without a time limit unless WARP_TIME_LIMIT sets one
	let time_limit = time_limit_from_environment();
	let engine = running_engine(time_limit)?;
	// SAFETY: the machine code is warp's own output, appended to this executable by `warp build`
	let module = unsafe { Module::deserialize(&engine, machine_code)? };
	if let Ran::Exited = run_module(&engine, &module, &c_functions, time_limit)? {
		use std::io::Write;
		let _ = std::io::stdout().flush();
		std::process::exit(crate::system_signals::take_exit_code().unwrap_or(0)); // `exit(code)` (P121)
	}
	Ok(())
}

/// The standalone engine, ticking when a time limit ends the run
pub fn running_engine(time_limit: Option<Duration>) -> Result<Engine> {
	Ok(match time_limit {
		Some(_) => ticking(standalone_engine()?),
		None => standalone_engine()?,
	})
}

/// How a program's run ended: main returned its results, or the program called `exit(code)`
pub enum Ran {
	Returned { store: Store<CHandles>, instance: Instance, results: Vec<Val> },
	Exited,
}

/// Run `module` with the stub's host words, libm, libc, print and `c_functions` linked: from main through the handlers
/// it listens with
pub fn run_module(
	engine: &Engine,
	module: &Module,
	c_functions: &[crate::c_signatures::CFunction],
	time_limit: Option<Duration>,
) -> Result<Ran> {
	let mut linker = Linker::new(engine);
	crate::host_words::link_host_words(&mut linker)?;
	crate::output::link_output(&mut linker)?;
	crate::libm::link_libm(&mut linker)?;
	crate::c_calls::link_libc_functions(&mut linker)?;
	for function in c_functions {
		crate::c_calls::link_carried_function(&mut linker, engine, function)?;
	}
	linker.func_wrap(
		crate::host_words::HOST_LIBRARY,
		FOREIGN_CALL,
		|mut caller: wasmtime::Caller<'_, CHandles>, runtime, module, member, call, arguments| {
			crate::gc_values::foreign_call(&mut caller, [runtime, module, member, call, arguments])
		},
	)?;
	let mut store = limited_store(engine, CHandles::default(), DEFAULT_FUEL, time_limit.unwrap_or(Duration::MAX));
	let instance = linker.instantiate(&mut store, module)?;
	let main = ENTRY_POINTS
		.iter()
		.find_map(|name| instance.get_func(&mut store, name))
		.ok_or_else(|| wasmtime::Error::msg(format!("the program has no entry point {ENTRY_POINTS:?}")))?;
	let mut results = vec![Val::AnyRef(None); main.ty(&store).results().len()];
	crate::system_signals::allow_staying();
	let outcome = main
		.call(&mut store, &[], &mut results)
		.and_then(|_| crate::system_signals::stay_while_listening(&mut store, &instance));
	let outcome = crate::system_signals::with_exit_handler(outcome, &mut store, |store, name| {
		instance.get_func(&mut *store, name)
	})?;
	Ok(match outcome {
		Some(()) => Ran::Returned { store, instance, results },
		None => Ran::Exited,
	})
}

/// The imports a standalone executable provides: (module, name); C functions beyond them it carries (c_signatures.rs)
pub fn provided_imports() -> impl Iterator<Item = (&'static str, &'static str)> {
	let host_words =
		crate::host_words::BASIC_HOST_WORDS.into_iter().map(|word| (crate::host_words::HOST_LIBRARY, word));
	let libm = crate::libm::libm_names().map(|name| (crate::libm::LIBM, name));
	host_words.chain(libm).chain(crate::c_calls::libc_names()).chain([
		(crate::output::WASI_LIBRARY, crate::output::FD_WRITE),
		(crate::host_words::HOST_LIBRARY, FOREIGN_CALL),
	])
}
