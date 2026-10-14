//! The lowering passes: each rewrites the program into forms the later passes and the emitter know (the order is
//! `SOURCE_PHASES` and `MEANING_PASSES` in src/pipeline.rs). The crate root re-exports them (`crate::mutation`).

pub mod ambiguous_forms;
pub mod blocks;
pub mod channel_words;
pub mod declarations;
pub mod folding;
pub mod kebab_names;
pub mod library_words;
pub mod list_phrases;
pub mod music_words;
pub mod negated_calls;
pub(crate) mod nodes;
pub use nodes::{function_head, DefinitionForm, FunctionHead};
pub mod number_words;
pub mod of_lookups;
pub mod parse_word;
pub mod phrase_calls;
pub mod phrase_words;
pub mod picked_calls;
pub mod pipes;
pub mod plain_enums;
pub mod soft_keywords;
pub mod std_aliases;
pub mod times_phrases;
pub mod type_name_matching;
pub mod welcome_forms;
pub mod word_operators;
pub mod word_slices;

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
