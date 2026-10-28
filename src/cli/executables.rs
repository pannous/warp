//! The runtime stub an executable is made of (crates/warp-runtime): found next to warp, else built from warp's source
//! checkout; compiler.warp's build command joins it with the program (core/commands/commands.warp write_executable)
use super::*;

/// The crate of the stub in warp's source checkout
pub(crate) const RUNTIME_STUB_CRATE: &str = "crates/warp-runtime";

/// The prebuilt runtimes executables are made of: a stub's binary, the variable naming another, its cargo features
pub(crate) struct RuntimeStub {
	name: &'static str,
	variable: &'static str,
	features: &'static str,
}

pub(crate) const PROGRAM_STUB: RuntimeStub =
	RuntimeStub { name: "warp-runtime", variable: "WARP_RUNTIME_STUB", features: "" };

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
		// builds one 
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
