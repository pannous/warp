//! add, append, push and a one-argument insert are one table (analyzer appends, card cleanup-closed-lists): every
//! pass that follows appends knows every spelling
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_every_append_word_appends_a_quantity() {
	for word in ["add", "append", "push", "insert"] {
		assert_eq!(eval(&format!("xs = [1 m]; xs.{word}(2 m); sum(xs)")).serialize().trim(), "3m", "{word}");
		fails_with(&format!("xs = [1 m]; xs.{word}(2 s); xs"), "DimensionError");
	}
}

#[test]
fn test_every_append_word_appends_a_closure() {
	for word in ["add", "append", "push", "insert"] {
		is!(&format!("fs = []; for n in [1 2] {{ fs.{word}(x => x + n) }}; fs#2(10)"), 12);
	}
}
