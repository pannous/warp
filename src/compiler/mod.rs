//! The compile itself: its context and memo, compile-time evaluation, function tables, metadata, dead-code removal

pub mod analysis_memo;
pub mod compile_context;
pub mod compile_mode;
pub(crate) mod compile_time;
pub mod context;
pub mod dead_functions;
pub mod function;
pub mod function_equality;
pub mod local;
pub mod meta;
pub mod node_views;
pub mod run_time_blocks;
pub mod self_hosted;
