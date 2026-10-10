use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
//noinspection ALL

/// Without the native feature (the compiler in the browser) there is no network: the page's host does the fetching at run time
#[cfg(not(feature = "native"))]
const NO_NETWORK: &str = "no network in this build of warp (compiled without the native feature)";

/// `://host/path` (no scheme) takes the page's protocol (web/playground/host-files.js withPageProtocol); without a page
/// it is https (card pannous-com)
const DEFAULT_SCHEME: &str = "https";
const SCHEME_MARK: &str = "://";

pub fn with_default_scheme(url: &str) -> String {
	if url.starts_with(SCHEME_MARK) { format!("{DEFAULT_SCHEME}{url}") } else { url.to_string() }
}

#[cfg(not(feature = "native"))]
pub fn download(url: &str) -> String {
	log::warn!("download {url}: {NO_NETWORK}");
	String::new()
}

// test mode (native): use mock
#[cfg(all(test, feature = "native"))]
pub fn download(url: &str) -> String {
	format!("mock{url}")
}

// normal mode (native): use ureq
#[must_use]
#[cfg(all(feature = "native", not(test)))]
pub fn download(url: &str) -> String {
	match ureq::get(&with_default_scheme(url)).call().map_err(anyhow::Error::from).and_then(|mut r| r.body_mut().read_to_string().map_err(anyhow::Error::from)) {
		Ok(body) => body,
		Err(e) => {
			eprintln!("download failed for {url}: {e}");
			String::new()
		}
	}
}

/// Like `download`, but a failure is a reason instead of an empty string: DNS, connection, timeout, HTTP status >= 400
#[cfg(not(feature = "native"))]
pub fn download_within(_url: &str, _timeout: std::time::Duration) -> Result<String, String> {
	Err(NO_NETWORK.to_string())
}

#[cfg(all(test, feature = "native"))]
pub fn download_within(url: &str, _timeout: std::time::Duration) -> Result<String, String> {
	Ok(download(url))
}

#[cfg(all(feature = "native", not(test)))]
pub fn download_within(url: &str, timeout: std::time::Duration) -> Result<String, String> {
	let reason = |error| network_reason(error, timeout);
	let mut response = agent_within(timeout, true).get(&with_default_scheme(url)).call().map_err(reason)?;
	response.body_mut().read_to_string().map_err(reason)
}

/// `post(url, body, headers)` of the stdlib module net (src/std_adapters.rs): the body sent as UTF-8 text (plain text
/// unless a header names its Content-Type), the answer's text; an answer of status >= 400 is the error, with its body
/// (an API's own reason, `{"error": … "invalid x-api-key"}`)
#[cfg(feature = "native")]
pub fn post_within(url: &str, body: &str, headers: &[(String, String)], timeout: std::time::Duration) -> Result<String, String> {
	let reason = |error| network_reason(error, timeout);
	let mut request = agent_within(timeout, false).post(&with_default_scheme(url)).header("Content-Type", "text/plain; charset=utf-8");
	for (name, value) in headers {
		request = request.header(name, value);
	}
	let mut response = request.send(body).map_err(reason)?;
	let status = response.status().as_u16();
	let answer = response.body_mut().read_to_string().map_err(reason)?;
	if status >= 400 {
		return Err(format!("HTTP status {status}: {answer}"));
	}
	Ok(answer)
}

#[cfg(feature = "native")]
/// `status_is_error`: an answer of status >= 400 fails the call itself, its body unread
fn agent_within(timeout: std::time::Duration, status_is_error: bool) -> ureq::Agent {
	ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(status_is_error).build().into()
}

/// A failed request in a few words: DNS, timeout, HTTP status >= 400, else ureq's own
#[cfg(feature = "native")]
fn network_reason(error: ureq::Error, timeout: std::time::Duration) -> String {
	use std::io::ErrorKind;
	match error {
		ureq::Error::StatusCode(status) => format!("HTTP status {status}"),
		ureq::Error::Timeout(_) => format!("timeout after {} ms", timeout.as_millis()),
		ureq::Error::Io(io) if matches!(io.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => format!("timeout after {} ms", timeout.as_millis()),
		ureq::Error::HostNotFound => "DNS: host not found".to_string(),
		other => other.to_string(),
	}
}

/// Write bytes to a WASM file, creating parent directories if needed
/// Returns true on success, false on error (no panics)
pub fn write_wasm(filename: &str, bytes: &[u8]) -> bool {
	let path = Path::new(filename);

	// Create parent directory if it doesn't exist
	if let Some(parent) = path.parent() {
		if !parent.exists()
			&& create_dir_all(parent).is_err() {
				return false;
			}
	}

	File::create(filename)
		.and_then(|mut f| f.write_all(bytes))
		.is_ok()
}

