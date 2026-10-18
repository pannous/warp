// under WARP_NO_WINDOW a library's window opener (raylib InitWindow, SDL_CreateWindow) is refused at link time: no test
// or agent run pops a window on the user's screen (user 2026-10-18), and the run says why instead of hanging
use warp::formats::ffi_parser::find_library_headers;

#[test]
fn a_raylib_window_is_refused_under_no_window() {
	if find_library_headers("raylib").is_empty() {
		eprintln!("raylib: NOT INSTALLED - skipping");
		return;
	}
	let run = crate::common::warp_command()
		.env("WARP_NO_WINDOW", "1")
		.args(["--no-ask", "eval", "import InitWindow from 'raylib'\nInitWindow(80, 60, \"no\")\n1"])
		.output()
		.unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	assert!(!run.status.success(), "{said}");
	assert!(said.contains("InitWindow from raylib opens a window: refused under WARP_NO_WINDOW"), "{said}");
}
