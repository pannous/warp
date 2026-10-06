//! A standalone executable is the warp-runtime stub (or warp itself) carrying a program's machine code:
//! `<executable><machine code><its length, u64 little-endian><TRAILER_MAGIC><zeros to a multiple of 16>`, then, in a
//! Mach-O executable, its code signature: the machine code lies inside the __LINKEDIT segment, so the executable signs
//! and verifies cleanly (`codesign --verify --strict`, macho.rs). At start the executable looks for the trailer where
//! its program would end and runs what it carries.
use crate::engine::{fueled_config, fueled_store};
use crate::fuel::{fuel_from_environment, DEFAULT_FUEL};
use crate::macho;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;
use wasmtime::{Engine, Linker, Module, Result, Val};

const TRAILER_MAGIC: &[u8; 8] = b"WRPCwasm";
const TRAILER_BYTES: usize = 16;
/// The carried program ends on this boundary, where codesign puts the signature
const ALIGNMENT: usize = 16;
/// The exported functions a program starts at, in order of preference
pub const ENTRY_POINTS: [&str; 2] = ["main", "_start"];

/// `runtime` (an executable, with or without a program) carrying `machine_code` instead. A Mach-O executable comes
/// back unsigned: sign it (`codesign --sign -`, which `warp build` runs), else macOS refuses to start it.
pub fn with_machine_code(runtime: &[u8], machine_code: &[u8]) -> Vec<u8> {
	let mut executable = without_machine_code(runtime);
	executable.extend_from_slice(machine_code);
	executable.extend_from_slice(&(machine_code.len() as u64).to_le_bytes());
	executable.extend_from_slice(TRAILER_MAGIC);
	executable.resize(executable.len().next_multiple_of(ALIGNMENT), 0);
	macho::extend_last_segment_to_end(&mut executable);
	executable
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
	(&trailer[8..] == TRAILER_MAGIC).then(|| (trailer_start, u64::from_le_bytes(trailer[..8].try_into().unwrap()) as usize))
}

/// The machine code the executable at `path` carries, reading only its load commands and end when it carries none
pub fn embedded_machine_code(path: &Path) -> Option<Vec<u8>> {
	let mut file = File::open(path).ok()?;
	let size = file.metadata().ok()?.len() as usize;
	let header = read_at(&mut file, 0, size.min(macho::MAX_HEADER_BYTES))?;
	let end = macho::signature_offset(&header).unwrap_or(size);
	let tail_start = end.checked_sub(TRAILER_BYTES + ALIGNMENT)?;
	let (trailer_start, length) = trailer_before(&read_at(&mut file, tail_start, end - tail_start)?)?;
	read_at(&mut file, (tail_start + trailer_start).checked_sub(length)?, length)
}

fn read_at(file: &mut File, offset: usize, length: usize) -> Option<Vec<u8>> {
	file.seek(SeekFrom::Start(offset as u64)).ok()?;
	let mut bytes = vec![0u8; length];
	file.read_exact(&mut bytes).ok()?;
	Some(bytes)
}

/// When this executable carries a program: run it and give the exit code
pub fn run_carried_program() -> Option<i32> {
	let machine_code = embedded_machine_code(&std::env::current_exe().ok()?)?;
	Some(match run(&machine_code) {
		Ok(()) => 0,
		Err(failure) => {
			eprintln!("{failure:#}");
			1
		}
	})
}

/// Run machine code compiled with warp's settings (`warp build`): its host words, libm and print are linked, its
/// result is what it printed (build makes the program print its value)
pub fn run(machine_code: &[u8]) -> Result<()> {
	let engine = Engine::new(&fueled_config())?;
	// SAFETY: the machine code is warp's own output, appended to this executable by `warp build`
	let module = unsafe { Module::deserialize(&engine, machine_code)? };
	let mut linker = Linker::new(&engine);
	crate::host_words::link_host_words(&mut linker)?;
	crate::output::link_output(&mut linker)?;
	crate::libm::link_libm(&mut linker)?;
	let mut store = fueled_store(&engine, (), fuel_from_environment().unwrap_or(DEFAULT_FUEL));
	let instance = linker.instantiate(&mut store, &module)?;
	let main = ENTRY_POINTS
		.iter()
		.find_map(|name| instance.get_func(&mut store, name))
		.ok_or_else(|| wasmtime::Error::msg(format!("the program has no entry point {ENTRY_POINTS:?}")))?;
	let mut results = vec![Val::AnyRef(None); main.ty(&store).results().len()];
	main.call(&mut store, &[], &mut results)
}

/// The imports a standalone executable provides: (module, name)
pub fn provided_imports() -> impl Iterator<Item = (&'static str, &'static str)> {
	let host_words = crate::host_words::BASIC_HOST_WORDS.into_iter().map(|word| (crate::host_words::HOST_LIBRARY, word));
	let libm = crate::libm::libm_names().map(|name| (crate::libm::LIBM, name));
	host_words.chain(libm).chain([(crate::output::WASI_LIBRARY, crate::output::FD_WRITE)])
}
