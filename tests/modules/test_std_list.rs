//! The standard library module list (notes/stdlib.md): `use list` loads lib/list.wasp, embedded in warp
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn use_list_brings_its_words() {
	is!("use list; unique([1, 2, 1, 3, 2])", parse("[1 2 3]"));
	is!("use list; zip([1, 2, 3], [4, 5])", parse("[[1 4] [2 5]]"));
	is!("use list; enumerate([5, 6])", parse("[[1 5] [2 6]]"));
	is!("use list; product([2, 3, 4])", 24);
	is!("use list; mean([1, 2, 3, 4])", 2.5);
	is!("use list; take([1, 2, 3], 2)", parse("[1 2]"));
	is!("use list; drop([1, 2, 3], 1)", parse("[2 3]"));
	is!("use list; flatten([[1, 2], [3]])", parse("[1 2 3]"));
}

#[test]
fn a_programs_own_word_wins_over_the_module() {
	is!("use list; product(a, b) := a * b; product(3, 4)", 12);
}

#[test]
fn a_module_word_without_its_use_names_the_module() {
	crate::common::fails_with("zip([1], [2])", "zip is in the standard module list: write `use list`");
	crate::common::fails_with("unique [1, 1]", "unique is in the standard module list: write `use list`");
}

#[test]
fn chunk_window_median() {
	is!("use list; chunk([1, 2, 3, 4, 5], 2)", parse("[[1 2] [3 4] [5]]"));
	is!("use list; window([1, 2, 3, 4], 3)", parse("[[1 2 3] [2 3 4]]"));
	is!("use list; median([3, 1, 2])", 2);
	is!("use list; median([4, 1, 3, 2])", 2.5);
}

#[test]
fn use_list_groups_partitions_and_picks_by() {
	is!("use list; count(flat_map([1, 2], x => [x, x * 10]))", 4);
	is!("use list; p = partition([1, 2, 3, 4], x => x > 2); count(p#1) * 10 + count(p#2)", 22);
	is!("use list; max_by([\"a\", \"ccc\", \"bb\"], w => count(w))", "ccc");
	is!("use list; min_by([3, -5, 2], x => x * x)", 2);
	is!("use list; g = group_by([\"ab\", \"c\", \"de\"], w => count(w)); count(g.get(\"2\"))", 2);
	is!("use list; t = tally([\"a\", \"b\", \"a\"]); t.get(\"a\")", 2);
}

#[test]
fn use_list_zips_with_sums_by_rotates_interleaves_dedupes() {
	is!("use list; zip_with([1, 2, 3], [10, 20], (a, b) => a + b)", parse("[11 22]"));
	is!("use list; sum_by([\"a\", \"bcd\"], w => count(w))", 4);
	is!("use list; count_by([1, 2, 3, 4], x => x > 1)", 3);
	is!("use list; rotate([1, 2, 3, 4], 1)", parse("[2 3 4 1]"));
	is!("use list; interleave([1, 3, 5], [2, 4])", parse("[1 2 3 4 5]"));
	is!("use list; dedupe([1, 1, 2, 1, 1])", parse("[1 2 1]"));
}

#[test]
fn use_list_scans_takes_and_drops_while_finds_argmax() {
	is!("use list; scan([1, 2, 3], 0, (a, b) => a + b)", parse("[1 3 6]"));
	is!("use list; take_while([1, 2, 5, 1], x => x < 3)", parse("[1 2]"));
	is!("use list; drop_while([1, 2, 5, 1], x => x < 3)", parse("[5 1]"));
	is!("use list; argmax([1, 5, 2])", 1);
	is!("use list; argmin([3, 1, 2])", 1);
}

#[test]
fn use_list_pairs_splits_crosses_and_measures_spread() {
	is!("use list; pairwise([1, 2, 3])", parse("[[1 2] [2 3]]"));
	is!("use list; split_at([1, 2, 3, 4], 1)", parse("[[1] [2 3 4]]"));
	is!("use list; cartesian([1, 2], [3, 4])", parse("[[1 3] [1 4] [2 3] [2 4]]"));
	is!("use list; mode([1, 2, 2, 3])", 2);
	is!("use list; variance([1, 2, 3, 4])", 1.25);
	is!("use list; stdev([2, 4, 4, 4, 5, 5, 7, 9])", 2);
}

#[test]
fn use_list_finds_positions_fills_and_bounds() {
	is!("use list; [index_where([1, 5, 2], x => x > 3), index_where([1], x => x > 3)]", warp::ints(vec![2, 0]));
	is!("use list; last_n([1, 2, 3], 2)", parse("[2 3]"));
	is!("use list; last_n([1], 3)", parse("[1]"));
	is!("use list; fill(3, 0)", parse("[0 0 0]"));
	is!("use list; minmax([3, 1, 2])", parse("[1 3]"));
}
