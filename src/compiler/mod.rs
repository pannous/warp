//! The compile itself: its context and memo, compile-time evaluation, function tables, metadata, dead-code removal

pub mod compile_context;
pub mod meta;
pub mod phase_times;
pub mod node_views;
#[cfg(feature = "native")]
pub mod pure_tests;
pub mod compiler_files;
#[cfg(feature = "native")]
pub mod standalone;
