// Sorting with a function, as other languages write it: a comparator of two items (JS/Ruby: negative puts the first
// first; Swift: a comparison holding puts the first first) or a key of one item (Python key=, Kotlin sortedBy,
// Ruby sort_by). The sort is stable and leaves the list itself unchanged.
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn sorted_without_function() {
	assert_eq!(printed("xs=[3,1,2]; sorted(xs)"), "[1 2 3]");
	assert_eq!(printed("xs=[3,1,2]; sorted(xs); xs"), "[3 1 2]");
}

#[test]
fn sort_with_comparator() {
	assert_eq!(printed("xs=[3,1,2]; sorted(xs, (a,b) => b-a)"), "[3 2 1]"); // JS / Python cmp
	assert_eq!(printed("xs=[3,1,2]; sort(xs, (a,b) => a-b)"), "[1 2 3]");
	assert_eq!(printed("xs=[3,1,2]; xs.sort((a,b) => b-a)"), "[3 2 1]"); // JS method
	assert_eq!(printed("xs=[3,1,2]; xs.sorted((a,b) => a > b)"), "[3 2 1]"); // Swift sorted(by: >)
	assert_eq!(printed("xs=[3,1,2]; sorted(xs, by: >)"), "[3 2 1]");
}

#[test]
fn sort_with_key() {
	assert_eq!(printed("xs=[\"bb\",\"ccc\",\"a\"]; sorted(xs, key: s => s.length)"), "['a' \"bb\" \"ccc\"]"); // Python key=
	assert_eq!(printed("xs=[3,-1,2]; xs.sortedBy { it*it }"), "[-1 2 3]"); // Kotlin sortedBy
	assert_eq!(printed("xs=[3,-1,2]; xs.sort_by(x => -x)"), "[3 2 -1]"); // Ruby sort_by
}

// texts compare alphabetically under a comparison comparator too (card words-sorted: ["kiwi" "fig" "banana"] looked
// unsorted by > but is already descending)
#[test]
fn sort_texts_with_comparator() {
	assert_eq!(printed("words = [\"fig\" \"kiwi\" \"apple\" \"banana\"]; words.sorted(by: >)"), "[\"kiwi\" \"fig\" \"banana\" \"apple\"]");
	assert_eq!(printed("words = [\"fig\" \"kiwi\" \"apple\" \"banana\"]; sorted(words, by: <)"), "[\"apple\" \"banana\" \"fig\" \"kiwi\"]");
}

#[test]
fn sort_is_stable() {
	assert_eq!(printed("xs=[21,12,11,22]; sorted(xs, x => x % 10)"), "[21 11 12 22]");
}

#[test]
fn sort_with_defined_function() {
	assert_eq!(printed("def desc(a,b){ b-a }; sorted([1,3,2], desc)"), "[3 2 1]");
	assert_eq!(printed("def size(s){ s.length }; sorted([\"bb\",\"ccc\",\"a\"], size)"), "['a' \"bb\" \"ccc\"]");
}

#[test]
fn reduce_with_a_start_is_fold() {
	assert_eq!(printed("xs = [1,2,3]; xs.reduce(0, +)"), "6"); // Swift
	assert_eq!(printed("xs = [1,2,3]; xs.reduce(10, (a, b) => a + b)"), "16");
	assert_eq!(printed("reduce([1,2,3], 1, (a, b) => a * b)"), "6");
}
