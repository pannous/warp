use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
//noinspection ALL

/// Without the native feature (the compiler in the browser) there is no network: the page's host does the fetching at run time
#[cfg(not(feature = "native"))]
const NO_NETWORK: &str = "no network in this build of warp (compiled without the native feature)";

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
	match ureq::get(url).call().map_err(anyhow::Error::from).and_then(|mut r| r.body_mut().read_to_string().map_err(anyhow::Error::from)) {
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
	let mut response = agent_within(timeout).get(url).call().map_err(reason)?;
	response.body_mut().read_to_string().map_err(reason)
}

/// `post(url, body)` of the stdlib module net (src/std_adapters.rs): the body sent as UTF-8 text, the answer's text
#[cfg(feature = "native")]
pub fn post_within(url: &str, body: &str, timeout: std::time::Duration) -> Result<String, String> {
	let reason = |error| network_reason(error, timeout);
	let mut response = agent_within(timeout).post(url).header("Content-Type", "text/plain; charset=utf-8").send(body).map_err(reason)?;
	response.body_mut().read_to_string().map_err(reason)
}

#[cfg(feature = "native")]
fn agent_within(timeout: std::time::Duration) -> ureq::Agent {
	ureq::Agent::config_builder().timeout_global(Some(timeout)).build().into()
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

pub trait FileExtensions {
	// std::fs::File does not directly expose the file name.
	fn name(&self) -> String;
	fn path(&self) -> String;
}

impl FileExtensions for File {
	fn name(&self) -> String {
		"std::fs::File does not expose the file name. ".to_string()
	}

	fn path(&self) -> String {
		"std::fs::File does not expose the file name or path.".to_string()
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

