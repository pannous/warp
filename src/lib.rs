#![allow(dead_code, unused_imports)]
// shared code with wasp tests etc
// only lib.rs allows reexporting as:
// use warp::extensions::*; etc
// use crate::extensions::*; // crate for F12
pub mod extensions;
pub use extensions::lists::*;
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::utils::*;
pub mod util; // reexported for tests
#[cfg(feature = "native")]
pub use util::gc_engine;
pub mod analyzer;
pub mod node;
#[cfg(feature = "native")]
pub mod run;
pub mod type_kinds;
#[cfg(feature = "native")]
pub mod gc_traits;
pub mod context;
pub mod wasm_emitter;
#[cfg(feature = "native")]
pub mod wasm_reader;
pub mod wasm_optimizer;
pub mod wasp_parser;
pub mod wisp_parser;
pub mod operators;
pub mod meta;
pub mod host;
pub mod ffi;
pub mod ffi_parser;
pub mod function;
pub mod normalize;
pub mod local;
pub mod law;
pub mod function_equality;
pub mod effects;
pub mod injection;
pub mod interpolation;
pub mod diagnostic;
pub mod time;
pub mod real;
pub mod units;
pub mod for_loop;
pub mod type_constructor;
pub mod type_name_matching;
pub mod meta_entries;
pub mod function_values;
pub mod lambdas;
pub mod closures;
pub mod library_words;
pub mod type_tests;
pub mod min_max;
pub mod mutation;
pub mod switch;
pub mod phrase_words;
pub mod declarations;
pub mod fixed_width;
pub mod modules;
#[cfg(feature = "native")]
pub mod package_tools;
pub mod versions;
pub mod web;
pub mod ambiguous_forms;
pub mod tuples;
pub mod traits;
pub mod overloads;

// ==================== Core Re-exports ====================
// Node AST - the heart of wasp
pub use node::{Bracket, Node, Separator};
pub use operators::{is_function_keyword, Op, FUNCTION_KEYWORDS};
// Node convenience constructors
pub use node::{block, codepoint, error, error_node, float, floats, data, int, ints, key, key_op, key_ops, list, parens, symbol, symbols, text, texts};
// Node variants (except Number/List which conflict with extension types)
pub use node::Node::{Char, Data, Empty, Error, False, Key, Meta, Symbol, Text, True};
// Parser
pub use wasp_parser::{parse, parse_data, parse_file, parse_xml, WaspParser};
pub use wisp_parser::{emit_wisp, parse_wisp, WispEmitter, WispParser};
// Type system
pub use type_kinds::{AstKind, NodeKind, Kind, TypeRegistry, TypeDef, FieldDef, USER_TYPE_TAG_START, extract_instance_values, RawFieldValue};
// Metadata
pub use meta::{Dada, LineInfo, DataType};
// WASM
pub use wasm_emitter::{WasmGcEmitter};
// Host functions
#[cfg(feature = "native")]
pub use host::{HostState, link_host_functions, create_host_linker};
// Functions
pub use function::{Function, FunctionRegistry, Signature, Arg, ABI, kind_to_valtype};
// Local (unified struct for variables)
pub use local::Local;
// Legacy GcObject for backward compatibility (3-field Node layout)
#[cfg(feature = "native")]
pub use wasm_reader::GcObject;
// New gc_traits module with rasm-style ergonomic GC struct access
// Note: gc_struct!, obj!, and wasm_struct! macros are exported at crate root via #[macro_export]
#[cfg(feature = "native")]
pub use gc_traits::{register_gc_types_from_wasm, FromVal, ToVal, FieldIndex, GcStructWrapper, GcReadable};
// GcComparable trait for Node comparison with wasm_struct types
#[cfg(feature = "native")]
pub use node::GcComparable;
