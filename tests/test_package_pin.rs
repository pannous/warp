//! A pinned package (packages.wasp `name: {repository: "git url", version: 1.2.3}`) is the clone of its version's tag,
//! fetched once per machine into ~/.cache/warp/packages and linked as packages/<name>: no fetch per checkout or export

use warp::modules::fetch_package;
use warp::is;

#[test]
fn a_pinned_package_links_its_cached_version() {
	let directory = fetch_package("uniscript").unwrap();
	let target = std::fs::read_link(&directory).expect("packages/uniscript links into the package cache");
	assert!(target.ends_with(".cache/warp/packages/uniscript@1.0.0"), "{}", target.display());
	assert!(directory.join("data/entities.idx").is_file()); // the compiled entity index ships with the tag
	is!("use uniscript version 1.0.0; uniscript(\"<:alpha>\")", "α");
}

/// The analyzer asks for an FFI signature of every name it meets; the system headers are parsed once per process
#[test]
fn header_signatures_are_parsed_once() {
	let first = warp::ffi::get_signatures_from_headers("m");
	assert!(first.contains_key("sqrt"));
	assert!(std::ptr::eq(first, warp::ffi::get_signatures_from_headers("m")));
	assert!(std::ptr::eq(warp::ffi::get_ffi_signatures(), warp::ffi::get_ffi_signatures()));
}
