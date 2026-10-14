// Removing one listener (P128, notes/signals.md): a listener named where it is declared, `alarm = whenever t > 30 {…}`,
// is a function of (new value, old value) and leaves with `remove alarm from listeners of t`
use crate::is;

#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
#[cfg(feature = "native")]
fn a_named_listener_is_removed() {
	assert_eq!(printed("t = 0\nalarm = whenever t > 30 {print \"hot \" + t}\nreport = on change t {print \"now \" + value}\nt = 35\nremove alarm from listeners of t\nt = 40"), "hot 35\nnow 35\nnow 40\n");
}

#[test]
fn removing_one_listener_keeps_the_others() {
	is!("t = 0\na = on set t {print 1}\nb = on set t {print 2}\nc = on set t {print 3}\nremove b from listeners of t\ncount listeners of t", 2);
	is!("t = 0\na = on set t {print 1}\nb = on set t {print 2}\nremove a from listeners of t\nremove b from listeners of t\nremove a from listeners of t\ncount listeners of t", 0);
}

#[test]
#[cfg(feature = "native")]
fn a_named_listener_can_be_called() {
	assert_eq!(printed("t = 0\nreport = on set t {print \"set \" + value}\nreport(7, 0)"), "set 7\n");
}
