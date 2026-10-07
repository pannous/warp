//! The binaryen tools (wasm-opt, wasm-metadce in wasm_optimizer.rs; wasm-split in route_split.rs), run when on PATH

/// WASM proposals the emitter uses (GC nodes, bulk memory for runtime texts, exceptions for `try`, multi-value results
/// for mag_divmod and the host text imports, tail calls); extended constant expressions are Wasm 3.0 and on by default
/// in wasmtime and wasmparser, so they are on here too. Named one by one: binaryen's --all-features would also allow
/// proposals no browser ships (exact function imports of custom descriptors), and wasm-split then emits them
pub const BINARYEN_FEATURES: [&str; 7] = [
	"--enable-gc",
	"--enable-reference-types",
	"--enable-bulk-memory",
	"--enable-exception-handling",
	"--enable-extended-const",
	"--enable-multivalue",
	"--enable-tail-call",
];

/// Is the binaryen tool on PATH
pub fn available(tool: &str) -> bool {
	std::process::Command::new(tool).arg("--version").output().is_ok_and(|output| output.status.success())
}
