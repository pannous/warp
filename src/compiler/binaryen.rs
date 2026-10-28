//! Binaryen's wasm-opt over the compiler `warp rebuild` writes (card wasm-untrimmed-output). Twin of core/commands.warp
//! WASM_OPT, which `warp build` trims programs with: the rebuild runs before any compiler that could run that one, and
//! goes once rebuilding moves into compiler.warp as building did
use std::path::Path;

const OPTIMIZER: &str = "wasm-opt";
const INSTALL: &str = "brew install binaryen";
/// What wasmtime runs; Binaryen's `-all` would add custom descriptors, whose exact references wasmtime refuses
const WASMTIME_FEATURES: [&str; 12] = [
	"--enable-gc",
	"--enable-reference-types",
	"--enable-bulk-memory",
	"--enable-multivalue",
	"--enable-sign-ext",
	"--enable-nontrapping-float-to-int",
	"--enable-mutable-globals",
	"--enable-tail-call",
	"--enable-simd",
	"--enable-extended-const",
	"--enable-multimemory",
	"--enable-exception-handling",
];
/// The name section stays, so traps and results still name their functions
const KEEP_NAMES: &str = "--debuginfo";
/// The passes that optimize a module whole
pub const OPTIMIZE: &[&str] = &["-O2"];

/// The module at `path` rewritten in place by wasm-opt's `passes`; its new bytes, or why it could not be
pub fn rewritten(path: &Path, passes: &[&str]) -> Result<Vec<u8>, String> {
	let ran = std::process::Command::new(OPTIMIZER)
		.args(WASMTIME_FEATURES)
		.arg(KEEP_NAMES)
		.args(passes)
		.arg(path)
		.arg("-o")
		.arg(path)
		.status()
		.map_err(|_| format!("{OPTIMIZER} is not installed ({INSTALL})"))?;
	if !ran.success() {
		return Err(format!("{OPTIMIZER} failed on {}", path.display()));
	}
	std::fs::read(path).map_err(|failure| format!("{}: {failure}", path.display()))
}
