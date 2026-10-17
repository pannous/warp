//! The lowering passes: each rewrites the program into forms the later passes and the emitter know (the order is
//! `SOURCE_PHASES` and `MEANING_PASSES` in src/pipeline.rs). The crate root re-exports them (`crate::mutation`).

pub mod folding;
pub(crate) mod nodes;
pub use nodes::{function_head, DefinitionForm, FunctionHead};
pub mod pipes;

// concept folders (notes/restructuring.md): their passes stay reachable as crate::lowering::x
mod signals;
pub use signals::*;
mod data;
pub use data::*;
mod foreign;
pub use foreign::*;
mod web;
pub use web::*;
mod collections;
pub use collections::*;
mod control;
pub use control::*;
mod definitions;
pub use definitions::*;
mod types;
pub use types::*;
mod words;
pub use words::*;
mod welcoming;
pub use welcoming::*;
