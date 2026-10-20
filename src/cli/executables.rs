//! Standalone executables: the warp-runtime stub with the program appended, app bundles, building the stub
use super::*;

/// WARP_EXECUTABLE=1 (or true/yes/on): `warp <file>` also leaves the standalone executable next to the file, off by
/// default (card executable-flag, user 2026-10-12, replaces P103's default); `warp build` always makes it
pub(crate) const EXECUTABLE_VARIABLE: &str = "WARP_EXECUTABLE";
/// The crate of the stub in warp's source checkout
pub(crate) const RUNTIME_STUB_CRATE: &str = "crates/warp-runtime";
#[cfg(unix)]
pub(crate) const EXECUTABLE_MODE: u32 = 0o755;
/// Where a failed standalone build is remembered (one file per program path), so its note shows once per version of
/// the program and of warp, not on every run
pub(crate) const STANDALONE_NOTES_CACHE: &str = ".cache/warp/standalone_notes";

/// The prebuilt runtimes executables are made of: a stub's binary, the variable naming another, its cargo features
pub(crate) struct RuntimeStub {
	name: &'static str,
	variable: &'static str,
	features: &'static str,
}

pub(crate) const PROGRAM_STUB: RuntimeStub =
	RuntimeStub { name: "warp-runtime", variable: "WARP_RUNTIME_STUB", features: "" };
// a page app's stub carries the web view (wry, winit), which plain executables do without (card build-links)
pub(crate) const PAGE_STUB: RuntimeStub =
	RuntimeStub { name: "warp-page-runtime", variable: "WARP_PAGE_RUNTIME_STUB", features: "webview" };

pub(crate) fn wants_executable() -> bool {
	env::var(EXECUTABLE_VARIABLE)
		.is_ok_and(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// P103 (with WARP_EXECUTABLE): the executable next to a program file that ran, unless one newer than the file is there already; a program
/// the runtime cannot carry (it fetches, say) gets a note on stderr instead
pub(crate) fn leave_executable(path: &str) {
	let modified = |file: &std::path::Path| fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
	let executable = standalone_output_path(path);
	if modified(&executable).zip(modified(std::path::Path::new(path))).is_some_and(|(built, written)| built >= written)
	{
		return;
	}
	let program_file = std::path::Path::new(path);
	let marker = standalone_note_marker(program_file);
	let noted = marker.as_deref().and_then(modified);
	let warp_binary = env::current_exe().ok().and_then(|binary| modified(&binary));
	if noted.is_some_and(|noted| Some(noted) >= modified(program_file) && Some(noted) >= warp_binary) {
		return;
	}
	let written = diagnostic::quietly(|| {
		warp::modules::with_program_file(program_file, || write_standalone_executable(&load_file(path), path))
	});
	if let Err(failure) = written {
		// (said once until the file or warp changes)
		eprintln!("note: no executable {}: {failure}", executable.display());
		if let Some(marker) = marker {
			let _ = marker.parent().map(fs::create_dir_all);
			let _ = fs::write(&marker, failure);
		}
	}
}

/// The file remembering that the standalone build of `program` failed: ~/.cache/warp/standalone_notes/<hash of its path>
pub(crate) fn standalone_note_marker(program: &std::path::Path) -> Option<std::path::PathBuf> {
	use std::hash::{Hash, Hasher};
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	fs::canonicalize(program).ok()?.hash(&mut hasher);
	Some(
		std::path::PathBuf::from(env::var_os("HOME")?)
			.join(STANDALONE_NOTES_CACHE)
			.join(format!("{:016x}", hasher.finish())),
	)
}

/// The program, printing its value, compiled to machine code and appended to a copy of the compiler-less `warp-runtime`
/// stub (crates/warp-runtime, notes/aot.md); Ok is the report of what was written
pub(crate) fn write_standalone_executable(code: &str, target: &str) -> Result<String, String> {
	let program = wasm_emitter::compile_printing_result(code)
		.map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
	let foreign_call = (HOST_LIBRARY.to_string(), FOREIGN_CALL.to_string());
	let imports = warp::host::host_parts::imports_of(&program.bytes)?;
	let c_functions = carried_c_functions(&imports);
	// a C text result is built in the program as foreign answers are
	let builds_texts = c_functions.iter().any(|function| function.result == Some(warp::ffi::CPointer::Text));
	let kept_exports = if imports.contains(&foreign_call) || builds_texts {
		warp_runtime::standalone::stub_calls_export_with_foreign_calls
	} else {
		warp_runtime::standalone::stub_calls_export
	};
	let bytes = warp::compiler::dead_functions::keeping_exports(&program.bytes, kept_exports)?;
	let engine = warp_runtime::standalone::standalone_engine().map_err(|failure| failure.to_string())?;
	let module = run::module_cache::compiled_module(&engine, &bytes).map_err(|failure| failure.to_string())?;
	let provided: Vec<(&str, &str)> = warp_runtime::standalone::provided_imports().collect();
	let mut missing: Vec<String> = vec![];
	let carried = |module: &str, name: &str| c_functions.iter().any(|f| f.module == module && f.name == name);
	for import in module.imports().filter(|import| {
		!provided.contains(&(import.module(), import.name())) && !carried(import.module(), import.name())
	}) {
		let feature = feature_of(import.module(), import.name());
		if !missing.contains(&feature) {
			missing.push(feature);
		}
	}
	if !missing.is_empty() {
		return Err(format!("{} used runtime.", missing.join(", ")));
	}
	let machine_code = module.serialize().map_err(|failure| failure.to_string())?;
	let (stub, runtime) = runtime_stub(&PROGRAM_STUB)?;
	let output = standalone_output_path(target);
	let carried = warp_runtime::c_signatures::with_c_functions(&c_functions, &machine_code);
	let executable = warp_runtime::standalone::with_machine_code(&runtime, &carried);
	write_executable(&executable, &output)?;
	Ok(format!(
		"wrote {} ({} bytes: runtime {}, machine code {})",
		output.display(),
		executable.len(),
		stub.display(),
		machine_code.len()
	))
}

/// The C functions among `imports` the stub doesn't link itself, as their library's headers declare them: the
/// executable carries these descriptions, so it reads no header when it runs (card exe-c-ffi)
fn carried_c_functions(imports: &[(String, String)]) -> Vec<warp::ffi::CFunction> {
	let provided: Vec<(&str, &str)> = warp_runtime::standalone::provided_imports().collect();
	let mut by_module: std::collections::BTreeMap<&str, std::collections::HashSet<String>> = Default::default();
	for (module, name) in imports.iter().filter(|(module, name)| !provided.contains(&(module.as_str(), name.as_str())))
	{
		if module != HOST_LIBRARY && warp::ffi::is_ffi_library(module) {
			by_module.entry(module).or_default().insert(name.clone());
		}
	}
	by_module.iter().flat_map(|(module, names)| warp::ffi::header_c_functions(module, names)).collect()
}

/// `warp build page.warp` of a program showing a page, as `warp page.warp` opens it in a window: an app, the stub
/// carrying the page's site, which it shows in the system's web view (card build-links); macOS only for now
pub(crate) fn builds_an_app(code: &str) -> bool {
	cfg!(target_os = "macos") && warp::serve::shows_page(code)
}

pub(crate) fn write_web_view_app(code: &str, target: &str) -> Result<String, String> {
	let output = standalone_output_path(target);
	let title = output.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	let site = warp::web::site::files(code, &title, false)?;
	let (stub, runtime) = runtime_stub(&PAGE_STUB)?;
	let executable = warp_runtime::standalone::with_site(&runtime, &site);
	write_executable(&executable, &output)?;
	Ok(format!(
		"wrote {} ({} bytes: runtime {}, the page in a web view window)",
		output.display(),
		executable.len(),
		stub.display()
	))
}

/// The runtime stub's path and bytes
pub(crate) fn runtime_stub(kind: &RuntimeStub) -> Result<(std::path::PathBuf, Vec<u8>), String> {
	let stub = runtime_stub_path(kind)?;
	let runtime =
		fs::read(&stub).map_err(|failure| format!("cannot read the runtime {}: {failure}", stub.display()))?;
	Ok((stub, runtime))
}

/// The executable written, runnable and (macOS) signed
pub(crate) fn write_executable(executable: &[u8], output: &std::path::Path) -> Result<(), String> {
	fs::write(output, executable).map_err(|failure| format!("cannot write {}: {failure}", output.display()))?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		fs::set_permissions(output, fs::Permissions::from_mode(EXECUTABLE_MODE))
			.map_err(|failure| failure.to_string())?;
	}
	// macOS starts only signed executables: an ad-hoc signature, which `codesign --verify --strict` accepts
	#[cfg(target_os = "macos")]
	match std::process::Command::new("codesign").args(["--sign", "-", "--force"]).arg(output).output() {
		Ok(signed) if signed.status.success() => {}
		Ok(signed) => {
			return Err(format!(
				"codesign {} failed: {}",
				output.display(),
				String::from_utf8_lossy(&signed.stderr).trim()
			))
		}
		Err(failure) => return Err(format!("cannot run codesign for {}: {failure}", output.display())),
	}
	Ok(())
}

pub(crate) fn standalone_output_path(target: &str) -> std::path::PathBuf {
	if file_exists(target) {
		std::path::Path::new(target).with_extension(env::consts::EXE_EXTENSION)
	} else {
		std::path::Path::new(DEFAULT_EXECUTABLE_NAME).with_extension(env::consts::EXE_EXTENSION)
	}
}

/// The runtime an executable is built from: `WARP_RUNTIME_STUB`, else `warp-runtime` next to this warp (or next to the
/// file a link to warp points to), else built once from warp's own source checkout (issue #10: it just works). P104
/// (user): never a copy of warp itself (~120 MB); without a stub nothing is written
pub(crate) fn runtime_stub_path(kind: &RuntimeStub) -> Result<std::path::PathBuf, String> {
	if let Ok(path) = env::var(kind.variable) {
		return Ok(std::path::PathBuf::from(path));
	}
	let warp = env::current_exe().map_err(|failure| failure.to_string())?;
	let resolved = fs::canonicalize(&warp).unwrap_or_else(|_| warp.clone());
	let next_to_warp = [&warp, &resolved].map(|binary| binary.with_file_name(kind.name));
	let neighbour = next_to_warp.iter().find(|stub| stub.is_file());
	if !in_cargo_target(&resolved) {
		// an installed warp (brew, a release tarball, ~/dev/bin) uses the prebuilt stub shipped next to it and never
		// builds one (user 2026-10-09)
		return neighbour.cloned().ok_or_else(|| {
			format!(
				"no runtime stub {}: {} ships next to warp (brew, the release tarball); put it there or name one in {}",
				next_to_warp[0].display(),
				kind.name,
				kind.variable
			)
		});
	}
	// a debug warp's neighbour in a cargo target is a debug stub (5 MB, it was 21 MB, card g_gFs8): build the release one
	match neighbour.filter(|_| !cfg!(debug_assertions)) {
		Some(stub) => Ok(stub.clone()),
		None => build_runtime_stub(kind, &next_to_warp[0]),
	}
}

/// Whether `binary` lies in a cargo profile folder (debug/, release/, deps/ under them; cargo keeps its .fingerprint
/// there): a development build, which may build its runtime stub from its source checkout
pub(crate) fn in_cargo_target(binary: &std::path::Path) -> bool {
	binary.ancestors().skip(1).take(2).any(|folder| folder.join(".fingerprint").is_dir())
}

/// `cargo build --release -p warp-runtime` in warp's source checkout, whatever warp's own profile: the stub lands next
/// to a release warp built there; its path as cargo reports it
pub(crate) fn build_runtime_stub(kind: &RuntimeStub, expected: &std::path::Path) -> Result<std::path::PathBuf, String> {
	let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	if !source.join(RUNTIME_STUB_CRATE).is_dir() {
		return Err(format!("no runtime stub {}: put warp-runtime there or name one in {} (the warp source {} that would build it is gone)", expected.display(), kind.variable, source.display()));
	}
	// cargo runs every time (it keeps the stub current, quickly once built); the note only before the first build
	let release_stub =
		expected.parent().and_then(std::path::Path::parent).map(|target| target.join("release").join(kind.name));
	if !release_stub.is_some_and(|stub| stub.is_file()) {
		eprintln!("note: building the runtime stub {} once (cargo build -p {})", kind.name, PROGRAM_STUB.name);
	}
	let mut build = std::process::Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
	build
		.args(["build", "--release", "--quiet", "--message-format=json", "-p", PROGRAM_STUB.name, "--bin", kind.name])
		.current_dir(source);
	if !kind.features.is_empty() {
		build.args(["--features", kind.features]);
	}
	let output = build.output().map_err(|failure| format!("cannot run cargo to build the runtime stub: {failure}"))?;
	if !output.status.success() {
		return Err(format!(
			"building the runtime stub {} failed (cargo build -p {} in {}): {}",
			kind.name,
			PROGRAM_STUB.name,
			source.display(),
			String::from_utf8_lossy(&output.stderr).trim()
		));
	}
	String::from_utf8_lossy(&output.stdout)
		.lines()
		.filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
		.find_map(|message| {
			message["executable"].as_str().filter(|path| path.ends_with(kind.name)).map(std::path::PathBuf::from)
		})
		.ok_or_else(|| format!("cargo built {} but named no executable", kind.name))
}
