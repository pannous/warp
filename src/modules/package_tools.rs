//! A package's tool is a prebuilt WebAssembly command, `<name>.wasm` (wasm32-wasip1), run in-process by wasmtime with
//! the package directory as its working directory: one artifact for every platform, and warp never builds a package
//! on the user's machine. It comes from, in this order:
//! 1. the package itself: `<package>/<name>.wasm`
//! 2. the release asset `<name>.wasm` of a pinned package's version tag, downloaded once per machine
//! 3. the package's Rust sources (Cargo.toml) as a last resort, loudly, built into the package's own build directory.
//!    Never into a shared cargo target directory: a fetched crate built there overwrote the binary of a checkout of
//!    the same crate (uniscript 1.0.0, 2026-10-02).
//!
//! A package's library (packages.warp `library: "x"`) is a core module x.wasm with its header x.h, shipped in the package
//! or release assets of the pinned tag, never built here (library_directory); `use <package>` calls it through warp's
//! binding lib/packages/<package>.warp.
//!
//! The build directory is `~/.cache/warp/packages/<name>@<version>.build` for a pinned package (outside its clone)
//! and `packages/.build/<name>` for any other (an unpinned clone, a local checkout linked as packages/<name>).
use crate::modules::{cached_package, fetch_package, package_repository, pinned_version, PACKAGES_DIRECTORY};
use crate::versions::Version;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{p1, FsPerms, I32Exit, WasiCtxBuilder};

const WASI_TARGET: &str = "wasm32-wasip1";
/// rustc's note when the standard library of a target is missing ("… the `wasm32-wasip1` target may not be installed")
pub(crate) const MISSING_TARGET_SIGN: &str = "target may not be installed";
const WASM_EXTENSION: &str = "wasm";
const HEADER_EXTENSION: &str = "h";
const WASM_MAGIC: &[u8] = b"\0asm";
/// `<name>@<version>` + this, next to the pinned clone in the package cache
const BUILD_SUFFIX: &str = ".build";
/// below packages/: the builds of packages that are not pinned clones
const LOCAL_BUILDS: &str = ".build";
const GITHUB: &str = "https://github.com/";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_TOOL_BYTES: u64 = 256 << 20;
/// what a tool may print to each of stdout and stderr
const OUTPUT_CAPACITY: usize = 64 << 20;
/// one artifact fetch or build at a time within a process; cargo locks the target directory across processes
static PROVIDING: Mutex<()> = Mutex::new(());

/// What a tool run printed and its exit status
#[derive(Debug)]
pub struct ToolRun {
	pub status: i32,
	pub stdout: String,
	pub stderr: String,
}

impl ToolRun {
	pub fn success(&self) -> bool {
		self.status == 0
	}
}

/// Runs a package's tool with arguments, in the package directory: `run_package_tool("uniscript", &["check"])`
pub fn run_package_tool(name: &str, arguments: &[&str]) -> Result<ToolRun, String> {
	let directory = fetch_package(name)?;
	let tool = package_tool(name)?;
	run_wasi_command(&tool, name, arguments, &directory)
		.map_err(|failure| format!("package {name}: {} failed: {failure:#}", tool.display()))
}

/// The path of a package's prebuilt tool `<name>.wasm`, fetched or (last resort) built once
pub fn package_tool(name: &str) -> Result<PathBuf, String> {
	let _providing = PROVIDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let file = wasm_file(name);
	prebuilt_file(name, &file).or_else(|reason| {
		eprintln!("warning: package {name}: no prebuilt {file}: {reason}");
		let directory = fetch_package(name)?;
		let build = build_directory(name, &directory);
		let artifact = build.join(&file);
		build_from_source(name, &directory, &build, &artifact)?;
		Ok(artifact)
	})
}

/// The directory of a package's library (packages.warp `library`): its core module `<library>.wasm` with the header
/// `<library>.h` beside it, both shipped in the package or both release assets; never built on the user's machine
pub fn library_directory(name: &str, library: &str) -> Result<PathBuf, String> {
	let _providing = PROVIDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let module = prebuilt_file(name, &wasm_file(library)).map_err(|reason| format!("package {name}: {reason}"))?;
	let header = prebuilt_file(name, &format!("{library}.{HEADER_EXTENSION}"))
		.map_err(|reason| format!("package {name}: {reason}"))?;
	match (module.parent(), header.parent()) {
		(Some(directory), Some(beside)) if directory == beside => Ok(directory.to_path_buf()),
		_ => Err(format!("package {name}: {} and {} are not in one directory", module.display(), header.display())),
	}
}

/// A prebuilt file of a package: shipped in the package itself, else the release asset of its pinned version's tag,
/// downloaded once into the build directory (a tag never changes, nor what was fetched of it). Callers hold PROVIDING.
fn prebuilt_file(name: &str, file: &str) -> Result<PathBuf, String> {
	let directory = fetch_package(name)?;
	let shipped = directory.join(file);
	if is_prebuilt(&shipped) {
		return Ok(shipped);
	}
	let Some(version) = pinned_version(name).filter(|version| is_pinned_clone(&directory, name, version)) else {
		return Err(format!("{} ships no {file}, and only a pinned version has release assets", directory.display()));
	};
	let artifact = build_directory(name, &directory).join(file);
	if !is_prebuilt(&artifact) {
		download_release_asset(name, &version, file, &artifact)?;
	}
	Ok(artifact)
}

/// Where a package's built and downloaded files go: beside the pinned clone in the package cache, else below packages/
fn build_directory(name: &str, directory: &Path) -> PathBuf {
	match pinned_version(name).filter(|version| is_pinned_clone(directory, name, version)) {
		Some(version) => PathBuf::from(format!("{}{BUILD_SUFFIX}", cached_package(name, &version).display())),
		None => Path::new(PACKAGES_DIRECTORY).join(LOCAL_BUILDS).join(name),
	}
}

fn wasm_file(name: &str) -> String {
	format!("{name}.{WASM_EXTENSION}")
}

/// A WebAssembly binary, or any other file with content
fn is_prebuilt(path: &Path) -> bool {
	let is_wasm = path.extension().is_some_and(|extension| extension == WASM_EXTENSION);
	std::fs::read(path).is_ok_and(|bytes| if is_wasm { bytes.starts_with(WASM_MAGIC) } else { !bytes.is_empty() })
}

/// packages/<name> is the pinned version's clone itself, not a local checkout or a changed clone standing in for it
fn is_pinned_clone(directory: &Path, name: &str, version: &Version) -> bool {
	let canonical = |path: &Path| path.canonicalize().ok();
	canonical(directory).is_some_and(|directory| Some(directory) == canonical(&cached_package(name, version)))
}

/// `file` of the GitHub release of the version's tag (`v1.2.3` or `1.2.3`)
fn download_release_asset(name: &str, version: &Version, file: &str, artifact: &Path) -> Result<(), String> {
	let repository = package_repository(name).unwrap_or_default();
	let repository = repository.trim_end_matches(".git");
	if !repository.starts_with(GITHUB) {
		return Err(format!("{repository} is not on GitHub, no release assets"));
	}
	let mut failures = vec![];
	for tag in [format!("v{version}"), version.to_string()] {
		let url = format!("{repository}/releases/download/{tag}/{file}");
		match download_bytes(&url) {
			Ok(bytes) if !file.ends_with(WASM_EXTENSION) || bytes.starts_with(WASM_MAGIC) => {
				return write_atomically(artifact, &bytes)
			}
			Ok(_) => failures.push(format!("{url}: not WebAssembly")),
			Err(reason) => failures.push(format!("{url}: {reason}")),
		}
	}
	Err(failures.join("; "))
}

pub(crate) fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
	let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(DOWNLOAD_TIMEOUT)).build().into();
	let mut response = agent.get(url).call().map_err(|failure| failure.to_string())?;
	response.body_mut().with_config().limit(MAX_TOOL_BYTES).read_to_vec().map_err(|failure| failure.to_string())
}

/// Parallel processes each write their own staging file and rename it into place
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
	let parent = path.parent().unwrap_or(Path::new("."));
	std::fs::create_dir_all(parent).map_err(|failure| format!("{}: {failure}", parent.display()))?;
	let staging =
		parent.join(format!(".{}.{}", path.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
	std::fs::write(&staging, bytes)
		.and_then(|()| std::fs::rename(&staging, path))
		.map_err(|failure| format!("{}: {failure}", path.display()))
}

/// The last resort: `cargo build --target wasm32-wasip1` of the package's binary `<name>`, with the package's own
/// target directory (CARGO_TARGET_DIR overrides any build.target-dir of ~/.cargo/config.toml)
fn build_from_source(name: &str, directory: &Path, build: &Path, artifact: &Path) -> Result<(), String> {
	let manifest = directory.join("Cargo.toml");
	if !manifest.is_file() {
		return Err(format!(
			"package {name} ships no {}, has no release asset of it and no Rust sources to build it from",
			wasm_file(name)
		));
	}
	let target = build.join("target");
	eprintln!("warning: package {name}: building {} from its sources into {}", wasm_file(name), target.display());
	let output = Command::new("cargo")
		.args(["build", "--quiet", "--release", "--target", WASI_TARGET, "--bin", name, "--manifest-path"])
		.arg(&manifest)
		.env("CARGO_TARGET_DIR", &target)
		.output()
		.map_err(|failure| format!("package {name}: cargo: {failure}"))?;
	if !output.status.success() {
		let errors = String::from_utf8_lossy(&output.stderr);
		if errors.contains(MISSING_TARGET_SIGN) {
			return Err(format!(
				"package {name}: missing rust target {WASI_TARGET}; fix: rustup target add {WASI_TARGET}"
			));
		}
		return Err(format!("package {name}: building {} failed: {}", wasm_file(name), errors.trim()));
	}
	let built = target.join(WASI_TARGET).join("release").join(wasm_file(name));
	let bytes = std::fs::read(&built).map_err(|failure| format!("package {name}: {}: {failure}", built.display()))?;
	write_atomically(artifact, &bytes)
}

/// A WASI preview 1 command (`_start`) with the arguments `name arguments…` and `directory` as its `.`
fn run_wasi_command(tool: &Path, name: &str, arguments: &[&str], directory: &Path) -> anyhow::Result<ToolRun> {
	let engine = Engine::default();
	let module = Module::from_file(&engine, tool)?;
	let (stdout, stderr) = (MemoryOutputPipe::new(OUTPUT_CAPACITY), MemoryOutputPipe::new(OUTPUT_CAPACITY));
	let mut context = WasiCtxBuilder::new();
	context.args(&[name]).args(arguments).stdout(stdout.clone()).stderr(stderr.clone());
	context.preopened_dir(directory, ".", FsPerms::ReadWrite)?;
	let mut store = Store::new(&engine, context.build_p1());
	warp_runtime::engine::cap_memory(&mut store);
	let mut linker: Linker<p1::WasiP1Ctx> = Linker::new(&engine);
	p1::add_to_linker_sync(&mut linker, |context| context)?;
	let start = linker.instantiate(&mut store, &module)?.get_typed_func::<(), ()>(&mut store, "_start")?;
	let status = match start.call(&mut store, ()) {
		Ok(()) => 0,
		Err(trap) => trap.downcast_ref::<I32Exit>().map(|exit| exit.0).ok_or(trap)?,
	};
	drop(store);
	let text = |pipe: &MemoryOutputPipe| String::from_utf8_lossy(&pipe.contents()).into_owned();
	Ok(ToolRun { status, stdout: text(&stdout), stderr: text(&stderr) })
}
