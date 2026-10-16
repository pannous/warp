#[cfg(feature = "native")]
pub mod twins; // card self-differential
use warp::wasm_emitter::eval;
use warp::Node;

/// Set to "true" in Claude Code cloud sessions
const CLOUD_SESSION_VAR: &str = "CLAUDE_CODE_REMOTE";
/// The browser suite stops a test after 120 s and running every sample came close and then over it: a test over all
/// samples is SAMPLE_SLICES tests, each over one slice
const SAMPLE_SLICES: usize = 4;
/// How a failed request reads when it never got an answer (src/extensions/utils.rs network_reason, ureq's io errors,
/// host-files.js): worth one retry
const NETWORK_FAILURES: [&str; 5] = ["timeout", "timed out", "dns", "connection", "network error"];
/// How it reads when the host answered: a wrong answer, never retried
const ANSWERED: &str = "http status";
#[cfg(feature = "native")]
const SERVER_START_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
/// Set by test.sh once it built the runtime stubs (built_stub)
#[cfg(feature = "native")]
const STUBS_BUILT_VARIABLE: &str = "WARP_TEST_STUBS_BUILT";
/// The echo server of echo_url: what a POST sent, as httpbin.org/post answers it
#[cfg(feature = "native")]
const ECHO_SERVER: &str = "post \"/post\" { {data: request.body, headers: request.headers} }";

/// The sample programs, one folder per chapter of the playground's guide (notes/samples.md)
pub const SAMPLES: &str = "samples";
/// Folders of samples/ that hold no menu samples: golf counts bytes, wit holds interfaces, tests links the C++ ones
const NOT_CHAPTERS: [&str; 3] = ["golf", "wit", "tests"];

/// samples/<chapter>/<name>.warp, every one, sorted
pub fn sample_files() -> Vec<std::path::PathBuf> {
	let entries =
		|folder: &std::path::Path| std::fs::read_dir(folder).into_iter().flatten().flatten().map(|entry| entry.path());
	let chapters = entries(std::path::Path::new(SAMPLES)).filter(|path| {
		path.is_dir() && !path.file_name().is_some_and(|name| NOT_CHAPTERS.iter().any(|other| name == *other))
	});
	let mut files: Vec<_> = chapters
		.flat_map(|chapter| entries(&chapter))
		.filter(|path| path.extension().is_some_and(|extension| extension == "warp"))
		.collect();
	files.sort();
	files
}

/// The name of a sample (the playground menu's): its file name without .warp
pub fn sample_name(path: &std::path::Path) -> String {
	path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The file of the sample of that name, in whichever chapter folder (samples/<chapter>/<name>.warp when there is none)
pub fn sample_path(name: &str) -> String {
	let found = sample_files().into_iter().find(|path| sample_name(path) == name);
	found.map(|path| path.to_string_lossy().into_owned()).unwrap_or_else(|| format!("{SAMPLES}/<chapter>/{name}.warp"))
}

/// Every SAMPLE_SLICES-th item from `slice` on
pub fn sample_slice<T>(items: &[T], slice: usize) -> impl Iterator<Item = &T> {
	items.iter().skip(slice).step_by(SAMPLE_SLICES)
}

/// Something a test needs that only some machines have (the Mac's C headers, sibling repositories, local files)
pub struct Resource {
	pub name: &'static str,
	pub available: fn() -> bool,
}

/// macOS SDK headers declare libm plainly (`double sqrt(double);`); glibc's math.h hides them behind __MATHCALL macros
pub const MACOS_C_HEADERS: Resource = Resource {
	name: "the macOS C headers (libm declared without glibc macros)",
	available: || cfg!(target_os = "macos"),
};

/// `lean`/`lake` runs are disabled in cloud sessions (user decision): no toolchain there, and proofs are slow
pub const LEAN: Resource = Resource {
	name: "the Lean toolchain (disabled in cloud sessions)",
	available: || std::env::var(CLOUD_SESSION_VAR).as_deref() != Ok("true"),
};
/// binaryen's wasm-split, which splits a built site's module by route (src/web/route_split.rs)
#[cfg(feature = "native")]
pub const WASM_SPLIT: Resource =
	Resource { name: "binaryen's wasm-split", available: || warp::modules::binaryen::available("wasm-split") };

// The test macros (user decision P100: they live with the tests, not in the library). is!(code, value) compiles and
// runs code through the whole pipeline (parse, analyze, emit, run, read back) and compares the result
#[macro_export]
macro_rules! eq {
	// Evaluate string expressions like "3+3"
	($a:expr, $b:expr) => {{
		assert_eq!($a, $b);
	}};
}

#[macro_export]
macro_rules! is {
	// Evaluate string expressions like "3+3" and roundtrip through WASM
	// Standard comparison for built-in types
	($a:expr, $b:expr) => {{
		let result = ::warp::wasm_emitter::eval($a);
		assert_eq!(result, $b);
	}};
	// For wasm_struct! types: use reverse comparison (Person == Node)
	($a:expr, $b:expr, gc) => {{
		let result = ::warp::wasm_emitter::eval($a);
		assert!($b == result, "is! xxx assertion failed:\n  code: {}\n  expected: {:?}\n  got: {:?}", $a, $b, result);
	}};
}

/// is! for code that reaches another host: retried once when only the network failed it (common::online_eval)
#[macro_export]
macro_rules! is_online {
	($a:expr, $b:expr) => {{
		let result = $crate::common::online_eval($a);
		assert_eq!(result, $b, "{}", $a);
	}};
}

#[macro_export]
macro_rules! skip {
	($($t:tt)*) => {};
}

#[macro_export]
macro_rules! check {
	($cond:expr) => {{
		assert!($cond);
	}};
}

#[macro_export]
macro_rules! put {
        // ($($arg:tt)*) => (println!($($arg)*));
    ($($arg:expr),*) => {{
        $(print!("{:?}", $arg);)*
        println!(); // New line at the end
    }};
}

/// `requires!(RESOURCE);` as the first line of a test: where the resource is missing the test ends here, saying so
/// loudly on stderr (never silently); where it exists the test runs as before
#[macro_export]
macro_rules! requires {
	($resource:expr) => {
		if !($resource.available)() {
			$crate::common::announce_skip($resource.name, module_path!());
			return;
		}
	};
}

/// The paint window's input is one per process (warp_runtime::system_values): a test that tells it holds this until it
/// has read back what it told, so no other test's pointer or click comes between
#[cfg(feature = "native")]
pub fn window_input_held() -> std::sync::MutexGuard<'static, ()> {
	static WINDOW_INPUT: std::sync::Mutex<()> = std::sync::Mutex::new(());
	WINDOW_INPUT.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Written to the stderr handle itself: libtest captures `eprintln!` of passing tests, which would hide the skip
pub fn announce_skip(resource: &str, test_module: &str) {
	use std::io::Write;
	let _ = writeln!(std::io::stderr(), "skipped: needs {resource} ({test_module})");
}

/// How close an @gpu result of sin, cos or exp must come to the CPU's f64 one: a real GPU's f32 within 1e-5; a software
/// adapter only within what WGSL guarantees, 2^-11 for sin and cos on [-π, π]. SwiftShader, the adapter of CI's
/// headless Chrome (named by web/playground/test-worker.js as WARP_GPU_ADAPTER), was off by up to 5.2e-5 (card browser-gpu)
pub fn gpu_tolerance() -> &'static str {
	if software_gpu() {
		"0.00048828125"
	} else {
		"0.00001"
	}
}

/// Whether the tests run on a software WebGPU adapter (gpu_tolerance)
pub fn software_gpu() -> bool {
	std::env::var("WARP_GPU_ADAPTER").is_ok_and(|adapter| adapter.ends_with("(software)"))
}

/// Whether a program's main in the page waits for a foreign call's promise: a browser with JSPI (web/playground
/// test-worker.js WARP_JSPI; Safari has none)
#[cfg(not(feature = "native"))]
pub fn jspi() -> bool {
	std::env::var("WARP_JSPI").is_ok()
}

/// Whether a test may start Chrome (agent-browser): only in CI (user, 2026-10-09: no test launches Chrome or Chromium
/// on the Mac); elsewhere it announces its skip
#[cfg(feature = "native")]
pub fn chrome_runs_here(test_module: &str) -> bool {
	let in_ci = std::env::var_os("CI").is_some();
	if !in_ci {
		announce_skip("Chrome: browser tests run in CI only", test_module);
	}
	in_ci
}

pub fn fails_with(code: &str, needle: &str) {
	match eval(code) {
		Node::Error(message) => assert!(format!("{message}").contains(needle), "{message}"),
		other => panic!("expected an error containing {needle:?} for {code}, got {other:?}"),
	}
}

/// The value of `code` and the compile-time warnings it reported
pub fn warnings_of(code: &str) -> (Node, Vec<String>) {
	warp::errors::diagnostic::take_warnings();
	let value = eval(code);
	(value, warp::errors::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect())
}

/// `code` gives `expected` and warns with a message containing `needle`
pub fn warns_with(code: &str, expected: impl Into<Node>, needle: &str) {
	let (value, warnings) = warnings_of(code);
	assert_eq!(value, expected.into(), "{code}");
	assert!(
		warnings.iter().any(|warning| warning.contains(needle)),
		"no warning containing {needle:?} for {code}: {warnings:?}"
	);
}

/// Local HTTP stub answering every request with `status` and `body`: fetch tests need no network
#[cfg(feature = "native")]
pub fn serve(status: &'static str, body: &'static str) -> String {
	use std::io::{Read, Write};
	let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
	let address = listener.local_addr().unwrap();
	std::thread::spawn(move || {
		for mut stream in listener.incoming().flatten() {
			let mut request = [0u8; 4096];
			let _ = stream.read(&mut request);
			let response =
				format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
			let _ = stream.write_all(response.as_bytes());
		}
	});
	format!("http://{address}/data")
}

/// In the browser the test server answers instead (web/playground/test_in_browser.py /__stub__, its URL in WARP_HTTP_STUB)
#[cfg(not(feature = "native"))]
pub fn serve(status: &'static str, body: &'static str) -> String {
	let stub = std::env::var("WARP_HTTP_STUB").expect("the browser test page names its HTTP stub (wasi.js)");
	format!("{stub}?status={}&body={}", percent_encoded(status), percent_encoded(body))
}

/// A warp server (web_server.rs, end to end) answering POST /post with what it got, as httpbin.org does: {data,
/// headers}; started once per test process on a free port
#[cfg(feature = "native")]
pub fn echo_url() -> String {
	static URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
	URL.get_or_init(|| {
		let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
		std::thread::spawn(move || warp::pipeline::serving_at(port, || eval(ECHO_SERVER).serialize()));
		let started = std::time::Instant::now();
		while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
			assert!(started.elapsed() < SERVER_START_LIMIT, "the echo server did not start");
			std::thread::sleep(std::time::Duration::from_millis(50));
		}
		format!("http://127.0.0.1:{port}/post")
	})
	.clone()
}

/// In the browser the test server echoes a POST to its stub (web/playground/test_in_browser.py /__stub__)
#[cfg(not(feature = "native"))]
pub fn echo_url() -> String {
	std::env::var("WARP_HTTP_STUB").expect("the browser test page names its HTTP stub (wasi.js)")
}

/// `attempt` once more when it failed for the network alone (`network_failure` names why), logged loudly: a slow or
/// unreachable host fails a test only twice in a row. A wrong answer is never retried
pub fn retried_on_network_failure<T>(
	what: &str,
	attempt: impl Fn() -> T,
	network_failure: impl Fn(&T) -> Option<String>,
) -> T {
	let first = attempt();
	let Some(reason) = network_failure(&first) else {
		return first;
	};
	eprintln!("⚠️  RETRY {what}: network failure ({reason}), trying once more");
	attempt()
}

/// The reason of an error value from a request that got no answer (no connection, DNS, timeout), else None
pub fn network_failure_of(value: &Node) -> Option<String> {
	let Node::Error(message) = value.drop_meta() else {
		return None;
	};
	let reason = message.to_string();
	let lower = reason.to_lowercase();
	(!lower.contains(ANSWERED) && NETWORK_FAILURES.iter().any(|failure| lower.contains(failure))).then_some(reason)
}

/// The value of `code`, which reaches another host: run again once when the network failed it
pub fn online_eval(code: &str) -> Node {
	retried_on_network_failure(code, || eval(code), network_failure_of)
}

#[cfg(not(feature = "native"))]
fn percent_encoded(text: &str) -> String {
	text.bytes()
		.map(|byte| if byte.is_ascii_alphanumeric() { (byte as char).to_string() } else { format!("%{byte:02X}") })
		.collect()
}

/// A directory for the scratch files of `name`, unique to this test process: below the system's temp dir, or below /tmp
/// of the browser's in-memory file system (web/playground/wasi.js; every browser test is an instance of its own), where
/// std::env::temp_dir and std::process::id panic
pub fn scratch_directory(name: &str) -> std::path::PathBuf {
	#[cfg(feature = "native")]
	return std::env::temp_dir().join(format!("{name}_{}", std::process::id()));
	#[cfg(not(feature = "native"))]
	return std::path::PathBuf::from(
		std::env::var("TMPDIR").expect("the browser test page names its temp dir (wasi.js)"),
	)
	.join(name);
}

/// What the warp binary writes to stdout running `code` (no questions asked)
#[cfg(feature = "native")]
pub fn printed(code: &str) -> String {
	let output = warp_command().args(["--no-ask", code]).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string()
}

/// The warp-runtime stub executables are built from (P104: warp never copies itself into one): built once per test run
/// and kept as this checkout's own copy, as warp_command keeps its warp. Never the shared target/debug/warp-runtime:
/// another checkout's build replaces it at any moment (card stub-race); this checkout's build in deps is found by
/// its dep-info, which names the checkout (BUILT_FROM in crates/warp-runtime/src/main.rs)
#[cfg(feature = "native")]
pub fn runtime_stub() -> &'static std::path::Path {
	static STUB: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	STUB.get_or_init(|| built_stub("warp-runtime", &[]))
}

/// The stub of page apps (card build-links): the runtime with the web view, as runtime_stub keeps the plain one
#[cfg(all(feature = "native", target_os = "macos"))]
pub fn page_runtime_stub() -> &'static std::path::Path {
	static STUB: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	STUB.get_or_init(|| built_stub("warp-page-runtime", &["--features", "webview"]))
}

#[cfg(feature = "native")]
fn built_stub(name: &str, features: &[&str]) -> std::path::PathBuf {
	// test.sh builds both stubs before the run: cargo here would wait for the shared target dir's lock while other
	// worktrees build (notes/test_speed.md)
	if std::env::var_os(STUBS_BUILT_VARIABLE).is_none() {
		let built = std::process::Command::new(env!("CARGO"))
			.args(["build", "--offline", "--quiet", "-p", "warp-runtime", "--bin", name])
			.args(features)
			.current_dir(env!("CARGO_MANIFEST_DIR"))
			.status()
			.expect("cargo runs");
		assert!(built.success(), "cargo build -p warp-runtime --bin {name} failed");
	}
	let shared = std::path::Path::new(env!("CARGO_BIN_EXE_warp")).with_file_name(name);
	let runtime_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates").join("warp-runtime");
	let build = checkout_build(
		&shared,
		&format!("{}-", name.replace('-', "_")),
		&runtime_manifest.to_string_lossy(),
		Some(env!("CARGO_PKG_VERSION")),
	)
	.unwrap_or_else(|| {
		panic!(
			"target/debug/deps holds no {name} {} built from {}",
			env!("CARGO_PKG_VERSION"),
			runtime_manifest.display()
		)
	});
	own_copy(&build, name).expect("copy the runtime stub")
}

/// The command's output, or a panic once it ran `limit` without ending (killed first): a run that should end, tested
#[cfg(feature = "native")]
pub fn output_within(command: &mut std::process::Command, limit: std::time::Duration) -> std::process::Output {
	use std::process::Stdio;
	let mut run = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("the command starts");
	let started = std::time::Instant::now();
	while run.try_wait().expect("the command runs").is_none() {
		if started.elapsed() > limit {
			run.kill().expect("stop the command");
			panic!("still running after {limit:?}: {:?}", run.wait_with_output().map(|output| output.stdout));
		}
		std::thread::sleep(std::time::Duration::from_millis(50));
	}
	run.wait_with_output().expect("the command's output")
}

/// The command line arguments a program runs with in the tests: the lines of `<path>.args` beside it (path without its
/// extension: samples/golf/rot13 reads samples/golf/rot13.args), none without that file
#[cfg(feature = "native")]
pub fn arguments_beside(path: &std::path::Path) -> Vec<String> {
	let file = path.with_file_name(format!("{}.args", path.file_name().unwrap_or_default().to_string_lossy()));
	std::fs::read_to_string(file).map(|lines| lines.lines().map(str::to_string).collect()).unwrap_or_default()
}

/// A command running this checkout's warp binary. Every checkout builds the one shared target/debug/warp, so another
/// worktree's build can replace it at any moment while these tests run, and versions overlap (`0.1.1` is a prefix of
/// every branch's `0.1.1-<branch>`). So the binary is this checkout's own build in target/debug/deps (checkout_build:
/// its dep-info names this manifest directory and version), copied once to a file keyed by this checkout's path,
/// checked by its exact version; the shared path only where deps holds none
#[cfg(feature = "native")]
pub fn warp_command() -> std::process::Command {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let binary = BINARY.get_or_init(|| {
		let shared = std::path::Path::new(env!("CARGO_BIN_EXE_warp"));
		let find = || {
			let candidates =
				checkout_build(shared, "warp-", env!("CARGO_MANIFEST_DIR"), Some(env!("CARGO_PKG_VERSION")))
					.into_iter()
					.chain([shared.to_path_buf()]);
			candidates.filter_map(|candidate| own_copy(&candidate, "warp").ok()).find(|own| is_this_version(own))
		};
		// another checkout's build can replace the shared binary before this one was found: build this checkout's anew
		let own = find().or_else(|| {
			let built = std::process::Command::new(env!("CARGO"))
				.args(["build", "--offline", "--quiet", "--bin", "warp"])
				.current_dir(env!("CARGO_MANIFEST_DIR"))
				.status()
				.is_ok_and(|status| status.success());
			built.then(find).flatten()
		});
		own.unwrap_or_else(|| {
			panic!(
				"neither target/debug/deps nor {} holds a warp of version {} built from {}, also after building it",
				shared.display(),
				env!("CARGO_PKG_VERSION"),
				env!("CARGO_MANIFEST_DIR")
			)
		})
	});
	// hints chosen explicitly (card hints-toggle): the tests read them without the closing "hide hints with" line
	let mut command = std::process::Command::new(binary);
	command.env("WARP_HINTS", "1");
	// a sample that paints writes PNGs here, no window (src/graphics/paint.rs)
	command.env(warp::graphics::paint::NO_WINDOW_VARIABLE, "1");
	command
}

/// A copy of `binary` that only this checkout writes: named by the checkout's version and path, put in place by a
/// rename, so a run never sees a half-written file nor another checkout's
#[cfg(feature = "native")]
fn own_copy(binary: &std::path::Path, name: &str) -> std::io::Result<std::path::PathBuf> {
	use std::hash::{Hash, Hasher};
	let mut checkout = std::collections::hash_map::DefaultHasher::new();
	env!("CARGO_MANIFEST_DIR").hash(&mut checkout);
	let directory = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
	let own = directory.join(format!("{name}-{}-{:x}", env!("CARGO_PKG_VERSION"), checkout.finish()));
	let partial = directory.join(format!("{name}-{:x}.partial{}", checkout.finish(), std::process::id()));
	let _ = std::fs::remove_file(&partial);
	// a hard link keeps this build when the linker writes a newer one (a new file); a copy where links fail
	if std::fs::hard_link(binary, &partial).is_err() {
		std::fs::copy(binary, &partial)?;
	}
	std::fs::rename(&partial, &own)?;
	Ok(own)
}

/// Whether `warp version` of `binary` names exactly this checkout's version
#[cfg(feature = "native")]
fn is_this_version(binary: &std::path::Path) -> bool {
	let output = std::process::Command::new(binary).arg("version").output();
	output.is_ok_and(|output| {
		String::from_utf8_lossy(&output.stdout).split_whitespace().last() == Some(env!("CARGO_PKG_VERSION"))
	})
}

/// This checkout's own build of a binary in target/debug/deps, where each package version keeps its own file
/// (`warp-<hash>`): the newest one whose dep-info names this manifest directory (and version), no rebuild needed
#[cfg(feature = "native")]
fn checkout_build(
	uplifted: &std::path::Path,
	prefix: &str,
	manifest_dir: &str,
	version: Option<&str>,
) -> Option<std::path::PathBuf> {
	let deps = uplifted.parent()?.join("deps");
	let names_this_checkout = |info: &str| {
		info.lines().any(|line| line == format!("# env-dep:CARGO_MANIFEST_DIR={manifest_dir}"))
			&& version
				.is_none_or(|version| info.lines().any(|line| line == format!("# env-dep:CARGO_PKG_VERSION={version}")))
	};
	std::fs::read_dir(deps)
		.ok()?
		.flatten()
		.map(|entry| entry.path())
		.filter(|path| {
			path.extension().is_some_and(|extension| extension == "d")
				&& path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(prefix))
		})
		.filter(|info| std::fs::read_to_string(info).is_ok_and(|text| names_this_checkout(&text)))
		.map(|info| info.with_extension(""))
		.filter(|binary| binary.is_file())
		.max_by_key(|binary| binary.metadata().and_then(|metadata| metadata.modified()).ok())
}

/// How many calls the module of `code` makes to the function `name` (by the wasm name section)
pub fn calls_to(code: &str, name: &str) -> usize {
	use wasmparser::{KnownCustom, Name, Operator, Parser, Payload};
	let bytes =
		warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}")).bytes;
	let mut index = None;
	let mut calls = Vec::new();
	for payload in Parser::new(0).parse_all(&bytes) {
		match payload.expect("valid module") {
			Payload::CustomSection(section) => {
				if let KnownCustom::Name(names) = section.as_known() {
					for subsection in names {
						if let Ok(Name::Function(map)) = subsection {
							index = map
								.into_iter()
								.flatten()
								.find(|naming| naming.name == name)
								.map(|naming| naming.index)
								.or(index);
						}
					}
				}
			}
			Payload::CodeSectionEntry(body) => {
				let mut reader = body.get_operators_reader().expect("operators");
				while !reader.eof() {
					if let Operator::Call { function_index } = reader.read().expect("operator") {
						calls.push(function_index);
					}
				}
			}
			_ => {}
		}
	}
	index.map_or(0, |index| calls.iter().filter(|called| **called == index).count())
}

/// The (module, name) of every import of the module `code` compiles to
pub fn imports_of(code: &str) -> Vec<(String, String)> {
	let bytes =
		warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}")).bytes;
	wasmparser::Parser::new(0)
		.parse_all(&bytes)
		.filter_map(|payload| match payload.expect("valid module") {
			wasmparser::Payload::ImportSection(section) => Some(
				section
					.into_imports()
					.map(|import| import.expect("valid import"))
					.map(|import| (import.module.to_string(), import.name.to_string()))
					.collect::<Vec<_>>(),
			),
			_ => None,
		})
		.flatten()
		.collect()
}
