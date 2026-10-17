// card comparator-sort: a comparator of two items sorts in n log n, as a key of one item does (a merge sort, stable)
use warp::wasm_emitter::eval;

const ITEMS: usize = 30_000;

#[test]
fn a_comparator_sorts_many_items_quickly() {
	let started = std::time::Instant::now();
	let code = format!(
		"xs = []; for i in 1 to {ITEMS} {{ xs = xs + [(i * 7919) % {ITEMS}] }}; ys = xs.sort_by((a b) => a > b); [ys#1, ys#{ITEMS}, xs#1]"
	);
	assert_eq!(eval(&code).serialize(), format!("[{} 0 7919]", ITEMS - 1));
	// an insertion sort took 5 s here
	assert!(started.elapsed().as_secs() < 3, "{:?}", started.elapsed());
}

#[test]
fn a_comparator_sort_is_stable() {
	assert_eq!(
		eval("ps = [[2, \"a\"], [1, \"b\"], [2, \"c\"], [1, \"d\"], [3, \"e\"]]; ps.sort_by((p q) => p#1 < q#1).map(p => p#2)").serialize(),
		"['b' 'd' 'a' 'c' 'e']"
	);
	assert_eq!(eval("xs = [5, 3, 9, 1, 7, 2]; sorted(xs, (a,b) => a-b)").serialize(), "[1 2 3 5 7 9]");
	assert_eq!(eval("xs = []; sorted(xs, (a,b) => a-b)"), eval("[]"));
	assert_eq!(eval("sorted([4], (a,b) => a-b)").serialize(), "[4]");
}
