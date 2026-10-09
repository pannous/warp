// card try-catch: `try X else Y` catches the failure of a standard library adapter (std_pure / std_io, the host's
// json, file or net), natively and in the browser; uncaught it stays the loud error it was
use crate::common::fails_with;
use crate::is;

#[test]
fn try_catches_an_adapter_failure() {
	is!("use json\ntry parse_json(\"not json\") else 7", 7);
	is!("use json\ntry parse_json(\"5\") else 7", 5);
}

#[test]
fn the_sample_falls_back_to_default_settings() {
	is!("samples/try_failed_json.warp", "light");
}

#[test]
fn the_caught_error_names_the_adapter() {
	fails_with("use json\ntry { parse_json(\"not json\") } catch e { e }", "json.parse");
}

#[test]
fn an_uncaught_adapter_failure_stays_loud() {
	fails_with("use json\nparse_json(\"not json\")", "json.parse");
}

#[test]
#[cfg(feature = "native")] // the file system, httpbin.org: the page's in-memory files take any path
fn try_catches_a_file_or_http_failure() {
	is!("try write(\"/no/such/folder/x.txt\", \"x\") else 7", 7);
	is!("use net\ntry post(\"https://httpbin.org/status/401\", \"x\") else \"caught\"", "caught");
}
