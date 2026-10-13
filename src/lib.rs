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
#[cfg(feature = "native")]
pub use util::gc_engine;
pub mod analysis_memo;
pub mod analyzer;
pub mod binaryen;
pub mod context;
pub mod dead_functions;
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
	ambiguous_forms, blocks, broadcasting, channel_words, class_methods, closures, component_state, component_worlds,
	comprehensions, database_tables, declarations, element_events, event_signals, fetch_signals, field_elements,
	folding, for_loop, foreign_modules, function_values, generator_consumers, generator_expressions, generator_objects,
	generators, getters, glued_attributes, go_blocks, gpu_maps, inlining, introspection, key_names, lambdas,
	late_binding, lazy_ranges, library_words, list_phrases, markup_tags, memoization, meta_entries, min_max,
	music_words, mutation, named_arguments, nested_index, nonlocal_cells, number_keys, object_groups, one_entry_maps,
	optional_casts, overloads, page_html, parallel, parameter_shapes, phrase_calls, phrase_words, picked_calls, pipes,
	printable, references, reflection, result_word, routes, ruby_blocks, run_time_blocks, scoped_handlers,
	select_options, serve, shared_arrays, signal_values, soft_keywords, std_aliases, stored_values, style_rules,
	switch, system_signals, system_values, traits, transitions, tuples, type_aliases, type_constructor,
	type_name_matching, type_tests, undo_history, variable_signals, variadic, versions, welcome_forms, word_operators,
	word_slices,
};
pub mod accessibility;
#[cfg(feature = "native")]
pub mod channels;
mod compile_time;
#[cfg(feature = "native")]
pub mod component_builder;
#[cfg(feature = "native")]
pub mod components;
#[cfg(feature = "native")]
pub mod crash_card;
#[cfg(feature = "native")]
pub mod database;
#[cfg(feature = "native")]
pub mod deploy;
#[cfg(feature = "native")]
pub mod dev_server;
pub mod diagnostic;
pub mod effects;
#[cfg(feature = "native")]
pub mod fetches;
pub mod ffi;
pub mod ffi_parser;
#[cfg(all(feature = "native", target_os = "macos"))]
pub mod file_type;
pub mod fixed_width;
pub mod fixits;
#[cfg(feature = "native")]
pub mod foreign;
pub mod function;
pub mod function_equality;
#[cfg(feature = "native")]
pub mod gpu;
#[cfg(feature = "native")]
pub mod gpu_canvas;
pub mod headless;
pub mod host;
pub mod host_parts;
pub mod injection;
pub mod interpolation;
pub mod law;
pub mod local;
pub mod markup;
pub mod meta;
pub mod meta_section;
#[cfg(feature = "native")]
pub mod midi;
pub mod modules;
pub mod normalize;
pub mod operators;
#[cfg(feature = "native")]
pub mod package_tools;
pub mod page_tests;
#[cfg(feature = "native")]
pub mod paint;
#[cfg(feature = "native")]
pub mod paint_window;
pub mod real;
#[cfg(feature = "native")]
pub mod route_split;
pub mod shader_holes;
#[cfg(feature = "native")]
pub mod shared;
#[cfg(feature = "native")]
pub mod site;
#[cfg(feature = "native")]
pub mod sound;
#[cfg(feature = "native")]
pub mod sound_engine;
#[cfg(feature = "native")]
pub mod std_adapters;
pub mod std_docs;
#[cfg(feature = "native")]
pub mod tasks;
#[cfg(feature = "native")]
pub mod text_raster;
pub mod time;
pub mod uncertain;
pub mod uniscript_entities;
pub mod units;
pub mod warp_parser;
pub mod wasm_modules;
pub mod wasm_optimizer;
#[cfg(feature = "native")]
pub mod wasm_reader;
pub mod web;
pub mod web_idl;
#[cfg(feature = "native")]
pub mod web_server;
#[cfg(feature = "native")]
pub mod web_sockets;
#[cfg(all(feature = "native", target_os = "macos"))]
pub mod webview;
pub mod wisp_parser;

// ==================== Core Re-exports ====================
// Node AST - the heart of warp
pub use node::{Bracket, Node, Separator};
pub use operators::{is_function_keyword, Op, FUNCTION_KEYWORDS};
// Node convenience constructors
pub use node::{
	block, codepoint, data, error, error_node, float, floats, int, ints, key, key_op, key_ops, list, parens, symbol,
	symbols, text, texts,
};
// Node variants (except Number/List which conflict with extension types)
pub use node::Node::{Char, Data, Empty, Error, False, Key, Meta, Symbol, Text, True};
// Parser
pub use warp_parser::{parse, parse_data, parse_file, parse_xml, WarpParser};
pub use wisp_parser::{emit_wisp, parse_wisp, WispEmitter, WispParser};
// Type system
pub use type_kinds::{
	extract_instance_values, AstKind, FieldDef, Kind, NodeKind, RawFieldValue, TypeDef, TypeRegistry,
	USER_TYPE_TAG_START,
};
// Metadata
pub use meta::{DataType, DataValue, LineInfo};
// WASM
pub use wasm_emitter::WasmGcEmitter;
// Host functions
#[cfg(feature = "native")]
pub use host::{create_host_linker, link_host_functions, HostState};
// Functions
pub use function::{kind_to_valtype, Arg, Function, FunctionRegistry, Signature, ABI};
// Local (unified struct for variables)
pub use local::Local;
// Legacy GcObject for backward compatibility (3-field Node layout)
#[cfg(feature = "native")]
pub use wasm_reader::GcObject;
// New gc_traits module with rasm-style ergonomic GC struct access
// Note: gc_struct!, obj!, and wasm_struct! macros are exported at crate root via #[macro_export]
#[cfg(feature = "native")]
pub use gc_traits::{register_gc_types_from_wasm, FieldIndex, FromVal, GcReadable, GcStructWrapper, ToVal};
// GcComparable trait for Node comparison with wasm_struct types
#[cfg(feature = "native")]
pub use node::GcComparable;
