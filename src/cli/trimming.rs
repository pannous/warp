//! `warp build [--trim=<way>]`: the written module without the runtime the program never reaches, trimmed by default
//! (notes/runtime_linking.md "Removing the unused runtime": all ways stay open, chosen per build)
use super::*;

pub(crate) const TRIM_FLAG: &str = "--trim";
const BINARYEN_OPTIMIZER: &str = "wasm-opt";
const BINARYEN_INSTALL: &str = "brew install binaryen";
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

/// The ways to trim, by the name `--trim=<way>` gives; a build without the flag, or with a bare `--trim`, optimizes
/// (user: trimmed by default; the faster way unless the other is much smaller for little time: -O2 is a third smaller
/// for 30–60 ms more, notes/runtime_linking.md)
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Trim {
	/// the module as compiled
	None,
	/// Binaryen drops what the exports never reach, the code itself untouched
	Unused,
	/// Binaryen drops it and optimizes the rest (-O2)
	Optimize,
	/// our own linker over the runtime's call-site list (card runtime-shaking, Later)
	Warp,
}

const TRIM_WAYS: [(&str, Trim); 4] =
	[("none", Trim::None), ("unused", Trim::Unused), ("optimize", Trim::Optimize), ("warp", Trim::Warp)];
const DEFAULT_TRIM: Trim = Trim::Optimize;

impl Trim {
	/// The way the leading `--trim[=<way>]` of build arguments names, the default without one; refused before compiling
	/// when it cannot trim yet
	pub(crate) fn of(arguments: &str) -> Trim {
		let leading = arguments.split_whitespace().take_while(|word| word.starts_with("--"));
		match leading.filter_map(|word| word.strip_prefix(TRIM_FLAG)).last() {
			None | Some("") => DEFAULT_TRIM,
			Some(way) => match way.strip_prefix('=').unwrap_or(way) {
				"warp" => fail(
					"warp build --trim=warp: not yet, our own linker is card runtime-shaking; --trim=unused trims now",
				),
				name => {
					TRIM_WAYS.iter().find(|(known, _)| *known == name).map(|(_, trim)| *trim).unwrap_or_else(|| {
						let names: Vec<&str> = TRIM_WAYS.iter().map(|(known, _)| *known).collect();
						fail(format!("warp build {TRIM_FLAG}={name}: no such way, the ways are {}", names.join(", ")))
					})
				}
			},
		}
	}

	fn binaryen_passes(self) -> &'static [&'static str] {
		match self {
			Trim::Unused => &["--remove-unused-module-elements"],
			Trim::Optimize => &["-O2"],
			Trim::None | Trim::Warp => &[],
		}
	}

	/// The module at `path` trimmed in place; its new bytes
	pub(crate) fn apply(self, path: &str, bytes: Vec<u8>) -> Vec<u8> {
		match self {
			Trim::None | Trim::Warp => bytes,
			Trim::Unused | Trim::Optimize => {
				let ran = std::process::Command::new(BINARYEN_OPTIMIZER)
					.args(WASMTIME_FEATURES)
					.arg(KEEP_NAMES)
					.args(self.binaryen_passes())
					.args([path, "-o", path])
					.status()
					.unwrap_or_else(|_| {
						fail(format!(
							"warp build: {BINARYEN_OPTIMIZER} trims the module and is not installed ({BINARYEN_INSTALL}); \
							 {TRIM_FLAG}=none builds it untrimmed"
						))
					});
				if !ran.success() {
					fail(format!(
						"warp build: {BINARYEN_OPTIMIZER} failed to trim {path} ({TRIM_FLAG}=none builds it untrimmed)"
					));
				}
				let trimmed = fs::read(path).unwrap_or_else(|failure| fail(format!("{path}: {failure}")));
				println!("trimmed {} to {} bytes", bytes.len(), trimmed.len());
				trimmed
			}
		}
	}
}
