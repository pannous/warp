//! A package's main file passes on the words it names in `from … import` lines, also of its own local files, and
//! nothing else of them: the helpers of the files and what they import stay inside (card import-hides; decision
//! package-reexport). Fixture: tests/warp/selective_package, pkg.warp over shapes.warp, tags.warp and palette.warp
use crate::common::fails_with;
use crate::is;

#[test]
fn the_main_file_passes_on_the_named_words() {
	is!("tests/warp/selective_package/main.warp", "<box 5> 6 4");
}

#[test]
fn a_word_one_package_file_imports_from_another_stays_inside() {
	fails_with("tests/warp/selective_package/main_hidden_tagged.warp", "undefined function: tagged");
	fails_with("tests/warp/selective_package/main_hidden_shade.warp", "undefined function: shade");
}
