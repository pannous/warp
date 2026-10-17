//! `import fourty_two` of a WebAssembly core module (`fourty_two.wasm`, or its text `fourty_two.wat`): its exported
//! functions are called like C functions, with the parameter and result types the module declares, and an exported
//! global reads as its value. The program imports them from the module's path (import module name), the runner links
//! each import to the module instantiated once per run (notes/wasm_modules.md).
use crate::ffi::FfiSignature;
use crate::node::{call, Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;
use std::path::Path;

pub const MODULE_EXTENSIONS: [&str; 2] = ["wasm", "wat"];

mod c_calls;
pub use c_calls::*;
mod exports;
pub use exports::*;
#[cfg(feature = "native")]
mod linking;
#[cfg(feature = "native")]
pub use linking::*;
mod program_uses;
pub use program_uses::*;
