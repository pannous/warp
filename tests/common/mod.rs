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

/// A command running this checkout's warp binary. Every checkout builds the one shared target/debug/warp, so another
/// worktree's build can replace it while these tests run: at first use it is linked (or copied) to a file of its own,
/// named by this checkout's version (each checkout builds `version = "0.1.1-<branch>"`), and checked by that version
#[cfg(feature = "native")]
pub fn warp_command() -> std::process::Command {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let binary = BINARY.get_or_init(|| {
		let shared = std::path::Path::new(env!("CARGO_BIN_EXE_warp"));
		let own = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("warp-{}", env!("CARGO_PKG_VERSION")));
		let _ = std::fs::remove_file(&own);
		// a hard link keeps this build even when cargo relinks target/debug/warp to a newer one; a copy where links fail
		if std::fs::hard_link(shared, &own).is_err() {
			std::fs::copy(shared, &own).expect("copy the warp binary");
		}
		let version = std::process::Command::new(&own).arg("version").output().expect("warp runs");
		let version = String::from_utf8_lossy(&version.stdout).to_string();
		assert!(version.contains(env!("CARGO_PKG_VERSION")),
			"{} is another checkout's build ({}), not version {}: another worktree built warp in between; run the tests again",
			shared.display(), version.trim(), env!("CARGO_PKG_VERSION"));
		own
	});
	std::process::Command::new(binary)
}
