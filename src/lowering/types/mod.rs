//! Types: classes and traits, sum types, casts, aliases, constructors, type tests, tuples

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod class_methods;
pub mod traits;
pub mod sum_variants;
pub mod optional_casts;
pub mod object_groups;
pub mod type_aliases;
pub mod type_constructor;
pub mod type_tests;
pub mod tuples;
pub mod named_patterns;
