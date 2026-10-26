//! The web as the harness keeps it on `pure` (card rust-bootstrap): fetches, the server a program's routes answer on,
//! web sockets, the browser opener and the files a program reads. Building pages, sites and deployments is compile
//! work, compiler.warp's once it has it; the browser playground's Rust compiler left with the Rust compiler.

#[cfg(feature = "native")]
pub mod fetches;
#[cfg(feature = "native")]
pub mod route_paths;
pub mod web_server;

/// What opens a URL in the default browser, by system (card dev-opens-browser)
#[cfg(target_os = "macos")]
const BROWSER_OPENER: &[&str] = &["open"];
#[cfg(target_os = "windows")]
const BROWSER_OPENER: &[&str] = &["cmd", "/c", "start", ""];
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const BROWSER_OPENER: &[&str] = &["xdg-open"];

/// Opens `url` in the default browser; whether that started
pub fn open_in_browser(url: &str) -> bool {
	let (opener, arguments) = BROWSER_OPENER.split_first().expect("an opener");
	std::process::Command::new(opener).args(arguments).arg(url).spawn().is_ok()
}
#[cfg(feature = "native")]
pub mod web_sockets;

/// The text of a file (module sources, C headers): `None` when missing
pub fn read_text(path: &str) -> Option<String> {
	read_bytes(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Is there a file at `path`
pub fn file_exists(path: &str) -> bool {
	std::path::Path::new(path).is_file()
}

/// The bytes of a file, `None` when missing
pub fn read_bytes(path: &str) -> Option<Vec<u8>> {
	std::fs::read(path).ok()
}
