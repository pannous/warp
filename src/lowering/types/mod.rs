//! Types: classes and traits, sum types, casts, aliases, constructors, type tests, tuples

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod class_methods;
pub mod conversions;
pub mod inline_unions;
pub mod named_patterns;
pub mod object_groups;
pub mod optional_casts;
pub mod plain_enums;
pub mod sum_variants;
pub mod traits;
pub mod tuples;
pub mod type_aliases;
pub mod type_constructor;
pub mod type_name_matching;
pub mod type_tests;
