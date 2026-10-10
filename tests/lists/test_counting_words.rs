//! count, size, len and length are one table (analyzer COUNTING_WORDS, card cleanup-closed-lists): every pass that
//! counts knows every spelling; `len` of a list of quantities was "undefined variable: m"
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_every_counting_word_counts_a_list_of_quantities() {
	for word in ["count", "size", "len", "length"] {
		assert_eq!(shown(&format!("xs = [1 m, 2 m]; {word}(xs) + 1")), "3", "{word}");
	}
}

#[test]
fn test_every_other_counting_word_finds_the_size_method_of_a_class() {
	for word in ["count", "len", "length"] {
		assert_eq!(shown(&format!("class Bag {{ size() := 4 }}\nb = Bag()\n{word}(b)")), "4", "{word}");
	}
}
