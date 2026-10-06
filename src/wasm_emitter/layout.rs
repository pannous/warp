//! Linear-memory access shapes shared by the runtime emitters.

use wasm_encoder::MemArg;

pub const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };
/// A 4-byte aligned i32 access
pub const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };

/// UTF-8: the code point ranges of 1-, 2-, 3- and 4-byte sequences, their lead bytes, and the payload bits each byte keeps
pub mod utf8 {
	/// the first code point needing two bytes, and the first lead byte that is no ASCII
	pub const ONE_BYTE_END: i32 = 0x80;
	pub const TWO_BYTE_END: i32 = 0x800;
	pub const THREE_BYTE_END: i32 = 0x10000;
	pub const MAX_CODE_POINT: i32 = 0x10FFFF;
	pub const TWO_BYTE_LEAD: i32 = 0xC0;
	pub const THREE_BYTE_LEAD: i32 = 0xE0;
	pub const FOUR_BYTE_LEAD: i32 = 0xF0;
	pub const ONE_BYTE_PAYLOAD: i32 = 0x7F;
	pub const TWO_BYTE_PAYLOAD: i32 = 0x1F;
	pub const THREE_BYTE_PAYLOAD: i32 = 0x0F;
	pub const FOUR_BYTE_PAYLOAD: i32 = 0x07;
	/// a continuation byte is 10xxxxxx: `byte & CONTINUATION_MASK == CONTINUATION_MARK`, 6 payload bits
	pub const CONTINUATION_MASK: i32 = 0xC0;
	pub const CONTINUATION_MARK: i32 = 0x80;
	pub const CONTINUATION_PAYLOAD: i32 = 0x3F;
	pub const PAYLOAD_BITS: i32 = 6;
}
