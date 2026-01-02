#![allow(dead_code, unused_imports)]
// shared code with wasp tests etc
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled
// only lib.rs allows reexporting as:
// use wasp::extensions::*; etc
pub mod extensions;
pub mod util; // reexported for tests

// use crate::extensions::*; // crate for F12
pub use extensions::lists::*;
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::utils::*;
pub mod analyzer;
pub mod compiler;
pub mod emitter;
pub mod node;
pub mod parser;
pub mod run;
pub mod type_kinds;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod wasp_parser;
pub mod wit_emitter;

pub mod ast;

pub mod meta;

pub mod test_utils;
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled
