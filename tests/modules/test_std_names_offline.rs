// card playground-module: in the browser a program without a file (the playground's) takes a standard module's name for
// the standard module, without a request (each lookup next to the page was a synchronous 404 on warp.pannous.com); a
// file of a program's own folder still wins (test_std_named_program.rs)
use crate::is;

#[test]
fn a_std_module_name_needs_no_file_lookup() {
	#[cfg(not(feature = "native"))]
	std::fs::write("regex.wasp", "matches(subject, pattern) := 42").expect("the overlay takes the file");
	#[cfg(not(feature = "native"))]
	assert_eq!(warp::modules::module_file_shadowing("regex"), None);
	is!("use regex; matches(\"7\", \"[0-9]\")", true);
}
