//! `warp dev app.warp [port]` (card web-dev, notes/web_framework.md step 13): serves the program's site (src/site.rs)
//! from memory, with dev.js in its page. The page asks /warp-dev/state for the build's version and failure; a request
//! finding the file changed builds it anew first, so no watcher runs. A new version reloads the page, a failure shows as
//! an overlay with its position, the source line and its fix (web/playground/dev.js), while the last good build serves on.

use crate::site::SiteFile;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const DEV_PORT: u16 = 8008;
const STATE_PATH: &str = "/warp-dev/state";
const INDEX_PATH: &str = "/";
const NOT_FOUND: u16 = 404;
const JSON_TYPE: &str = "application/json";
const TEXT_TYPE: &str = "text/plain; charset=utf-8";

/// The program's latest build: the files of the last good one, the failure of the latest if it failed
struct DevSite {
	program: PathBuf,
	title: String,
	modified: Option<SystemTime>,
	version: u64,
	files: Vec<SiteFile>,
	failure: Option<String>,
}

impl DevSite {
	fn new(program: &Path) -> DevSite {
		let title = program.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
		DevSite { program: program.to_path_buf(), files: crate::site::dev_shell(&title), title, modified: None, version: 0, failure: None }
	}

	/// Builds the program anew when its file changed since the last build
	fn refresh(&mut self) {
		let modified = std::fs::metadata(&self.program).and_then(|metadata| metadata.modified()).ok();
		if self.version > 0 && modified == self.modified {
			return;
		}
		self.modified = modified;
		self.version += 1;
		let built = std::fs::read_to_string(&self.program).map_err(|failure| format!("cannot read {}: {failure}", self.program.display()))
			.and_then(|code| crate::modules::with_program_file(&self.program, || crate::site::files(&code, &self.title, true)));
		match built {
			Ok(files) => (self.files, self.failure) = (files, None),
			Err(failure) => self.failure = Some(failure),
		}
	}

	fn state(&self) -> String {
		json!({ "version": self.version, "error": self.failure }).to_string()
	}

	fn file(&self, path: &str) -> Option<&SiteFile> {
		crate::site::file_at(&self.files, path)
	}
}

/// Serves the site of `program` on `port` until the request limit (web_server::stop_after), if any
pub fn serve(program: &Path, port: u16) -> Result<(), String> {
	let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|problem| format!("warp dev on port {port}: {problem}"))?;
	let limit = crate::web_server::take_request_limit();
	let mut site = DevSite::new(program);
	site.refresh();
	println!("warp dev: http://localhost:{port}/ shows {}, built anew when it changes", program.display());
	for (served, request) in server.incoming_requests().enumerate() {
		site.refresh();
		let path = request.url().split('?').next().unwrap_or(INDEX_PATH).to_string();
		let response = match (path.as_str(), site.file(&path)) {
			(STATE_PATH, _) => answer(site.state().into_bytes(), JSON_TYPE),
			(_, Some((name, bytes))) => answer(bytes.clone(), crate::site::content_type(name)),
			(_, None) => answer(format!("warp dev: no file {path}").into_bytes(), TEXT_TYPE).with_status_code(NOT_FOUND),
		};
		let _ = request.respond(response);
		if limit > 0 && served + 1 >= limit {
			break;
		}
	}
	Ok(())
}

fn answer(body: Vec<u8>, content_type: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
	let header = tiny_http::Header::from_bytes("Content-Type", content_type).expect("a valid header");
	tiny_http::Response::from_data(body).with_header(header)
}
