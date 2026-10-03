#![allow(mixed_script_confusables)]
#![allow(clippy::nonminimal_bool)] // `is!("not true", !true)` spells the expected value like the code
//! One test crate for all tests: every topic folder tests/<topic>/ is a module (its mod.rs lists the files), so cargo
//! links one test binary instead of one per file. Layout: notes/tests_layout.md.
//! Run one file with `cargo test --test tests <file_stem>::`.

mod common;
mod control;
mod ffi;
mod functions;
mod lists;
mod modules;
mod node;
mod numbers;
mod operators;
mod parser;
mod programs;
mod scope;
mod text;
mod types;
mod wasm;
mod web;
mod probe_destructuring;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod probe_footguns;
mod test_c_style;
mod test_float_fields;
mod test_got_it_warnings;
mod test_it_shadow_warning;
mod test_logical_calls;
mod test_like;
mod test_object_arguments;
mod test_panic_sweep;
mod test_records;
mod test_return_type_dispatch;
mod test_spaced_construction;
mod test_style_dont_care;
mod test_sweep_fixes;
mod test_text_as_float;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_todo;
mod test_try_else_value;
mod test_typed_returns;
mod test_tuples;
mod test_warning_mode;
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
mod test_user_method_form;
mod test_type_word_user_function;
