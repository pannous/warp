//! The compile itself: its context and memo, compile-time evaluation, function tables, metadata, dead-code removal

pub mod compile_context;
pub(crate) mod compile_time;
pub mod analysis_memo;
pub mod context;
pub mod local;
pub mod function;
pub mod meta;
pub mod dead_functions;
pub mod function_equality;
