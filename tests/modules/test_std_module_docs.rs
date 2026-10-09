// card std-module-docs (notes/stdlib.md §8 "Discoverability"): `warp help list` prints a module's words, each with the
// comment line above it; the same index makes wiki/standard-library.md (`warp help --markdown`)
use warp::std_docs::{module_help, standard_library_markdown};

#[test]
fn help_lists_a_modules_words_with_their_comment() {
	let list = module_help("list").expect("list is a standard module");
	assert!(list.contains("use list") && list.contains("unique(xs)") && list.contains("zip(a, b)"), "{list}");
	let math = module_help("math").expect("math is a standard module");
	assert!(math.contains("lerp(a, b, t)") && math.contains("the value a fraction t of the way from a to b"), "{math}");
	let collections = module_help("collections").expect("collections is a standard module");
	assert!(collections.contains("class Stack"), "{collections}");
	assert_eq!(module_help("no_such_module"), None);
}

#[test]
fn the_markdown_page_has_every_standard_module() {
	let page = standard_library_markdown();
	for module in warp::modules::std_module_names() {
		assert!(page.contains(&format!("## {module}")), "{module} missing");
	}
	assert!(page.contains("`unique(xs)`"), "{page}");
}
