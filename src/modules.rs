//! Modules as the harness sees them on `pure` (card rust-bootstrap): where lib/ and the program's file are, the
//! standard modules' sources (warp help), the adapters a program's os words call, and how a used .wasm module links.
//! Resolving `use` into the program is compiler.warp's work.
use std::path::{Path, PathBuf};

mod std_modules;
pub use std_modules::*;
#[cfg(feature = "native")]
pub mod std_adapters;
pub mod std_docs;
pub mod wasm_modules;
pub mod foreign;
mod program_file;
pub use program_file::*;

/// Where modules are looked for, in this order: the current directory, the library (lib/extra: modules
/// that are not standard, like netbase, P194), the modules among the samples (samples/modules: lib, square;
/// samples/wasm: shapes.wasm)
pub const SEARCH_DIRECTORIES: [&str; 9] =
	[".", "include", "lib", "lib/extra", "src", "source", SAMPLE_MODULES[0], SAMPLE_MODULES[1], SAMPLE_MODULES[2]];
/// The samples' modules never stand in for a standard module: samples/modules/agent.warp is a sample, `use agent` from
/// the repository root means lib/agent.warp (card samples-name)
/// samples/numbers: sin.warp, which `use sin` imports (test_sinus_warp_import) and the Numbers chapter links
pub const SAMPLE_MODULES: [&str; 3] = ["samples/modules", "samples/wasm", "samples/numbers"];

pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];

