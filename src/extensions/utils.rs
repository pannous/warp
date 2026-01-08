use std::fs::{create_dir_all, File};
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
//noinspection ALL
use crate::extensions::strings::StringExtensions;

// #[cfg(any(feature = "wasm",test))]
#[cfg(feature = "wasm")]
extern "C" {
	fn download(url: &str) -> String;
} // String not FFI-safe

#[cfg(test)]
fn download(url: &str) -> String {
	String::from("mock".s() + url)
}

#[must_use]
#[cfg(not(feature = "wasm"))]
#[cfg(not(test))]
pub fn download(url: &str) -> String {
	ureq::get(url)
		.call()
		.ok()
		.and_then(|mut r| r.body_mut().read_to_string().ok())
		.unwrap_or_default()
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
		if !parent.exists() {
			if create_dir_all(parent).is_err() {
				return false;
			}
		}
	}

	// Write file
	File::create(filename)
		.and_then(|mut f| f.write_all(bytes))
		.is_ok()
}

//  use log.trace! macro for conditional tracing
// macro_rules! trace {
//     ($($arg:tt)*) => ({
//         #[cfg(feature = "trace")]
//         {
//             println!($($arg)*);
//         }
//     })
// }
