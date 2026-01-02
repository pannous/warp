#![allow(dead_code, unused_imports)]
// shared code with wasp tests etc
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled
// only lib.rs allows reexporting as:
// use wasp::extensions::*; etc
pub mod util;
pub mod extensions;// reexported for tests

// use crate::extensions::*; // crate for F12
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::lists::*;
pub use extensions::utils::*;
pub mod node;
pub mod compiler;
pub mod parser;
pub mod wasp_parser;
pub mod emitter;
pub mod wit_emitter;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod run;
pub mod analyzer;
pub mod type_kinds;

pub mod meta;
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled