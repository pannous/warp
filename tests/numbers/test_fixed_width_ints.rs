//! D16 (user, 2026-10-03): Int stays unbounded; `byte` (unsigned 8) and `int8 … int64`, `uint8 … uint64` are declared
//! types for FFI that trap on overflow on assignment and arithmetic. There is no `overflow` value.

use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_plain_int_stays_unbounded() {
	is!("x=2^100; x", eval("1267650600228229401496703205376"));
	is!("int i = 9223372036854775807; i+1", eval("9223372036854775808"));
}

#[test]
fn test_fixed_width_values_in_range() {
	is!("byte b = 255; b", 255);
	is!("byte b = 200; b-100", 100);
	is!("int8 x = -128; x", -128);
	is!("int8 x = 100; x+27", 127);
	is!("int16 x = 32767; x", 32767);
	is!("x:int32 = 7; x*3", 21);
	is!("uint8 u = 255; u", 255);
	is!("int64 x = 9223372036854775807; x", 9223372036854775807i64);
}

#[test]
fn test_fixed_width_declaration_traps_on_overflow() {
	fails_with("byte b = 256; b", "byte overflow");
	fails_with("byte b = -1; b", "byte overflow");
	fails_with("int8 x = 128; x", "int8 overflow");
	fails_with("uint16 x = 65536; x", "uint16 overflow");
}

#[test]
fn test_fixed_width_assignment_traps_on_overflow() {
	fails_with("byte b = 1; b = 300; b", "byte overflow");
	fails_with("int8 x = 1; n = 200; x = n; x", "int8 overflow");
}

#[test]
fn test_fixed_width_arithmetic_traps_on_overflow() {
	fails_with("byte b = 200; b+100", "byte overflow");
	fails_with("int8 x = 127; x+1", "int8 overflow");
	fails_with("int8 x = -128; x-1", "int8 overflow");
	fails_with("int32 x = 2147483647; x*2", "int32 overflow");
	fails_with("int64 x = 9223372036854775807; x+1", "int64 overflow");
	fails_with("byte b = 250; b += 10; b", "byte overflow");
	fails_with("byte b = 255; b++; b", "byte overflow");
}

#[test]
fn test_a_parameter_may_be_named_like_a_type() {
	is!("offset(text:text, byte, start) := byte+start; offset(\"a\", 2, 3)", 5);
	is!("f(int, start) := int+start; f(2, 3)", 5);
	is!("f(int x) := x+1; f(2)", 3);
	is!("byte=3; byte+1", 4);
}
