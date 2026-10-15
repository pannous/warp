//! A program function that replaces a used module's function of the same arguments says so (card use-sound-def):
//! `use sound; def play(x) := …` makes every play(C4) of the program call it, which must not happen silently
use crate::common::{warnings_of, warns_with};

const REPLACES: &str = "replaces sound's play(tone)";

fn replace_warnings(code: &str) -> Vec<String> {
	warnings_of(code).1.into_iter().filter(|warning| warning.contains("replaces")).collect()
}

#[test]
fn a_program_function_replacing_a_module_function_warns() {
	warns_with("use sound\nplay(x) := 7\nplay(3)", 7, REPLACES);
	warns_with("use sound; def play(Sample) := 7; play(3)", 7, REPLACES);
	warns_with("use math\nclamp(x, low, high) := 9\nclamp(5, 0, 3)", 9, "replaces math's clamp(x, low, high)");
	// a helper too: the program cannot reach the module's own any more, though the module still calls it
	warns_with("use sound\nadded(x, y) := 7\nadded(1, 2)", 7, "replaces sound's added");
}

#[test]
fn a_function_adding_to_a_module_word_or_a_variable_does_not_warn() {
	// another number of arguments adds to the module word
	assert_eq!(replace_warnings("use math\nclamp(x) := 9\nclamp(1) + clamp(5, 0, 3)"), Vec::<String>::new());
	assert_eq!(replace_warnings("use math\nangle = 0.5\nangle * 2"), Vec::<String>::new());
	// list's take comes only with text's own `use list`: the program never saw it
	assert_eq!(replace_warnings("use text\ntake(xs, n) := 5\ntake(1, 2)"), Vec::<String>::new());
	// a module the program needs without naming it (beep → sound) brought none of its words to mind
	assert_eq!(replace_warnings("added(x, y) := 7\nbeep\nadded(1, 2)"), Vec::<String>::new());
}
