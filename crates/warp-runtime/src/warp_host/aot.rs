//! Ahead-of-time compilation (card warp-host-aot, notes/aot.md): a module's machine code for this machine and this
//! warp-host (its wasmtime version and engine settings), written to a .cwasm (`compile --aot`), or carried by a copy of
//! warp-host itself (`build`): a standalone executable running it with the same imports (WASI, std_io, C calls).
//! A .cwasm is native code: running one is running an executable.
use std::path::Path;
use warp_runtime::c_signatures::{split_c_functions, with_c_functions, CFunction, CARRIED_C_FUNCTIONS};
use wasmtime::{Engine, Module, Precompiled};

pub const EXTENSION: &str = "cwasm";
/// What a precompiled module starts with: wasmtime's ELF image, or the table of C functions in front of it
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";

/// A module's machine code: the table of its C functions in front (its `warp.c_functions` section, which machine
/// code keeps no custom section of; c_signatures.rs with_c_functions, as warp build carries it), then wasmtime's image
pub fn machine_code(engine: &Engine, wasm: &[u8]) -> Result<Vec<u8>, String> {
	let functions = crate::ffi::described(wasm).map_err(|problem| format!("{problem:#}"))?;
	let image = engine.precompile_module(wasm).map_err(|problem| format!("cannot compile to machine code: {problem:#}"))?;
	Ok(with_c_functions(&functions, &image))
}

/// A file of machine code (`warp-host f.cwasm`): by its extension, else by how it starts
pub fn is_machine_code(file: &str) -> bool {
	use std::io::Read;
	let mut start = [0u8; 8];
	let starts_so = std::fs::File::open(file).and_then(|mut opened| opened.read_exact(&mut start)).is_ok()
		&& (&start == CARRIED_C_FUNCTIONS || start.starts_with(ELF_MAGIC));
	file.ends_with(&format!(".{EXTENSION}")) || (Path::new(file).is_file() && starts_so)
}

/// The module and C functions of what `machine_code` wrote (`name` for the errors). Machine code of another
/// warp-host (another wasmtime version or engine settings) or another machine (CPU features) is refused clearly
pub fn load(engine: &Engine, carried: &[u8], name: &str) -> Result<(Module, Vec<CFunction>), String> {
	let (functions, image) = split_c_functions(carried)?;
	match Engine::detect_precompiled(image) {
		Some(Precompiled::Module) => {}
		Some(Precompiled::Component) => return Err(format!("{name} is a component's machine code, not a module's")),
		None => return Err(format!("{name} is no machine code of a module (`warp-host compile --aot` writes it)")),
	}
	// SAFETY: precompiled machine code runs as it is, as an executable does; wasmtime checks that its version,
	// engine settings and the CPU features it needs are this warp-host's and this machine's
	match unsafe { Module::deserialize(engine, image) } {
		Ok(module) => Ok((module, functions)),
		Err(problem) => Err(format!(
			"{name} was compiled by another warp-host or for another machine ({}): compile it again with `warp-host compile --aot`",
			format!("{problem:#}").lines().next().unwrap_or_default()
		)),
	}
}

/// The machine code this executable carries, when it is one `build` wrote
pub fn carried() -> Option<Vec<u8>> {
	warp_runtime::standalone::embedded_machine_code(&std::env::current_exe().ok()?)
}

/// The standalone executable: a copy of this warp-host (without any program it carries) with `machine_code`
/// appended behind the trailer of warp's executables (standalone.rs: deflated, its length, WRPCwasm), inside the
/// __LINKEDIT segment on macOS and signed ad hoc there; its size
pub fn write_executable(machine_code: &[u8], output: &Path) -> Result<usize, String> {
	let host = std::env::current_exe().map_err(|problem| format!("cannot find warp-host itself: {problem}"))?;
	let runtime = std::fs::read(&host).map_err(|problem| format!("{}: {problem}", host.display()))?;
	let executable = warp_runtime::standalone::with_machine_code(&runtime, machine_code);
	// a new file, not one written over: a running executable of that name keeps its pages
	let _ = std::fs::remove_file(output);
	std::fs::write(output, &executable).map_err(|problem| format!("{}: {problem}", output.display()))?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))
			.map_err(|problem| format!("{}: {problem}", output.display()))?;
	}
	signed(output)?;
	Ok(executable.len())
}

/// macOS starts only signed arm64 executables: an ad-hoc signature (`codesign --verify --strict` accepts it); a
/// Developer ID is the same call with an identity
#[cfg(target_os = "macos")]
fn signed(output: &Path) -> Result<(), String> {
	match std::process::Command::new("codesign").args(["--sign", "-", "--force"]).arg(output).output() {
		Ok(signed) if signed.status.success() => Ok(()),
		Ok(signed) => {
			Err(format!("codesign {} failed: {}", output.display(), String::from_utf8_lossy(&signed.stderr).trim()))
		}
		Err(problem) => Err(format!("cannot run codesign for {}: {problem}", output.display())),
	}
}

#[cfg(not(target_os = "macos"))]
fn signed(_: &Path) -> Result<(), String> {
	Ok(())
}
