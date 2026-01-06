#![allow(dead_code, unused_imports)]
// shared code with wasp tests etc
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled
// only lib.rs allows reexporting as:
// use wasp::extensions::*; etc
// use crate::extensions::*; // crate for F12
pub mod extensions;
pub use extensions::lists::*;
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::utils::*;
pub mod smarty;
pub mod util; // reexported for tests
pub mod analyzer;
pub mod compiler;
pub mod node;
pub mod run;
pub mod type_kinds;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod wasm_optimizer;
pub mod wasp_parser;
pub mod wisp_parser;
pub mod operators;
pub mod ast;
pub mod meta;
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled

// ==================== Core Re-exports ====================
// Node AST - the heart of wasp
pub use node::{Bracket, Node, Op, Separator};
// Node convenience constructors
pub use node::{block, codepoint, error, error_node, float, floats, data, int, ints, key, key_op, key_ops, list, parens, symbol, symbols, text, texts};
// Node variants (except Number/List which conflict with extension types)
pub use node::Node::{Char, Data, Empty, Error, False, Key, Meta, Symbol, Text, True};
// Parser
pub use wasp_parser::{parse, parse_file, parse_xml, WaspParser};
pub use wisp_parser::{emit_wisp, parse_wisp, WispEmitter, WispParser};
// Type system
pub use type_kinds::{AstKind, NodeKind, NodeTag, TypeRegistry, TypeDef, FieldDef, USER_TYPE_TAG_START};
// Metadata
pub use meta::{Dada, LineInfo, DataType};
// WASM
pub use wasm_gc_emitter::WasmGcEmitter;
pub use wasm_gc_reader::GcObject;
