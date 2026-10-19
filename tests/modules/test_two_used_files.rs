//! Two used files declaring one name (card two-used-files): the later silently replaced the earlier, now it is the
//! error naming both files; overloads by parameter type stay apart
use crate::common::fails_with;
use crate::is;

#[test]
fn two_files_declaring_one_function_name_both() {
	fails_with(
		"use \"tests/modules/twice_doubled.warp\"; use \"tests/modules/twice_tripled.warp\"; twice(5)",
		"twice(any) is declared in both tests/modules/twice_doubled.warp and tests/modules/twice_tripled.warp",
	);
}

#[test]
fn overloads_from_two_files_stay_apart() {
	is!("use \"tests/modules/twice_doubled.warp\"; use \"tests/modules/twice_of_text.warp\"; twice(\"ab\")", "abab");
	is!("use \"tests/modules/twice_doubled.warp\"; use \"tests/modules/twice_of_text.warp\"; twice(5)", 10);
}
