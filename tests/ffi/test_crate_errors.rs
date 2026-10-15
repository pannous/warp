//! A Rust crate's Err is a raised warp error (card try-crate-errors): `try` catches it as it catches json.parse's, and
//! its message is the crate's, without the cached component's path
use crate::common::fails_with;
use crate::is;

#[test]
fn try_catches_a_crate_error() {
	is!("use cargo semver; try semver.Version.parse(\"x\") else 0", 0);
	is!("use cargo semver; try semver.Version.parse(\"1.2.3\").minor else 0", 2);
	is!("use cargo regex; try regex.Regex(\"a(\") else \"bad\"", "bad");
}

#[test]
fn an_uncaught_crate_error_says_only_what_the_crate_said() {
	fails_with("use cargo semver; semver.Version.parse(\"x\")", "unexpected character 'x'");
	let error = warp::wasm_emitter::eval("use cargo semver; semver.Version.parse(\"x\")").serialize();
	assert!(!error.contains(".wasm"), "{error}");
}
