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

/// The same through the registry: svg of github.com/pannous/warp-packages passes on colors' rainbow, not its to_hex,
/// also when svg's shapes.warp imported colors first; a program's own `use colors` sees all of colors
#[cfg_attr(not(feature = "native"), ignore = "browser: needs git")]
#[test]
fn a_registry_package_passes_on_its_chosen_words() {
	is!("use svg; count(rainbow(3))", 3);
	fails_with("use svg; to_hex(255)", "undefined function: to_hex");
	is!("use colors; use svg; to_hex(255)", "#0000ff");
}
