//! The binaryen tools (wasm-split in route_split.rs), run when on PATH

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

const WASM_OPT: &str = "wasm-opt";
/// -O3 pays only with aggressive inlining: the self-hosted stages run 15% faster (pure-only passes.warp 25.5 → 21.7 s), for
/// ~4 s more on their compile once per lib/ (probes/perf/wasm_opt_bench.sh, notes/performance.md); `-g` keeps the names
/// traps are told by
const SPEED_FLAGS: [&str; 6] = [
	"-O3",
	"--inline-functions-with-loops",
	"--flexible-inline-max-function-size=200",
	"--partial-inlining-ifs=4",
	"-g",
	"--quiet",
];
/// Set to 0: the self-hosted stages run as compiled, without wasm-opt, for a one-off check (`warp rebuild self` of a
/// branch): -O3 takes ~40% of a fresh self build's CPU, at ~240% of a core, more than its 15% saves on one build. A
/// WARP_ setting, so the stages kept this way are other modules than the optimized ones (stage_key)
const OPTIMIZE_SETTING: &str = "WARP_WASM_OPT";

/// The module wasm-opt made faster, for one compiled once and run often (a self-hosted stage); None with
/// WARP_WASM_OPT=0, when wasm-opt is not on PATH, said once, or when it fails, said loudly
pub fn optimized_for_speed(module: &[u8]) -> Option<Vec<u8>> {
	static NOTED: std::sync::Once = std::sync::Once::new();
	if std::env::var(OPTIMIZE_SETTING).is_ok_and(|setting| setting == "0") {
		return None;
	}
	if !available(WASM_OPT) {
		if crate::errors::diagnostic::is_shown(crate::errors::diagnostic::Level::Hint) {
			NOTED.call_once(|| {
				eprintln!("note: install binaryen for wasm-opt, so the self-hosted stages run ~15% faster")
			});
		}
		return None;
	}
	let folder =
		std::env::temp_dir().join(format!("warp-wasm-opt-{}-{:?}", std::process::id(), std::thread::current().id()));
	let input = folder.join("module.wasm");
	let output = folder.join("optimized.wasm");
	let optimized = std::fs::create_dir_all(&folder)
		.and_then(|()| std::fs::write(&input, module))
		.map_err(|failure| failure.to_string())
		.and_then(|()| {
			let run = std::process::Command::new(WASM_OPT)
				.args(SPEED_FLAGS)
				.args(BINARYEN_FEATURES)
				.arg(&input)
				.arg("-o")
				.arg(&output)
				.output()
				.map_err(|failure| failure.to_string())?;
			match run.status.success() {
				true => std::fs::read(&output).map_err(|failure| failure.to_string()),
				false => Err(String::from_utf8_lossy(&run.stderr).trim().to_string()),
			}
		});
	let _ = std::fs::remove_dir_all(&folder);
	optimized.map_err(|failure| eprintln!("warning: {WASM_OPT} failed, the module runs unoptimized: {failure}")).ok()
}
