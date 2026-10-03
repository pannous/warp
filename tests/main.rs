#![allow(mixed_script_confusables)]
#![allow(clippy::nonminimal_bool)] // `is!("not true", !true)` spells the expected value like the code
//! One test crate for all tests: every topic folder tests/<topic>/ is a module (its mod.rs lists the files), so cargo
//! links one test binary instead of one per file. Layout: notes/tests_layout.md.
//! Run one file with `cargo test --test tests <file_stem>::`.

mod common;
mod welcoming;
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
mod sweeps;
mod text;
mod types;
mod wasm;
mod web;
mod probe_destructuring;
mod test_c_style;
mod test_float_fields;
mod test_logical_calls;
mod test_like;
mod test_object_arguments;
mod test_records;
mod test_return_type_dispatch;
mod test_spaced_construction;
mod test_text_as_float;
mod test_try_else_value;
mod test_typed_returns;
mod test_tuples;
mod test_user_method_form;
mod test_type_word_user_function;
