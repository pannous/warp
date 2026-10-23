//! Reads of C memory for programs calling C (notes/ffi_primitives.md), generic: nothing here knows a library.
//! A C pointer the program declares `i64` (`sqlite3_column_blob(…): i64 from 'sqlite3'`) is its address as a number:
//!   std_io("c", "bytes", [address, count])   the `count` bytes there, a list of 0 to 255; [] for NULL or a count ≤ 0
//!   std_io("c", "text", [address])           the NUL-terminated text there; ø for NULL
//!   std_io("c", "write", [address, bytes])   copies a text's bytes or a list of 0 to 255 there; their count
//!   std_io("c", "load", [address, size])     the unsigned little-endian integer of 1, 2, 4 or 8 bytes there
//!   std_io("c", "store", [address, size, n]) writes n there as `size` bytes; n
//!   std_io("c", "errno", [])                 errno as the last C call left it (strerror names it)
//! Memory to write to comes from C too: calloc and free are libc's (lib/c_memory.warp).
//! The address must come from C and stay valid while it is read (a blob until the next step), as with ctypes.string_at.
use crate::nodes::{self, Node};

/// The answer of `c.<member>`, None for a member or arguments of another shape
pub fn c(member: &str, arguments: &[Node]) -> Option<Result<Node, String>> {
	match (member, arguments) {
		("bytes", [Node::Int(address), Node::Int(count)]) => Some(Ok(bytes(*address, *count))),
		("text", [Node::Int(address)]) => Some(Ok(text(*address))),
		("errno", []) => Some(Ok(Node::Int(warp_runtime::c_calls::last_errno().into()))),
		("write", [Node::Int(address), bytes]) => Some(written(*address, bytes)),
		("load", [Node::Int(address), Node::Int(size)]) => Some(sized(*size).and_then(|size| loaded(*address, size))),
		("store", [Node::Int(address), Node::Int(size), Node::Int(value)]) => {
			Some(sized(*size).and_then(|size| stored(*address, size, *value)))
		}
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

/// The bytes of a text, or of a list of 0 to 255
fn byte_list(bytes: &Node) -> Result<Vec<u8>, String> {
	match bytes {
		Node::Text(bytes) => Ok(bytes.clone()),
		Node::List(_, items) => items
			.iter()
			.map(|item| match item {
				Node::Int(byte) => u8::try_from(*byte).map_err(|_| format!("c.write: {byte} is no byte (0 to 255)")),
				other => Err(format!("c.write: {} is no byte (0 to 255)", other.shown())),
			})
			.collect(),
		Node::Empty => Ok(vec![]),
		other => Err(format!("c.write takes a text or a list of bytes, not {}", other.shown())),
	}
}

fn written(address: i64, bytes: &Node) -> Result<Node, String> {
	let bytes = byte_list(bytes)?;
	if address == 0 {
		return Err("c.write to NULL".into());
	}
	// SAFETY: the program writes into memory a C function (calloc) gave it, as many bytes as it holds (its contract)
	unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as usize as *mut u8, bytes.len()) };
	Ok(Node::Int(bytes.len() as i64))
}

/// 1, 2, 4 or 8, the sizes of C's integers
fn sized(size: i64) -> Result<usize, String> {
	match size {
		1 | 2 | 4 | 8 => Ok(size as usize),
		_ => Err(format!("c.load and c.store take 1, 2, 4 or 8 bytes, not {size}")),
	}
}

fn loaded(address: i64, size: usize) -> Result<Node, String> {
	if address == 0 {
		return Err("c.load from NULL".into());
	}
	let mut bytes = [0u8; 8];
	// SAFETY: as for bytes: `size` bytes a C function gave the program
	unsafe { std::ptr::copy_nonoverlapping(address as usize as *const u8, bytes.as_mut_ptr(), size) };
	Ok(Node::Int(u64::from_le_bytes(bytes) as i64))
}

fn stored(address: i64, size: usize, value: i64) -> Result<Node, String> {
	if address == 0 {
		return Err("c.store to NULL".into());
	}
	let bytes = value.to_le_bytes();
	// SAFETY: as for written
	unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as usize as *mut u8, size) };
	Ok(Node::Int(value))
}
