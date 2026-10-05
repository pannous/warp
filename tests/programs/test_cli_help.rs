// `warp --help`, `warp help`, `warp -h` print the usage list (as a bare `warp` does) and the wiki link
#![cfg(feature = "native")]

#[test]
fn help_prints_the_usage() {
	for flag in ["--help", "help", "-h"] {
		let printed = crate::common::printed(flag);
		assert!(printed.contains("warp eval <code>") && printed.contains("github.com/pannous/warp/wiki"), "{flag}: {printed}");
	}
}
