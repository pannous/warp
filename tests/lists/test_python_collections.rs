// card python-stdlib: Python's list, dict and tuple methods and builtins as aliases of warp's own words. The changing
// ones that only change (`xs.extend(ys)`, `m.update(n)`) change the list or map in place, like xs.append(v); Python's
// in-place `xs.sort()` stays warp's copy with its warning (wiki/mutable.md: `xs.sort!` changes xs)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn list_methods() {
	is!("xs = [1 2]; xs.extend([3 4]); count(xs)", 4);
	is!("xs = [1 2]; ys = xs; xs.extend([3]); count(ys)", 3);
	is!("xs = [5 6 7]; xs.index(6)", 1);
	is!("reversed([1 2 3])#1", 3);
	is!("count(tuple([1 2]))", 2);
	is!("len([1 2 3])", 3);
}

#[test]
fn a_missing_index_is_an_error() {
	let missing = format!("{:?}", eval("xs = [5 6 7]; xs.index(9)"));
	assert!(missing.contains("9 is not in the list"), "{missing}");
}

#[test]
fn dict_methods() {
	is!("m = {a:1 b:2}; count(m.items())", 2);
	is!("m = {a:1}; m.update({b:2}); m.b", 2);
	is!("m = {a:1}; m.setdefault(\"b\", 5) + m.setdefault(\"a\", 7)", 6);
	is!("d = dict([[\"a\" 1] [\"b\" 2]]); d.b", 2);
}

// card python-aliases
#[test]
fn popitem_and_fromkeys() {
	is!("m = {a:1 b:2}; e = m.popitem(); e#2 * 10 + count(m)", 21);
	is!("count(fromkeys([\"a\" \"b\"], 0))", 2);
	is!("dict.fromkeys([\"a\" \"b\"], 7).b", 7);
}

// lowered to lib/prelude.warp's popped_item and popped_rest (by popped_position)
#[test]
fn pop_of_a_position_or_a_key() {
	is!("xs = [5 6 7]; first = xs.pop(0); first * 100 + count(xs) * 10 + xs#1", 526);
	is!("xs = [5 6 7]; xs.pop(-1) * 10 + count(xs)", 72);
	is!("m = {a:1 b:2}; m.pop(\"b\") * 10 + count(m)", 21);
}

// lowered to lib/prelude.warp's cleared
#[test]
fn clear_empties_a_list_or_a_map() {
	is!("xs = [1 2]; xs.clear(); xs.add(3); count(xs)", 1);
	is!("m = {a:1}; m.clear(); m.b = 2; count(m)", 1);
}
