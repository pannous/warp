//! Fixed-width integer types for FFI (user decision D16, 2026-10-03): `byte` (unsigned 8 bit), `int8 … int64`,
//! `uint8 … uint64`. Int stays unbounded; a variable declared with one of these types traps when an assignment or
//! an arithmetic result leaves its range. There is no `overflow` value.

/// Prefix of the runtime function that traps when a value does not fit the type named after it
pub const OVERFLOW_PREFIX: &str = "overflow_of_";

pub struct FixedWidth {
	pub type_name: &'static str,
	pub trap: &'static str,
	pub low: i128,
	pub high: i128,
}

const fn signed(type_name: &'static str, trap: &'static str, bits: u32) -> FixedWidth {
	FixedWidth { type_name, trap, low: -(1 << (bits - 1)), high: (1 << (bits - 1)) - 1 }
}

const fn unsigned(type_name: &'static str, trap: &'static str, bits: u32) -> FixedWidth {
	FixedWidth { type_name, trap, low: 0, high: (1 << bits) - 1 }
}

pub const FIXED_WIDTHS: [FixedWidth; 9] = [
	unsigned("byte", "overflow_of_byte", 8),
	signed("int8", "overflow_of_int8", 8),
	signed("int16", "overflow_of_int16", 16),
	signed("int32", "overflow_of_int32", 32),
	signed("int64", "overflow_of_int64", 64),
	unsigned("uint8", "overflow_of_uint8", 8),
	unsigned("uint16", "overflow_of_uint16", 16),
	unsigned("uint32", "overflow_of_uint32", 32),
	unsigned("uint64", "overflow_of_uint64", 64),
];

pub fn fixed_width(type_name: &str) -> Option<&'static FixedWidth> {
	FIXED_WIDTHS.iter().find(|width| width.type_name == type_name)
}

/// The wider of two fixed widths: `int8 + int32` computes in int32
pub fn wider(a: Option<&'static FixedWidth>, b: Option<&'static FixedWidth>) -> Option<&'static FixedWidth> {
	match (a, b) {
		(Some(a), Some(b)) => Some(if b.high - b.low > a.high - a.low { b } else { a }),
		(a, b) => a.or(b),
	}
}

/// The message of the trap in `overflow_of_<type>`
pub fn overflow_message(type_name: &str) -> String {
	match fixed_width(type_name) {
		Some(width) => format!("{type_name} overflow: the value does not fit {type_name} ({}…{}); fix: declare it int, which is unbounded", width.low, width.high),
		None => format!("{type_name} overflow"),
	}
}
