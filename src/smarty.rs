#![allow(unused)]
use crate::extensions::numbers::Number;
use crate::extensions::strings::String;
use crate::node::{int, float, text, Node};
use crate::node::Node::{Empty, Error, True, False};
use crate::wasp_parser::parse;
//
// // todo move these to ABI.h once it is used:
// //	map_header_32 = USE Node!
// #define array_header_length 16 // 4*4 to align with int64
// //#include "ABI.h"
// #define string_header_32 0x10000000 // compatible with String
// #define array_header_32  0x40000000 // compatible with List
// #define buffer_header_32  0x44000000 // incompatible with List!
// //#define map_header_32    0x46000000 // compatible with Map
// #define map_header_32    0x50000000 // compatible with Map
// #define ref_header_32    0x60000000 // index of externref == js object ! in table (export "externref_table") 1 externref
// //#define smart_mask_32    0x70000000 ??
// #define node_header_32   0x80000000 // more complex than array!
// #define kind_header_32   0xDD000000
// // 64 bit headers occur 1. if no multi value available

// 32 bit i32 as [[smart pointer]]s with first hex (4bit) as type
// smartType4bit
// as used in smartlong, smart_pointer_32 and EXTENDED TO upto 4 bytes in smart_pointer_64 !
// OBSOLETE because wasm runtimes now support int64 return from main AND multi-value!
// first bat of Primitive's < 0x100 !
// enum smartType4bit { // NOT compatible with smartType8bit below!
// 	int28 = 0x0, // or long60 ?
// 	//	overflow=0x1,
// 	plong = 0x1, // int64 pointer
// 	float28t = 0x2,
// 	//	foverflow=0x3,
// 	symbola = 0x4, // ≈ stringa &memoryChars[payload]
// 	json5 = 0x5,
// 	ref_index = 0x6, // value = index of externref(0x6f) in (table (export "externref_table") 1 externref) => js object !
// 	//	long60p = 0x6, // pointer to long60
// 	septet = 0x7, // 7 hexes à 4 bit OR 7 bytes in smart64! 7 is NUMERIC through 0x7C…0x7F float32 etc
// 	utf8char = 0x8, // UTF24 Unicode
// 	stringa = 0x9, // may start with 0x10 ... 0x1F
// 	anys = 0xA, // Node* / angle array / object pointer to header! => i64 pointer : 32bit type + 32bit pointer indirect or in linear memory!
// 	// nodep = 0xA,
// 	byter = 0xB, // unsigned char* with length ... ?
// 	codes = 0xC,  // direct wasm code! (char*) interpreted inline OR:
// 	//	code=0xC,  // angle code tree REDUNDANT wit 0xA / 0xD
// 	datas = 0xD, // angle node tree as STRING vs 0xA as array
// 	error_spo = 0xE,
// 	sint28 = 0xF,// negatives
// }

// NOT compatible with smartType4bit above!
// 56 bit i64 as data / [[smart pointer]]s with first byte (8bit) as type OR
// 24 bit i32 as data / [[smart pointer]]s with first byte (8bit) as type
// enum smartType8bit {
// 	int56 = 0x00, // int64 pointer
// 	float56 = 0x02,
// 	symbolb = 0x04, // ≈ stringb &memoryChars[payload]
// 	json5b = 0x05,
// 	ref_indexb = 0x06, // value = index of externref(0x6f) in (table (export "externref_table") 1 externref) => js object !
// 	septetb = 0x07, // 7 hexes à 4 bit OR 7 bytes in smart64! 7 is NUMERIC through 0x7C…0x7F float32 etc
// 	utf8charb = 0x08, // UTF24/ UTF56 Unicode
// 	stringb = 0x09, // may start with 0x10 ... 0x1F
// 	anyb = 0x0A, // Node* / angle array / object pointer to header! => i64 pointer : 32bit type + 32bit pointer indirect or in linear memory!
// 	byterb = 0x0B, // unsigned char* with length ... ?
// 	codesb = 0x0C,  // direct wasm code! (char*) interpreted inline OR:
// 	data_wasp_string = 0x0D, // angle node tree as STRING vs 0xA as array datasb
// 	error_spob = 0x0E,
// 	sint56 = 0x0F,// negatives
// 	double56 = 0x7F, // first hex of nearly all f64 by coincidence
// }

pub unsafe fn str56(ptr: u64) -> &'static str {
	let p = ptr as *const u8;
	let len: usize = 100; // todo strlen(ptr) as usize;
	let bytes = core::slice::from_raw_parts(p, len);
	core::str::from_utf8_unchecked(bytes)
}

pub unsafe fn str32(ptr: u32, len: u32) -> &'static str {
	let bytes = core::slice::from_raw_parts(ptr as *const u8, len as usize);
	core::str::from_utf8_unchecked(bytes)
}

pub unsafe fn string56(pointer: u64) -> Node {
	text(str56(pointer))
}

pub fn char24(data32: u32) -> Node {
	Node::Char(char::from_u32(data32).unwrap_or_default())
}

// supperfluous since we have multi-value returns now!! (type32 + data64 )
// or even full node (type32 + node64 + payload64 ) ( P S O ) ( predicate subject object )
// or even full node (type32 + value64 + node64 ) ( P O S ) ( type/predicate object subject )
// ( P O S ) ≈ (T V N) (type value node) perfect for:
// (tag 'html' [(meta 'attribute' (class "item")) /* mixed with body : */ (text "hello")  ))
// (defn 'myfunc' [(meta 'params' [a b]) (body ( ... ) ) ) // params are STRONG meta, not comments!
// (call 'myfunc' [arg1 arg2])  shorthand: my
// BUT CONFUSING otherwise! unless we put payload left!!
// last slot ø can be used for META data too!
// (list [a b c] ø) shorthand: [a b c]
// (cons a b) shorthand: (a . b) linked list  a via any but still Node!
// (text "ok" ø)  shorthand: "ok"
// (key "a" b) shorthand: "a":b
// (pair a b) shorthand: a=b  ( a can be any Node, so a symbol as variable )
// (number 32 Int)  shorthand: 32 // can we reference fixed Nodes? sure, why not wasm_init
// (number 3.14 Float) shorthand: 3.14
// (int -42 ø)  shorthand: -42   we don't need to follow Rust Node::Number here!
// (float 3.14 ø)  shorthand: 3.14  careful don't confuse with wasm f32.const, we are in Node land!
// (float_pointer 0xF123 ø)  shorthand: float@F123
// (byte 0xFF ø)  useless shorthand: 0xFF
// (char '⚡︎' ø)  shorthand: '⚡︎'
// (true 1 True)  shorthand: True
// (true 1 ø)  shorthand: True
// (bool 0 ø)  shorthand: False  == ALWAYS over method vs === like in js
// or even full node (node64 type32 payload64) ( S P O ) ( subject predictable object )
// or even full node (node64 type32 payload64) ( S P O ) ( subject predictable object )

pub fn float28(data28: u32) -> Node {
	// ⚠️ todo float28 needs 0 inserted as 6th nibble!
	// 0xf921550 => 0xf9021550  10000000000.0 works for MOST floats!!
	let left = data28 << 1 & 0xFF00000;
	let right = data28 & 0x00FFFFF;
	let f = f32::from_bits(left | right);
	float(f as f64)
}

pub fn float_data28(f: f32) -> u32 {
	// ⚠️ cut 6th nibble and add 0x2 header
	// 0x2f921550 <= 0xf9021550  10000000000.0 works for MOST floats!!
	let bits: u32 = f.to_bits();
	if (bits & 0xF0000000u32) == 0x30000000 {
		return bits;
	}
	if (bits & 0xF0000000u32) == 0x20000000 {
		return bits;
	}
	let left = (bits & 0xFF00000) >> 1;
	let right = bits & 0x00FFFFF;
	left | right
}

// often only makes sense for 32 bit pointers in wasm linear memory!
pub fn smarty32(smart: u32) -> Node {
	let header16 = (smart >> 16) as u16;
	let header8 = (smart >> 24) as u8;
	let header4 = (smart >> 28) as u8; // just one nibble u4
	let data16 = smart & 0x0000FFFF;
	let data24 = smart & 0x00FFFFFF; // small header + 24 bits data
	let data28 = smart & 0x0FFFFFFF;
	if smart == 0 { return Empty; } // null pointer or 0 we neither know nor care?
	match header4 {
		0x0 => int(data28 as i64), // positive int, just reinterpret!
		0xF => int(smart as i32 as i64), // negative int juat all bits
		0x2 => float28(data28),
		0x3 => float(f32::from_bits(smart) as f64), // which ones??
		0x1 => unsafe { string56(data24 as u64) },
		0xC => char24(data24),
		0xD => unsafe { parse(str32(data24, 3)) },
		0xE => unsafe { Error(format!("{}", str32(data24, 3))) }, // error with string message
		_ => unreachable!(),
	}
}

pub fn smarty(smart: u64) -> Node {
	if smart == 0 {
		return Empty;
	};
	let header64 = smart & 0xFFFFFFFF00000000;
	let header32 = (smart >> 32) as u32;
	let header16 = (smart >> 48) as u16;
	let header8 = (smart >> 56) as u8;
	let data32 = (smart & 0x00000000FFFFFFFF) as u32;
	let data56 = smart & 0x00FFFFFFFFFFFFFF; // small header + 56 bits data
	match header8 {
		0x00 => int(data56 as i64), // positive int, just reinterpret!
		0xFF => int(smart as i64),  // negative int juat all bits
		0x7F => float(f64::from_bits(smart)), // double just all bits!
		0x01 => unsafe { string56(data56) },
		0x10 => unsafe { string56(data56) },
		0xC0 => char24(data32),
		0xD0 => unsafe { parse(str56(data56)) }, // wasp data string!
		0xE1 => unsafe { Error(format!("{}", str56(data56))) }, // error with string message
		// _ => unreachable!(),
		_ => {
			if header16 == 0xB001 {  // BOOL wasteful header
				if data32 == 0 { False } else { True }
			} else {
				Error(format!("smart: {} unknown", smart))
			}
		}
	}
}
