use warp::wasm_emitter::eval;
use warp::Node;

/// Set to "true" in Claude Code cloud sessions
const CLOUD_SESSION_VAR: &str = "CLAUDE_CODE_REMOTE";

/// Something a test needs that only some machines have (the Mac's C headers, sibling repositories, local files)
pub struct Resource {
	pub name: &'static str,
	pub available: fn() -> bool,
}

/// macOS SDK headers declare libm plainly (`double sqrt(double);`); glibc's math.h hides them behind __MATHCALL macros
pub const MACOS_C_HEADERS: Resource = Resource { name: "the macOS C headers (libm declared without glibc macros)", available: || cfg!(target_os = "macos") };

/// `lean`/`lake` runs are disabled in cloud sessions (user decision): no toolchain there, and proofs are slow
pub const LEAN: Resource = Resource { name: "the Lean toolchain (disabled in cloud sessions)", available: || std::env::var(CLOUD_SESSION_VAR).as_deref() != Ok("true") };

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

/// Written to the stderr handle itself: libtest captures `eprintln!` of passing tests, which would hide the skip
pub fn announce_skip(resource: &str, test_module: &str) {
	use std::io::Write;
	let _ = writeln!(std::io::stderr(), "skipped: needs {resource} ({test_module})");
}

pub fn fails_with(code: &str, needle: &str) {
	match eval(code) {
		Node::Error(message) => assert!(format!("{message}").contains(needle), "{message}"),
		other => panic!("expected an error containing {needle:?} for {code}, got {other:?}"),
	}
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
			let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
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

#[cfg(not(feature = "native"))]
fn percent_encoded(text: &str) -> String {
	text.bytes().map(|byte| if byte.is_ascii_alphanumeric() { (byte as char).to_string() } else { format!("%{byte:02X}") }).collect()
}

/// A directory for the scratch files of `name`, unique to this test process: below the system's temp dir, or below /tmp
/// of the browser's in-memory file system (web/playground/wasi.js; every browser test is an instance of its own), where
/// std::env::temp_dir and std::process::id panic
pub fn scratch_directory(name: &str) -> std::path::PathBuf {
	#[cfg(feature = "native")]
	return std::env::temp_dir().join(format!("{name}_{}", std::process::id()));
	#[cfg(not(feature = "native"))]
	return std::path::PathBuf::from(std::env::var("TMPDIR").expect("the browser test page names its temp dir (wasi.js)")).join(name);
}

/// What the warp binary writes to stdout running `code` (no questions asked)
#[cfg(feature = "native")]
pub fn printed(code: &str) -> String {
	let output = warp_command().args(["--no-ask", code]).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string()
}

/// The warp-runtime stub executables are built from (P104: warp never copies itself into one): built once per test run
/// and kept as this checkout's own copy, as warp_command keeps its warp
#[cfg(feature = "native")]
pub fn runtime_stub() -> &'static std::path::Path {
	static STUB: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	STUB.get_or_init(|| {
		let built = std::process::Command::new(env!("CARGO")).args(["build", "--offline", "--quiet", "-p", "warp-runtime", "--bin", "warp-runtime"])
			.current_dir(env!("CARGO_MANIFEST_DIR")).status().expect("cargo runs");
		assert!(built.success(), "cargo build -p warp-runtime failed");
		let shared = std::path::Path::new(env!("CARGO_BIN_EXE_warp")).with_file_name("warp-runtime");
		let runtime_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates").join("warp-runtime");
		let build = checkout_build(&shared, "warp_runtime-", &runtime_manifest.to_string_lossy(), None).unwrap_or(shared);
		own_copy(&build, "warp-runtime").expect("copy the warp-runtime stub")
	})
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
			let candidates = checkout_build(shared, "warp-", env!("CARGO_MANIFEST_DIR"), Some(env!("CARGO_PKG_VERSION"))).into_iter().chain([shared.to_path_buf()]);
			candidates.filter_map(|candidate| own_copy(&candidate, "warp").ok()).find(|own| is_this_version(own))
		};
		// another checkout's build can replace the shared binary before this one was found: build this checkout's anew
		let own = find().or_else(|| {
			let built = std::process::Command::new(env!("CARGO")).args(["build", "--offline", "--quiet", "--bin", "warp"])
				.current_dir(env!("CARGO_MANIFEST_DIR")).status().is_ok_and(|status| status.success());
			built.then(find).flatten()
		});
		own.unwrap_or_else(|| panic!("neither target/debug/deps nor {} holds a warp of version {} built from {}, also after building it", shared.display(), env!("CARGO_PKG_VERSION"), env!("CARGO_MANIFEST_DIR")))
	});
	std::process::Command::new(binary)
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
	output.is_ok_and(|output| String::from_utf8_lossy(&output.stdout).split_whitespace().last() == Some(env!("CARGO_PKG_VERSION")))
}

/// This checkout's own build of a binary in target/debug/deps, where each package version keeps its own file
/// (`warp-<hash>`): the newest one whose dep-info names this manifest directory (and version), no rebuild needed
#[cfg(feature = "native")]
fn checkout_build(uplifted: &std::path::Path, prefix: &str, manifest_dir: &str, version: Option<&str>) -> Option<std::path::PathBuf> {
	let deps = uplifted.parent()?.join("deps");
	let names_this_checkout = |info: &str| info.lines().any(|line| line == format!("# env-dep:CARGO_MANIFEST_DIR={manifest_dir}"))
		&& version.is_none_or(|version| info.lines().any(|line| line == format!("# env-dep:CARGO_PKG_VERSION={version}")));
	std::fs::read_dir(deps).ok()?.flatten().map(|entry| entry.path())
		.filter(|path| path.extension().is_some_and(|extension| extension == "d") && path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(prefix)))
		.filter(|info| std::fs::read_to_string(info).is_ok_and(|text| names_this_checkout(&text)))
		.map(|info| info.with_extension(""))
		.filter(|binary| binary.is_file())
		.max_by_key(|binary| binary.metadata().and_then(|metadata| metadata.modified()).ok())
}

/// How many calls the module of `code` makes to the function `name` (by the wasm name section)
pub fn calls_to(code: &str, name: &str) -> usize {
	use wasmparser::{KnownCustom, Name, Operator, Parser, Payload};
	let bytes = warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}")).bytes;
	let mut index = None;
	let mut calls = Vec::new();
	for payload in Parser::new(0).parse_all(&bytes) {
		match payload.expect("valid module") {
			Payload::CustomSection(section) => {
				if let KnownCustom::Name(names) = section.as_known() {
					for subsection in names {
						if let Ok(Name::Function(map)) = subsection {
							index = map.into_iter().flatten().find(|naming| naming.name == name).map(|naming| naming.index).or(index);
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
