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
