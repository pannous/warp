//! card min-max-lib: lib/extrema.warp writes min and max in warp, overloaded by arity (two or more values, variadic)
//! and by type (one list of candidates); `use extrema` takes them instead of the builtin lowering
use crate::common::fails_with;
use crate::is;

#[test]
fn extrema_of_two_or_more_values() {
	is!("use extrema; min(3, 7)", 3);
	is!("use extrema; max(3, 7)", 7);
	is!("use extrema; max(3, 7, 5) * 10 + min(4, 2, 9, 6)", 72);
	is!("use extrema; min(2.5, -1.5)", -1.5);
}

#[test]
fn extrema_of_one_list_of_candidates() {
	is!("use extrema; max([3, 7, 5])", 7);
	is!("use extrema; xs = [4, 2, 9]; min(xs) * 10 + max(xs)", 29);
}

#[test]
fn an_empty_list_has_no_extremum() {
	fails_with("use extrema; xs = []; min(xs)", "min of an empty list");
	fails_with("use extrema; xs = []; max(xs)", "max of an empty list");
}

#[test]
fn the_extremum_enum_names_its_word() {
	is!("use extrema; extremum_word(Extremum.Greatest)", "max");
	is!("use extrema; beats(1, 2, Extremum.Least)", true);
}
