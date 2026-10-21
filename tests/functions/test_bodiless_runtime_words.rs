//! card bodiless-imports (decision bodiless-imports): `is_null(x: any): bool from runtime` declares an intrinsic of the
//! self-hosted emitter without a body, so a file of lib/compiler/runtime/ compiles alone; the Rust lane has no such
//! intrinsic and traps if one is called
use crate::common::fails_with;
use crate::is;

#[test]
fn a_declared_runtime_word_compiles_where_it_is_not_called() {
	is!("is_null(x: any): bool from runtime\nempty(n: any): bool := is_null(n)\n7", 7);
	is!("byte_count(t: text): int from runtime\nsize_of(t: text): int := byte_count(t)\n8", 8);
}

#[test]
fn a_declared_runtime_word_traps_when_the_rust_lane_calls_it() {
	fails_with("is_null(x: any): bool from runtime\nis_null(3)", "unreachable");
}
