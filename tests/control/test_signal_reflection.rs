// Reflection over listeners (card g-3HmY, like a C# event's invocation list; notes/signals.md): `listeners of x` is the
// list of the functions listening to x, `clear listeners of x` unsubscribes them all. Either makes x a $Signal, so the
// main level's own listeners on x subscribe at run time too and show in the list
use crate::is;

#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
fn listeners_of_a_variable_counts_its_listeners() {
	is!("x = 0; count listeners of x", 0);
	is!("x = 0; on set x {print value}; whenever x > 3 {print \"big\"}; count listeners of x", 2);
	is!("watch(s) := on change s {print value}; x = 0; watch(x); watch(x); count listeners of x", 2);
}

#[test]
#[cfg(feature = "native")]
fn clear_listeners_unsubscribes_them_all() {
	assert_eq!(printed("x = 0\non set x {print \"seen \" + value}\nx = 1\nclear listeners of x\nx = 2\ncount listeners of x"), "seen 1\n");
	assert_eq!(printed("watch(s) := on change s {print value}\nx = 0\nwatch(x)\nx = 1\nclear listeners of x\nx = 2\nwatch(x)\nx = 3"), "1\n3\n");
}

#[test]
#[cfg(feature = "native")]
fn a_listener_taken_from_the_list_can_be_called() {
	assert_eq!(printed("x = 0\non set x {print \"set \" + value}\nfirst = (listeners of x)#1\nfirst(5, 0)"), "set 5\n");
	assert_eq!(printed("x = 0\non set x {print \"set \" + value}\nfor f in listeners of x { f(6, 0) }"), "set 6\n");
}
