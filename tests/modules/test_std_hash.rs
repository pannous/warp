//! The standard library module hash (notes/stdlib.md section 7): sha256 as lowercase hex and crc32 as a number, of a
//! text's UTF-8 bytes; the same values natively (sha2, crc32fast) and in the browser (host.js)
use crate::is;

#[test]
fn use_hash_gives_sha256_and_crc32() {
	is!("use hash; sha256(\"abc\")", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
	is!("use hash; sha256(\"héllo wörld\")", "a1003f7d04a4115711d0b48a2eaf1359ce565d2d2a6fd65098dfcffadeeef59f");
	is!("use hash; crc32(\"abc\")", 891568578);
	is!("use hash; crc32(\"héllo wörld\")", 354246585);
}
