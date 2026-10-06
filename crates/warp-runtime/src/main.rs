//! warp-runtime: the stub of a standalone executable (`warp build` appends a program to a copy of it)
fn main() {
	match warp_runtime::standalone::run_carried_program() {
		Some(exit_code) => std::process::exit(exit_code),
		None => {
			eprintln!("warp-runtime carries no program: make an executable with `warp build <file>`");
			std::process::exit(2);
		}
	}
}
