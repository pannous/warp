//! A compound update of a field named like a library word (size, bytes) writes the field, as `p.size = …` does
//! (card word-field-update; emitter.warp's `module.bytes += t.bytes` wrote no text data in compiler.warp)
use crate::is;

#[test]
fn a_field_named_like_a_word_takes_its_update() {
	is!("f(p) := { p.size += 1; p.size }; f({size: 3})", 4);
	is!("f(p) := { p.bytes += [1]; p.bytes }; count(f({bytes: [0]}))", 2);
	is!("f(p) := { p.size++; p.size }; f({size: 3})", 4);
	is!("class M { bytes: any = [] }\nm = M()\nm = \"text\"\nm = M()\nm.bytes += [5]\ncount(m.bytes)", 1);
}
