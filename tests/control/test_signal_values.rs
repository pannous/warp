// Signals as values (notes/signals.md phase 5): a variable a function subscribes to escapes into a $Signal cell; the
// listener made inside the function stays subscribed after it returns, so every later write of the variable runs it
use crate::is;

/// What the program prints, without the CLI's last line `» value`
#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(feature = "native")]
fn a_function_subscribes_to_the_signal_it_is_given() {
	assert_eq!(printed("watch(s) := on change s {print \"now \" + value}\nx = 1\nwatch(x)\nx = 2"), "now 2\n");
	assert_eq!(printed("def alarm(reading) { whenever reading > 30 { print \"hot \" + reading } }\nt = 20\nalarm(t)\nt = 25\nt = 35"), "hot 35\n");
}

#[test]
#[cfg(feature = "native")]
fn on_change_skips_writes_of_the_same_value() {
	assert_eq!(printed("watch(s) := on change s {print value}\nx = 1\nwatch(x)\nx = 1\nx = 2\nx = 2\nx = 3"), "2\n3\n");
}

#[test]
#[cfg(feature = "native")]
fn on_set_runs_after_each_write() {
	assert_eq!(printed("def log(s) { on set s { print \"set \" + value } }\nx = 1\nlog(x)\nx = 1\nx = 4"), "set 1\nset 4\n");
}

#[test]
#[cfg(feature = "native")]
fn writes_before_the_subscription_are_not_seen() {
	assert_eq!(printed("watch(s) := on set s {print value}\nx = 1\nx = 2\nwatch(x)\nx = 3"), "3\n");
}

#[test]
#[cfg(feature = "native")]
fn two_subscriptions_both_run_in_order() {
	assert_eq!(printed("def a(s) { on set s { print \"a\" } }\ndef b(s) { on set s { print \"b\" } }\nx = 0\na(x)\nb(x)\nx = 1"), "a\nb\n");
}

#[test]
#[cfg(feature = "native")]
fn a_function_subscribes_to_a_main_level_variable() {
	assert_eq!(printed("count = 0\ndef log_changes() { on change count { print \"count \" + value } }\ncount = 1\nlog_changes()\ncount = 2"), "count 2\n");
}

#[test]
fn the_escaping_variable_keeps_its_value() {
	is!("watch(s) := on set s {print value}\nx = 1\nwatch(x)\nx = x + 5\nx * 2", 12);
	is!("watch(s) := on set s {print value}\nx = 1\nwatch(x)\nx += 5\nx", 6);
}

// a subscription runs inside the write, the main level's own listeners right after it
#[test]
#[cfg(feature = "native")]
fn a_static_listener_and_a_subscription_on_one_variable() {
	assert_eq!(printed("watch(s) := on set s {print \"inside\"}\nx = 0\non set x {print \"main\"}\nwatch(x)\nx = 1"), "inside\nmain\n");
}

#[test]
#[cfg(feature = "native")]
fn once_inside_a_function_runs_the_first_time() {
	assert_eq!(printed("def first(s) { once s > 2 { print \"over 2: \" + s } }\nx = 0\nfirst(x)\nx = 3\nx = 5"), "over 2: 3\n");
}

#[test]
#[cfg(feature = "native")]
fn a_signal_passed_on_to_a_subscribing_function() {
	assert_eq!(printed("watch(s) := on change s {print \"now \" + value}\ndef relay(t) { watch(t) }\nx = 1\nrelay(x)\nx = 7"), "now 7\n");
}

#[test]
#[cfg(feature = "native")]
fn a_signal_of_text() {
	assert_eq!(printed("watch(s) := on change s {print \"name \" + value}\nn = \"a\"\nwatch(n)\nn = \"b\""), "name b\n");
}

#[test]
#[cfg(feature = "native")]
fn writes_in_loops_and_functions_notify() {
	assert_eq!(printed("watch(s) := on set s {print value}\nx = 0\nwatch(x)\nfor i in 1 to 3 { x = i }"), "1\n2\n3\n");
	assert_eq!(printed("watch(s) := on set s {print value}\nx = 1\nwatch(x)\ndef bump() { global x; x = x + 1 }\nbump()\nbump()"), "2\n3\n");
}

#[test]
fn a_value_given_for_a_signal_parameter_is_a_signal_nobody_writes() {
	is!("watch(s) := on set s {print value}\nwatch(3)\n1", 1);
}
