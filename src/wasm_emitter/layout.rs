//! Linear-memory access shapes shared by the runtime emitters.

use wasm_encoder::MemArg;

pub const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };
/// A 4-byte aligned i32 access
pub const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
