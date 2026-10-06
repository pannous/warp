//! Running compiled warp programs without the compiler: warp links this crate for its own runs, and its binary `warp-runtime` (main.rs)
//! is the standalone executable `warp build` appends a program's machine code to (notes/aot.md)
pub mod floats;
pub mod fuel;
pub mod host_words;
#[cfg(feature = "engine")]
pub mod engine;
#[cfg(feature = "engine")]
pub mod libm;
#[cfg(feature = "engine")]
pub mod macho;
#[cfg(feature = "engine")]
pub mod output;
#[cfg(feature = "engine")]
pub mod standalone;
