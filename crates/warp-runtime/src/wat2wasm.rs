//! wat2wasm <file.wat> <file.wasm>: WAT text assembled into a module (tests/host/test_host.sh, whose warp-host reads no WAT)
fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [wat, wasm] = args.as_slice() else {
		eprintln!("usage: wat2wasm <file.wat> <file.wasm>");
		std::process::exit(2)
	};
	if let Err(problem) = wat::parse_file(wat)
		.map_err(|problem| problem.to_string())
		.and_then(|bytes| std::fs::write(wasm, bytes).map_err(|problem| problem.to_string()))
	{
		eprintln!("wat2wasm: {wat}: {problem}");
		std::process::exit(1)
	}
}
