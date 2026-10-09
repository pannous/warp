//! warp-runtime: the stub of a standalone executable (`warp build` appends a program to a copy of it)

/// The checkout a debug stub was built from: naming it puts it in the stub's dep-info, by which the tests pick their
/// own stub among the builds of all checkouts sharing one target dir (tests/common runtime_stub); release stubs,
/// the ones executables ship, don't carry the path
#[cfg(debug_assertions)]
const BUILT_FROM: &str = env!("CARGO_MANIFEST_DIR");
#[cfg(not(debug_assertions))]
const BUILT_FROM: &str = "a release build";

fn main() {
	match warp_runtime::standalone::run_carried_program() {
		Some(exit_code) => std::process::exit(exit_code),
		None => {
			eprintln!("warp-runtime ({} from {BUILT_FROM}) carries no program: make an executable with `warp build <file>`", env!("CARGO_PKG_VERSION"));
			std::process::exit(2);
		}
	}
}
