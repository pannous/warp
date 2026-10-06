// A one-entry map {a:1} is its entry a:1 at run time; its text keeps the braces: print {a:{b:1}} wrote a:b:1
#[cfg(feature = "native")] // printed runs the warp binary
use crate::common::printed;
use crate::is;

#[cfg(feature = "native")]
#[test]
fn print_keeps_the_braces_of_a_one_entry_map() {
	assert_eq!(printed("print {a:{b:1}}"), "{a:{b:1}}\n");
	assert_eq!(printed("m = {a:{b:1}}; print m"), "{a:{b:1}}\n");
	assert_eq!(printed("print {a:1}"), "{a:1}\n");
	assert_eq!(printed("print {a:1 b:2}"), "{a:1 b:2}\n");
	assert_eq!(printed("print [a:1]"), "[a:1]\n");
}

#[test]
fn the_text_of_a_one_entry_map_keeps_its_braces() {
	is!("m = {a:{b:1}}; string(m)", "{a:{b:1}}");
	is!("m = {a:{b:1}, c:2}; \"\\(m)\"", "{a:{b:1} c:2}");
	is!("m = {a:1}; \"\\(m)\"", "{a:1}");
}
