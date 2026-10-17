//! #14e (user decision 2026-10-03): `download <url>` is an alias of `fetch`

use crate::common::{fails_with, serve};
use crate::is;

#[test]
fn download_fetches_like_fetch() {
	is!(&format!("download \"{}\"", serve("200 OK", "hello")), "hello\n");
	is!(&format!("x = download \"{}\"; x", serve("200 OK", "hi")), "hi\n");
}

#[test]
fn a_failed_download_is_an_error_value() {
	fails_with(&format!("download \"{}\"", serve("404 Not Found", "")), "HTTP status 404");
}
