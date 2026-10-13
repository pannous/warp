//! warp-page-runtime: the same stub as warp-runtime, built with the web view; a file of its own, since cargo warns
//! when two bin targets share one source file
#[path = "main.rs"]
mod stub;

fn main() {
	stub::main()
}
