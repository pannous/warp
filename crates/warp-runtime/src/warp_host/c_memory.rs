//! Reads of C memory for programs calling C (notes/ffi_primitives.md), generic: nothing here knows a library.
//! A C pointer the program declares `i64` (`sqlite3_column_blob(…): i64 from 'sqlite3'`) is its address as a number:
//!   std_io("c", "bytes", [address, count])   the `count` bytes there, a list of 0 to 255; [] for NULL or a count ≤ 0
//!   std_io("c", "text", [address])           the NUL-terminated text there; ø for NULL
//! The address must come from C and stay valid while it is read (a blob until the next step), as with ctypes.string_at.
use crate::nodes::{self, Node};

/// The answer of `c.<member>`, None for a member or arguments of another shape
pub fn c(member: &str, arguments: &[Node]) -> Option<Result<Node, String>> {
	match (member, arguments) {
		("bytes", [Node::Int(address), Node::Int(count)]) => Some(Ok(bytes(*address, *count))),
		("text", [Node::Int(address)]) => Some(Ok(text(*address))),
		_ => None,
	}
}

fn bytes(address: i64, count: i64) -> Node {
	let read = if address == 0 || count <= 0 {
		&[][..]
	} else {
		// SAFETY: the program passes an address and length a C function gave it (generic FFI, the caller's contract)
		unsafe { std::slice::from_raw_parts(address as usize as *const u8, count as usize) }
	};
	Node::List(nodes::SQUARE_LIST, read.iter().map(|byte| Node::Int((*byte).into())).collect())
}

fn text(address: i64) -> Node {
	if address == 0 {
		return Node::Empty;
	}
	// SAFETY: as for bytes: a NUL-terminated string a C function gave the program
	let text = unsafe { std::ffi::CStr::from_ptr(address as usize as *const std::ffi::c_char) };
	Node::Text(text.to_bytes().to_vec())
}
