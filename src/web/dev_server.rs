//! `warp dev app.warp [port]` (card web-dev, notes/web_framework.md step 13): serves the program's site (src/web/site.rs)
//! from memory, with dev.js in its page. The page asks /warp-dev/state for the build's version and failure; a request
//! finding the file or a module it uses changed builds it anew first, so no watcher runs. A new version reloads the page, a failure shows as
//! an overlay with its position, the source line and its fix (web/playground/dev.js), while the last good build serves on.

use crate::web::site::SiteFile;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const DEV_PORT: u16 = 8008;
const STATE_PATH: &str = "/warp-dev/state";
const INDEX_PATH: &str = "/";
const NOT_FOUND: u16 = 404;
const JSON_TYPE: &str = "application/json";
const TEXT_TYPE: &str = "text/plain; charset=utf-8";
/// What opens a URL in the default browser, by system (card dev-opens-browser)
#[cfg(target_os = "macos")]
const BROWSER_OPENER: &[&str] = &["open"];
#[cfg(target_os = "windows")]
const BROWSER_OPENER: &[&str] = &["cmd", "/c", "start", ""];
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const BROWSER_OPENER: &[&str] = &["xdg-open"];

/// The program's latest build: the files of the last good one, the failure of the latest if it failed
struct DevSite {
	program: PathBuf,
	title: String,
	/// the program's file and the module files its last build used (card web-dev-modules), each as it was then
	watched: Vec<(PathBuf, Option<SystemTime>)>,
	version: u64,
	files: Vec<SiteFile>,
	failure: Option<String>,
}

impl DevSite {
	fn new(program: &Path) -> DevSite {
		let title = crate::web::site::program_stem(program);
		DevSite {
			program: program.to_path_buf(),
			files: crate::web::site::dev_shell(&title),
			title,
			watched: vec![],
			version: 0,
			failure: None,
		}
	}

	/// Builds the program anew when its file or a module it uses changed since the last build
	fn refresh(&mut self) {
		if self.version > 0 && self.watched.iter().all(|(file, modified)| modified_time(file) == *modified) {
			return;
		}
		let program_modified = modified_time(&self.program);
		self.version += 1;
		let built = crate::web::site::read_program(&self.program).and_then(|code| {
			crate::modules::with_program_file(&self.program, || crate::web::site::files(&code, &self.title, true))
		});
		let modules =
			crate::compiler::compile_context::read(|context| context.module_files.clone()).into_iter().map(|file| {
				let modified = modified_time(&file);
				(file, modified)
			});
		self.watched = [(self.program.clone(), program_modified)].into_iter().chain(modules).collect();
		match built {
			Ok(files) => (self.files, self.failure) = (files, None),
			Err(failure) => self.failure = Some(failure),
		}
	}

	fn state(&self) -> String {
		json!({ "version": self.version, "error": self.failure }).to_string()
	}

	fn file(&self, path: &str) -> Option<&SiteFile> {
		crate::web::site::file_at(&self.files, path)
	}
}

/// Serves the site of `program` on `port` until the request limit (web_server::stop_after), if any
pub fn serve(program: &Path, port: u16) -> Result<(), String> {
	serve_and_open(program, port, false)
}

/// `warp dev` from a terminal: serves as `serve` does and opens the page in the default browser once it listens
pub fn serve_and_open(program: &Path, port: u16, open: bool) -> Result<(), String> {
	let server = tiny_http::Server::http(("127.0.0.1", port))
		.map_err(|problem| format!("warp dev on port {port}: {problem}"))?;
	let limit = crate::web::web_server::take_request_limit();
	let mut site = DevSite::new(program);
	site.refresh();
	let address = format!("http://localhost:{port}/");
	let opened = open && open_in_browser(&address);
	println!(
		"warp dev: {address} shows {}, built anew when it changes{}",
		program.display(),
		if opened { " (opened in your browser)" } else { "" }
	);
	for (served, request) in server.incoming_requests().enumerate() {
		site.refresh();
		let path = request.url().split('?').next().unwrap_or(INDEX_PATH).to_string();
		let response = match (path.as_str(), site.file(&path)) {
			(STATE_PATH, _) => answer(site.state().into_bytes(), JSON_TYPE),
			(_, Some((name, bytes))) => answer(bytes.clone(), crate::web::site::content_type(name)),
			(_, None) => {
				answer(format!("warp dev: no file {path}").into_bytes(), TEXT_TYPE).with_status_code(NOT_FOUND)
			}
		};
		let _ = request.respond(response);
		if limit > 0 && served + 1 >= limit {
			break;
		}
	}
	Ok(())
}

/// Opens `url` in the default browser; whether that started
pub fn open_in_browser(url: &str) -> bool {
	let (opener, arguments) = BROWSER_OPENER.split_first().expect("an opener");
	std::process::Command::new(opener).args(arguments).arg(url).spawn().is_ok()
}

fn modified_time(file: &Path) -> Option<SystemTime> {
	std::fs::metadata(file).and_then(|metadata| metadata.modified()).ok()
}

fn answer(body: Vec<u8>, content_type: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
	let header = tiny_http::Header::from_bytes("Content-Type", content_type).expect("a valid header");
	tiny_http::Response::from_data(body).with_header(header)
}
