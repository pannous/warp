//! Foreign code: modules of other languages, library signatures, GPU maps

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod dollar_names;
pub mod foreign_modules;
pub mod gpu_maps;
pub mod library_signatures;
