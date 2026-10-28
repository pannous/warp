//! `warp build [--trim=<way>]`: the written module without the runtime the program never reaches, trimmed by default
//! (notes/runtime_linking.md "Removing the unused runtime": all ways stay open, chosen per build)
use super::*;

pub(crate) const TRIM_FLAG: &str = "--trim";
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
			Trim::Optimize => warp::compiler::binaryen::OPTIMIZE,
			Trim::None | Trim::Warp => &[],
		}
	}

	/// The module at `path` trimmed in place; its new bytes
	pub(crate) fn apply(self, path: &str, bytes: Vec<u8>) -> Vec<u8> {
		match self {
			Trim::None | Trim::Warp => bytes,
			Trim::Unused | Trim::Optimize => {
				let trimmed = warp::compiler::binaryen::rewritten(std::path::Path::new(path), self.binaryen_passes()).unwrap_or_else(|problem| {
					fail(format!("warp build: {problem}, so it trims nothing; {TRIM_FLAG}=none builds the module untrimmed"))
				});
				println!("trimmed {} to {} bytes", bytes.len(), trimmed.len());
				trimmed
			}
		}
	}
}
