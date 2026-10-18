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
pub mod analyzer;
pub mod compiler;
#[cfg(feature = "native")]
pub mod gc_traits;
/// The lowering passes the pipeline runs (src/pipeline.rs), each a module of its own; re-exported at the crate root
pub mod lowering;
pub mod node;
pub mod pipeline;
#[cfg(feature = "native")]
pub mod run;
pub mod type_kinds;
pub mod wasm_emitter;
pub use lowering::{
	ambiguous_forms, blocks, broadcasting, class_methods, closures, component_state, component_worlds, comprehensions,
	database_tables, declaration_forms, declarations, element_events, event_signals, fetch_signals, field_elements,
	filled_lists, folding, for_loop, foreign_modules, function_values, getters, go_blocks, gpu_maps, inline_unions,
	inlining, introspection, kebab_names, key_names, lambdas, late_binding, lazy_ranges, library_words, list_mutations,
	markup_tags, members, memoization, meta_entries, min_max, mutation, named_arguments, negated_calls, nested_index,
	nonlocal_cells, number_keys, object_groups, one_entry_maps, optional_casts, overloads, page_html, parallel,
	parameter_shapes, phrase_words, picked_calls, pipes, printable, program_names, references, reflection, result_word,
	routes, run_time_blocks, serve, shared_arrays, signal_values, soft_keywords, std_aliases, stored_values, switch,
	system_signals, system_values, traits, tuples, type_aliases, type_constructor, type_name_matching,
	type_tests, typed_library_words, undo_history, variable_signals, variadic, versions, welcome_forms, word_help,
	word_operators,
};
pub mod errors;
pub mod ffi;
pub mod formats;
pub mod formatting;
pub mod graphics;
pub mod host;
pub mod law;
pub mod modules;
pub mod normalize;
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
pub mod text;
pub mod time;
pub mod warp_parser;
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
pub use warp_parser::{parse, parse_data, parse_file, parse_xml, WarpParser};
// Type system
pub use type_kinds::{
	extract_instance_values, FieldDef, Kind, NodeKind, RawFieldValue, TypeDef, TypeRegistry, USER_TYPE_TAG_START,
};
// Metadata
pub use compiler::meta::{DataType, DataValue, LineInfo};
// WASM
pub use wasm_emitter::WasmGcEmitter;
// Host functions
#[cfg(feature = "native")]
pub use host::{create_host_linker, link_host_functions, HostState};
// Functions
pub use compiler::function::{Function, FunctionRegistry};
// Local (unified struct for variables)
pub use compiler::local::Local;
// New gc_traits module with rasm-style ergonomic GC struct access
// Note: gc_struct!, obj!, and wasm_struct! macros are exported at crate root via #[macro_export]
#[cfg(feature = "native")]
pub use gc_traits::{register_gc_types_from_wasm, FieldIndex, FromVal, GcReadable, GcStructWrapper, ToVal};
