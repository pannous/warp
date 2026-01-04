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
pub mod emitter;
pub mod node;
pub use node::Node;
pub mod run;
pub mod type_kinds;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod wasm_optimizer;
pub mod wasp_parser;
pub mod ast;
pub mod meta;
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled
