#[cfg(feature = "native")]
#[cfg(feature = "native")]
pub mod twins; // card self-differential // notes/self_hosting_dropped.md Deviations
use warp::wasm_emitter::eval;
use warp::Node;

/// Set to "true" in Claude Code cloud sessions
const CLOUD_SESSION_VAR: &str = "CLAUDE_CODE_REMOTE";
/// Set by tests/nightly.sh: the nightly run checks what the gate only samples (twins.rs)
#[cfg(feature = "native")]
const NIGHTLY_VARIABLE: &str = "WARP_NIGHTLY";
/// The browser suite stops a test after 120 s and running every sample came close and then over it: a test over all
/// samples is SAMPLE_SLICES tests, each over one slice
const SAMPLE_SLICES: usize = 4;
/// How a failed request reads when it never got an answer (src/extensions/utils.rs network_reason, ureq's io errors,
/// host-files.js): worth one retry
const NETWORK_FAILURES: [&str; 5] = ["timeout", "timed out", "dns", "connection", "network error"];
/// How it reads when the host answered: a wrong answer, never retried
const ANSWERED: &str = "http status";
/// The folder below the temp dir holding each test process's scratch_root, named by its process id
#[cfg(feature = "native")]
const SCRATCH_ROOTS: &str = "warp-tests";
/// The TMPDIR of the warp processes a test starts, inside its scratch_root
#[cfg(feature = "native")]
const CHILD_TMPDIR: &str = "tmp";
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

/// samples/<chapter>/<name>.warp, every one, sorted, in the folder the samples run from (samples_run_folder)
pub fn sample_files() -> Vec<std::path::PathBuf> {
	let entries =
		|folder: &std::path::Path| std::fs::read_dir(folder).into_iter().flatten().flatten().map(|entry| entry.path());
	let chapters = entries(samples_run_folder()).filter(|path| {
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

/// Whether this is the nightly run (tests/nightly.sh), which checks in full what the gate samples
#[cfg(feature = "native")]
pub fn nightly() -> bool {
	std::env::var_os(NIGHTLY_VARIABLE).is_some()
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

/// `effects of function` after `code`, as printed: `Pure`, `IO`, `(IO Unsafe)`
pub fn effects_of(code: &str, function: &str) -> String {
	warp::wasm_emitter::eval(&format!("{code}\neffects of {function}")).serialize()
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

/// The scratch files of this test process, also the TMPDIR of the warp processes it starts (warp_command): one folder
/// per process below SCRATCH_ROOTS. Tests leave what they write (card tests-leak: 475809 WAVs in ten days); the next
/// test process removes the roots of ended ones, and the headless sound WAVs ended processes left in the temp dir
#[cfg(feature = "native")]
pub fn scratch_root() -> &'static std::path::Path {
	static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	ROOT.get_or_init(|| {
		let temp = std::env::temp_dir();
		let roots = temp.join(SCRATCH_ROOTS);
		let running = running_processes();
		remove_left_by_ended(&roots, &running, |name| name.parse().ok());
		remove_left_by_ended(&temp.join(warp::sound::FOLDER), &running, sound_file_process);
		let root = roots.join(std::process::id().to_string());
		std::fs::create_dir_all(root.join(CHILD_TMPDIR)).expect("the test scratch root is writable");
		root
	})
}

/// The ids of the running processes (ps); none known when ps fails, so nothing is swept
#[cfg(feature = "native")]
fn running_processes() -> Option<std::collections::HashSet<u32>> {
	let listed = std::process::Command::new("ps").args(["-A", "-o", "pid="]).output().ok()?;
	listed.status.success().then(|| {
		String::from_utf8_lossy(&listed.stdout).split_whitespace().filter_map(|pid| pid.parse().ok()).collect()
	})
}

/// The entries of `folder` whose name names a process (`process_of`) that has ended, removed
#[cfg(feature = "native")]
fn remove_left_by_ended(
	folder: &std::path::Path,
	running: &Option<std::collections::HashSet<u32>>,
	process_of: impl Fn(&str) -> Option<u32>,
) {
	let (Some(running), Ok(entries)) = (running, std::fs::read_dir(folder)) else { return };
	for entry in entries.flatten() {
		let ended = process_of(&entry.file_name().to_string_lossy()).is_some_and(|pid| !running.contains(&pid));
		if ended {
			let path = entry.path();
			let _ = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
		}
	}
}

/// The process that wrote a sound WAV: sound-<process>.wav or sound-<process>-<handle>.wav (src/sound/mod.rs wav_file)
#[cfg(feature = "native")]
fn sound_file_process(name: &str) -> Option<u32> {
	let rest = name.strip_prefix(warp::sound::FILE_STEM)?.strip_prefix('-')?.strip_suffix(".wav")?;
	rest.split('-').next()?.parse().ok()
}

/// A directory for the scratch files of `name`, unique to this test process: below scratch_root, or below /tmp
/// of the browser's in-memory file system (web/playground/wasi.js; every browser test is an instance of its own), where
/// std::env::temp_dir and std::process::id panic
pub fn scratch_directory(name: &str) -> std::path::PathBuf {
	#[cfg(feature = "native")]
	return scratch_root().join(name);
	#[cfg(not(feature = "native"))]
	return std::path::PathBuf::from(
		std::env::var("TMPDIR").expect("the browser test page names its temp dir (wasi.js)"),
	)
	.join(name);
}

/// The folder the samples run from. Natively a scratch copy of the committed samples (git ls-files): what a run keeps
/// beside its program (stored values, database tables) stays out of the checkout, and every test run starts from the
/// samples as committed (card samples-store). The checkout's other top-level entries are linked beside the copy, so
/// paths like ../tests/fixtures still reach them. The browser suite's file system is in memory: samples/ itself
pub fn samples_run_folder() -> &'static std::path::Path {
	#[cfg(feature = "native")]
	{
		static FOLDER: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
		FOLDER.get_or_init(copy_of_the_samples)
	}
	#[cfg(not(feature = "native"))]
	std::path::Path::new(SAMPLES)
}

#[cfg(feature = "native")]
fn copy_of_the_samples() -> std::path::PathBuf {
	use std::fs;
	let root = scratch_directory("samples_run");
	let _ = fs::remove_dir_all(&root);
	let listing =
		std::process::Command::new("git").args(["ls-files", "-z", SAMPLES]).output().expect("git lists the samples");
	for file in listing.stdout.split(|byte| *byte == 0).filter(|file| !file.is_empty()) {
		let file = std::path::Path::new(std::str::from_utf8(file).expect("a sample path in UTF-8"));
		let copy = root.join(file);
		fs::create_dir_all(copy.parent().expect("a sample lies in a folder")).expect("the copy's folder");
		// a link stays a link (samples/shapes.wasm → ../tests/fixtures/…), resolved against the linked siblings
		let copied = match fs::read_link(file) {
			Ok(target) => std::os::unix::fs::symlink(target, &copy),
			Err(_) => fs::copy(file, &copy).map(drop),
		};
		copied.unwrap_or_else(|error| panic!("copying {}: {error}", file.display()));
	}
	let checkout = std::env::current_dir().expect("the checkout");
	for entry in fs::read_dir(&checkout).expect("the checkout").flatten() {
		if entry.file_name() != SAMPLES {
			// the entry's own path, not its resolved one: a dangling link of the checkout stays a dangling link
			std::os::unix::fs::symlink(entry.path(), root.join(entry.file_name()))
				.expect("a link beside the samples' copy");
		}
	}
	root.join(SAMPLES)
}

/// What the warp binary writes to stdout running `code` (no questions asked)
#[cfg(feature = "native")]
pub fn printed(code: &str) -> String {
	let output = warp_command().args(["--no-ask", code]).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string()
}

/// What `warp eval` of `code` says, stdout then stderr: its value, hints and errors as the user sees them
#[cfg(feature = "native")]
pub fn said(code: &str) -> String {
	let run = warp_command().env("WARP_NO_WINDOW", "1").args(["--no-ask", "eval", code]).output().expect("warp runs");
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
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
	// its temporary files (headless sound WAVs, painted PNGs, compiled modules) go where the next test run sweeps them
	command.env("TMPDIR", scratch_root().join(CHILD_TMPDIR));
	command
}

/// The folder holding this checkout's libwarp_runtime (card run-wasm-lib), built once per test run: point
/// DYLD_LIBRARY_PATH / LD_LIBRARY_PATH there and WARP_INCLUDE at runtime_library_include() to `use warp_runtime`
#[cfg(feature = "native")]
pub fn runtime_library_dir() -> &'static std::path::Path {
	static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	DIR.get_or_init(|| {
		let output = std::process::Command::new(env!("CARGO"))
			.args(["rustc", "--offline", "--quiet", "-p", "warp-runtime", "--lib", "--crate-type", "cdylib"])
			.args(["--features", "library", "--message-format=json"])
			.current_dir(env!("CARGO_MANIFEST_DIR"))
			.output()
			.expect("cargo runs");
		assert!(
			output.status.success(),
			"building libwarp_runtime failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
		let library_name = format!("{}warp_runtime{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
		let built = String::from_utf8_lossy(&output.stdout)
			.lines()
			.filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
			.flat_map(|message| message["filenames"].as_array().cloned().unwrap_or_default())
			.filter_map(|file| file.as_str().map(std::path::PathBuf::from))
			.find(|file| file.file_name().is_some_and(|name| name.to_string_lossy() == library_name))
			.expect("cargo names the built libwarp_runtime");
		// the shared target's file is any checkout's latest build: this checkout's copy, under the name dlopen finds
		let own = own_copy(&built, "warp_runtime_library").expect("copy libwarp_runtime");
		let dir = own.with_extension("dir");
		std::fs::create_dir_all(&dir).expect("the library folder");
		std::fs::rename(&own, dir.join(library_name)).expect("libwarp_runtime in its folder");
		dir
	})
}

/// The folder of warp_runtime.h, the header `use warp_runtime` reads
#[cfg(feature = "native")]
pub fn runtime_library_include() -> std::path::PathBuf {
	std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/warp-runtime/include")
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
