// card double-for-comprehension: a list comprehension with several `for` clauses nests them in order, as Python does;
// a filter `if c` sits between or after them. It was silently empty
use crate::is;
use warp::warp_parser::parse;

#[test]
fn two_for_clauses_nest() {
	is!("[f * i for f in [1, 2] for i in 0..3]", parse("[0 1 2 0 2 4]"));
	is!("count([f * i for f in [1, 2] for i in 0..3])", 6);
	is!("[x + y for x in [10, 20] for y in [1, 2] for z in [0, 0]]", parse("[11 11 12 12 21 21 22 22]"));
}

#[test]
fn filters_sit_between_and_after_the_clauses() {
	is!("[f * i for f in [1, 2] for i in 0..3 if i > 0]", parse("[1 2 2 4]"));
	is!("[f * i for f in [1, 2] if f > 1 for i in 0..3]", parse("[0 2 4]"));
	is!("[f + i for f in [1, 2, 3] if f != 2 for i in [10, 20] if i < 20]", parse("[11 13]"));
}

#[test] // a condition after `where` may start with `#`, `-` or a variable named like an operator word (`to`) (card topo-sort-wasm)
fn a_where_condition_starts_with_a_count() {
	is!("idx=[1 2]; done=[5]; #[i for i in idx where #[d for d in done where d == i] == 0]", 2);
	is!("xs=[1 2 3]; [x for x in xs where -x < -1]", parse("[2 3]"));
	is!("to = [2 0]; idx=[1 2]; #[i for i in idx where to#i == 0]", 1);
	is!("from = [5 5 4 4 2 3]\nto = [2 0 0 1 3 1]\nidx = [1 2 3 4 5 6]\ndone = []\nwhile #done < 6 {\n\tpick = -1\n\tfor n in 0 to 5 {\n\t\tisdone = #[d for d in done where d == n] > 0\n\t\tblockers = #[i for i in idx where to#i == n and #[d for d in done where d == from#i] == 0]\n\t\tif pick < 0 and not isdone and blockers == 0 : pick = n\n\t}\n\tdone += [pick]\n}\ndone", parse("[4 5 0 2 3 1]"));
}
