//! A standalone executable is the warp-runtime stub (or warp itself) with a program's machine code appended:
//! `<executable><machine code><its length, u64 little-endian><TRAILER_MAGIC>`. At start the executable looks at its
//! own last bytes and runs the program it carries.
use crate::engine::{fueled_config, fueled_store};
use crate::fuel::{fuel_from_environment, DEFAULT_FUEL};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use wasmtime::{Engine, Linker, Module, Result, Val};

const TRAILER_MAGIC: &[u8; 8] = b"WRPCwasm";
const TRAILER_BYTES: usize = 16;
/// The exported functions a program starts at, in order of preference
pub const ENTRY_POINTS: [&str; 2] = ["main", "_start"];

/// `runtime` (an executable, with or without a program) carrying `machine_code` instead
pub fn with_machine_code(runtime: &[u8], machine_code: &[u8]) -> Vec<u8> {
	let mut executable = without_machine_code(runtime).to_vec();
	executable.extend_from_slice(machine_code);
	executable.extend_from_slice(&(machine_code.len() as u64).to_le_bytes());
	executable.extend_from_slice(TRAILER_MAGIC);
	executable
}

/// The executable without the program it carries
pub fn without_machine_code(executable: &[u8]) -> &[u8] {
	match carried_length(executable) {
		Some(length) => &executable[..executable.len() - TRAILER_BYTES - length],
		None => executable,
	}
}

fn carried_length(executable: &[u8]) -> Option<usize> {
	let trailer = executable.get(executable.len().checked_sub(TRAILER_BYTES)?..)?;
	parse_trailer(trailer.try_into().ok()?).filter(|length| length + TRAILER_BYTES <= executable.len())
}

fn parse_trailer(trailer: &[u8; TRAILER_BYTES]) -> Option<usize> {
	(&trailer[8..] == TRAILER_MAGIC).then(|| u64::from_le_bytes(trailer[..8].try_into().unwrap()) as usize)
}

/// The machine code the executable at `path` carries, reading only its end when it carries none
pub fn embedded_machine_code(path: &Path) -> Option<Vec<u8>> {
	let mut file = File::open(path).ok()?;
	let size = file.metadata().ok()?.len();
	file.seek(SeekFrom::End(-(TRAILER_BYTES as i64))).ok()?;
	let mut trailer = [0u8; TRAILER_BYTES];
	file.read_exact(&mut trailer).ok()?;
	let length = parse_trailer(&trailer)?;
	let start = size.checked_sub((TRAILER_BYTES + length) as u64)?;
	file.seek(SeekFrom::Start(start)).ok()?;
	let mut machine_code = vec![0u8; length];
	file.read_exact(&mut machine_code).ok()?;
	Some(machine_code)
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

/// Run machine code compiled with warp's settings (`warp build --exe`): its host words, libm and print are linked, its
/// result is what it printed (build --exe makes the program print its value)
pub fn run(machine_code: &[u8]) -> Result<()> {
	let engine = Engine::new(&fueled_config())?;
	// SAFETY: the machine code is warp's own output, appended to this executable by `warp build --exe`
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
