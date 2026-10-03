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
