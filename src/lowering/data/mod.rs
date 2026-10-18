//! Data: database tables, stored values, meta entries, reflection and introspection, printable forms, versions

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod database_tables;
pub mod doc_strings;
pub mod introspection;
pub mod members;
pub mod meta_entries;
pub mod printable;
pub mod reflection;
pub mod stored_values;
pub mod tag_entries;
pub mod versions;
pub mod word_help;
