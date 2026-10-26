// shared code with warp tests etc
// only lib.rs allows reexporting as:
// use warp::extensions::*; etc
// use crate::extensions::*; // crate for F12
pub mod extensions;
pub use extensions::lists::*;
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::utils::*;
pub mod util; // reexported for tests
/// The words and marks the parser and the lowering passes share
pub mod words;
#[cfg(feature = "native")]
pub use util::gc_engine;
pub mod abi;
pub mod compiled_module;
pub mod compiler;
#[cfg(feature = "native")]
pub mod gc_traits;
/// The lowering passes the pipeline runs (src/pipeline.rs), each a module of its own; re-exported at the crate root
pub mod node;
#[cfg(feature = "native")]
pub mod run;
pub mod type_kinds;
pub mod errors;
pub mod ffi;
pub mod formats;
pub mod graphics;
pub mod host;
pub mod modules;
pub mod numbers;
pub mod operators;
#[cfg(feature = "native")]
pub mod sound;
/// The browser build's part of sound: the one effects and spectrum DSP, which the playground runs too (card shared-buses
/// stage 2, visualizer-spectrum)
#[cfg(not(feature = "native"))]
pub mod sound {
	pub mod effects;
	pub mod spectrum;
}
pub mod time;
pub mod web;

// ==================== Core Re-exports ====================
// Node AST - the heart of warp
pub use node::{Bracket, Node, Separator};
pub use operators::{is_function_keyword, Op, FUNCTION_KEYWORDS};
// Node convenience constructors
pub use node::{
	block, codepoint, data, error, float, floats, int, ints, key, key_ops, list, parens, symbol, symbols, text, texts,
};
// Node variants (except Number/List which conflict with extension types)
pub use node::Node::{Char, Data, Empty, Error, False, Key, Meta, Symbol, Text, True};
// Parser
pub use formats::wisp_parser::{emit_wisp, parse_wisp, WispEmitter, WispParser};
// Type system
pub use type_kinds::{
	extract_instance_values, FieldDef, Kind, NodeKind, RawFieldValue, TypeDef, TypeRegistry, USER_TYPE_TAG_START,
};
// Metadata
pub use compiler::meta::{DataType, DataValue, LineInfo};
// WASM
// Host functions
#[cfg(feature = "native")]
pub use host::{create_host_linker, link_host_functions, HostState};
// Functions
// Local (unified struct for variables)
// New gc_traits module with rasm-style ergonomic GC struct access
// Note: gc_struct!, obj!, and wasm_struct! macros are exported at crate root via #[macro_export]
#[cfg(feature = "native")]
pub use gc_traits::{register_gc_types_from_wasm, FieldIndex, FromVal, GcReadable, GcStructWrapper, ToVal};
