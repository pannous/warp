// Signals kept in a list (notes/signals.md phase 5): a function that subscribes to each item of a list it is given
// makes the variables in that list signals; the list holds the signals themselves, so later writes reach the listener
#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(feature = "native")]
fn a_function_subscribes_to_each_signal_of_a_list() {
	assert_eq!(printed("def watch_all(xs) { for s in xs { on change s {print \"now \" + value} } }\na = 1\nb = 2\nwatch_all([a, b])\na = 3\nb = 4\nb = 4"), "now 3\nnow 4\n");
}

#[test]
#[cfg(feature = "native")]
fn a_list_of_signals_kept_in_a_variable() {
	assert_eq!(printed("watch_all(xs) := for s in xs { on set s {print value} }\na = 1\nb = 2\nwatched = [a, b]\nwatch_all(watched)\nb = 5\na = 6\na + b"), "5\n6\n");
}

#[test]
fn the_variables_of_the_list_keep_their_values() {
	crate::is!("watch_all(xs) := for s in xs { on set s {print value} }\na = 1\nb = 2\nwatch_all([a, b])\na = a + 10\na * b", 22);
}
