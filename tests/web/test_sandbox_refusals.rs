// cards sandbox-exec, sandbox-hosted (notes/hosting.md): `warp --sandbox` starts no process (`exec sh"…"` is os.exec
// through std_io, hidden from the compile-time check), calls no C, and writes files only inside its state folder (the
// working directory, or systemd's STATE_DIRECTORY), each refusal a loud error naming the flag; it reaches the network,
// and a pure program runs as always
use crate::common::warp_command;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const REPLY: &str = "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nREACHED";

/// A fresh state folder of the test's own, and a fresh folder outside it
fn folders(test: &str) -> (PathBuf, PathBuf) {
	let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sandbox_refusals").join(test);
	let _ = std::fs::remove_dir_all(&root);
	let (state, outside) = (root.join("state"), root.join("outside"));
	std::fs::create_dir_all(&state).expect("a state folder");
	std::fs::create_dir_all(&outside).expect("an outside folder");
	(state, outside)
}

/// What `warp --sandbox <code>` prints and whether it succeeded, run in `state`
fn sandboxed_run(state: &Path, code: &str) -> (String, bool) {
	let output = warp_command()
		.args(["--no-ask", "--sandbox", code])
		.current_dir(state)
		.env_remove("STATE_DIRECTORY")
		.env("WARP_HINTS", "0")
		.env("WARP_NO_WINDOW", "1")
		.output()
		.expect("warp runs");
	let printed = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
	(printed, output.status.success())
}

fn refused(state: &Path, code: &str, capability: &str) {
	let (printed, succeeded) = sandboxed_run(state, code);
	assert!(
		!succeeded
			&& printed.contains("capability denied")
			&& printed.contains(capability)
			&& printed.contains("--sandbox"),
		"{code}: {printed}"
	);
}

#[test]
fn a_sandboxed_program_starts_no_process_and_calls_no_c() {
	let (state, _) = folders("process");
	refused(&state, "exec sh\"echo SHELL-RAN\"", "process");
	refused(&state, "use os; exec(sh\"echo SHELL-RAN\")", "process");
	assert!(!sandboxed_run(&state, "exec sh\"echo SHELL-RAN\"").0.contains("\"SHELL-RAN\""));
	refused(&state, "import strlen from \"c\"; strlen(\"ab\")", "ffi");
}

#[test]
fn a_sandboxed_program_writes_inside_its_state_folder() {
	let (state, _) = folders("inside");
	let code = "use file; write(\"notes/a.txt\", \"x\"); file.append(\"notes/a.txt\", \"y\"); \
		write_bytes(\"b.bin\", [1, 2]); 3";
	let (printed, succeeded) = sandboxed_run(&state, code);
	assert!(succeeded && printed.contains('3'), "{printed}");
	assert_eq!(std::fs::read_to_string(state.join("notes/a.txt")).ok().as_deref(), Some("xy"));
	assert_eq!(std::fs::read(state.join("b.bin")).ok(), Some(vec![1, 2]));
}

#[test]
fn a_sandboxed_program_writes_nothing_outside_its_state_folder() {
	let (state, outside) = folders("outside");
	let escaped = outside.join("escaped.txt");
	let path = escaped.display();
	refused(&state, &format!("use file; write(\"{path}\", \"x\")"), "file-write");
	refused(&state, &format!("use file; file.append(\"{path}\", \"x\")"), "file-write");
	refused(&state, &format!("use file; write_bytes(\"{path}\", [1, 2])"), "file-write");
	refused(&state, "use file; write(\"../outside/escaped.txt\", \"x\")", "file-write");
	refused(&state, "use file; write(\"notes/../../outside/escaped.txt\", \"x\")", "file-write");
	std::os::unix::fs::symlink(&outside, state.join("link")).expect("a link to the outside folder");
	refused(&state, "use file; write(\"link/escaped.txt\", \"x\")", "file-write");
	std::os::unix::fs::symlink(&escaped, state.join("dangling.txt")).expect("a dangling link");
	refused(&state, "use file; write(\"dangling.txt\", \"x\")", "file-write");
	assert!(!escaped.exists(), "{path} was written");
}

/// The whole request, its body too: answering after the headers alone closed the socket while a POST still sent its
/// body, which then failed with "Invalid argument (os error 22)" (card sandbox-post-flaky)
fn read_request(stream: &mut std::net::TcpStream) {
	let mut request = Vec::new();
	let mut chunk = [0; 4096];
	let complete = |request: &[u8]| {
		let text = String::from_utf8_lossy(request);
		let Some(end) = text.find("\r\n\r\n") else { return false };
		let length = (text[..end].lines())
			.find_map(|line| {
				line.to_ascii_lowercase().strip_prefix("content-length:").map(|n| n.trim().parse().unwrap_or(0))
			})
			.unwrap_or(0);
		request.len() >= end + 4 + length
	};
	while !complete(&request) {
		match stream.read(&mut chunk) {
			Ok(0) | Err(_) => return,
			Ok(read) => request.extend_from_slice(&chunk[..read]),
		}
	}
}

#[test]
fn a_sandboxed_program_reaches_the_network() {
	let (state, _) = folders("network");
	let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a local port");
	let address = listener.local_addr().expect("its address");
	std::thread::spawn(move || {
		for mut stream in listener.incoming().flatten() {
			read_request(&mut stream);
			let _ = stream.write_all(REPLY.as_bytes());
		}
	});
	for code in [format!("fetch \"http://{address}/\""), format!("use net; post(\"http://{address}/\", \"x\")")] {
		let (printed, succeeded) = sandboxed_run(&state, &code);
		assert!(succeeded && printed.contains("REACHED"), "{code}: {printed}");
	}
}

#[test]
fn a_sandboxed_pure_program_still_runs() {
	let (state, _) = folders("pure");
	let (printed, succeeded) =
		sandboxed_run(&state, "fib(n) := if n < 2 then n else fib(n-1) + fib(n-2); [1, 2, 3].map(x => x * fib(10))");
	assert!(succeeded && printed.contains("55") && printed.contains("165"), "{printed}");
}
