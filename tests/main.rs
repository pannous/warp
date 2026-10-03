#![allow(mixed_script_confusables)]
#![allow(clippy::nonminimal_bool)] // `is!("not true", !true)` spells the expected value like the code
//! One test crate for all tests: every topic folder tests/<topic>/ is a module (its mod.rs lists the files), so cargo
//! links one test binary instead of one per file. Layout: notes/tests_layout.md.
//! Run one file with `cargo test --test tests <file_stem>::`.

mod common;
mod control;
mod functions;
mod lists;
mod node;
mod numbers;
mod operators;
mod parser;
mod scope;
mod text;
mod types;
mod web;
mod probe_destructuring;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod probe_footguns;
mod test_algo_dijkstra;
mod test_algo_levenshtein;
mod test_algo_life;
mod test_algo_queens;
mod test_algo_sieve;
mod test_algo_sorting;
mod test_all_samples;
mod test_c_style;
mod test_calculator_fixes;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_compile_only;
mod test_download;
mod test_emitter;
mod test_ffi_import_group;
mod test_ffi_warning_once;
mod test_ffi;
mod test_float_fields;
mod test_folder_scope;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_name_registry;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_struct;
mod test_got_it_warnings;
mod test_header_search;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_host;
mod test_host_words;
mod test_include;
mod test_it_shadow_warning;
mod test_kitchensink;
mod test_logical_calls;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_libm_linking;
mod test_like;
mod test_module_cache;
mod test_name_subsection_order;
mod test_object_arguments;
mod test_package_pin;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_package_tools;
mod test_packages;
mod test_panic_sweep;
mod test_records;
mod test_return_type_dispatch;
mod test_samples;
mod test_spaced_construction;
mod test_style_dont_care;
mod test_sweep_fixes;
mod test_text_as_float;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_todo;
mod test_try_else_value;
mod test_typed_returns;
mod test_tuples;
mod test_use_modules;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_versions;
mod test_warning_mode;
mod test_wasi;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_emitter;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_reader;
mod test_wasm;
mod test_wast;
mod test_welcoming_ask;
mod test_welcoming_break;
mod test_welcoming_count_argument;
mod test_welcoming_elements;
mod test_welcoming_empty_push;
mod test_welcoming_globals;
mod test_welcoming_indent;
mod test_welcoming_listexpr;
mod test_welcoming_listparams;
mod test_welcoming_maps;
mod test_welcoming_parse;
mod test_welcoming_print;
mod test_welcoming_rangeblock;
mod test_welcoming_slices;
mod test_welcoming_sugar;
mod test_wasm_names_order;
mod test_wit_types;
mod test_wit;
mod wasm_optimizer_test;
mod test_optimizer_exceptions;
mod test_optimizer_extended_const;
mod test_read_bytes_plain_result;
mod test_user_method_form;
mod test_type_word_user_function;
