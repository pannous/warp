// Removing one listener (P128) from inside a function or a block, not only as a main-level statement
use crate::is;

#[test]
fn a_listener_is_removed_inside_a_function() {
	is!("t = 0\na = on set t {print 1}\nb = on set t {print 2}\nstop() := { remove b from listeners of t }\nstop()\ncount listeners of t", 1);
	is!("t = 0\na = on set t {print 1}\nb = on set t {print 2}\nstop() := { remove a from listeners of t }\nstop()\nstop()\nremove b from listeners of t\ncount listeners of t", 0);
}

#[test]
fn a_listener_is_removed_inside_a_block() {
	is!("t = 0\na = on set t {print 1}\nb = on set t {print 2}\nif t < 1 { remove b from listeners of t }\ncount listeners of t", 1);
}

#[test]
#[cfg(feature = "native")]
fn a_listener_removed_by_a_function_stops_listening() {
	let output = crate::common::printed("t = 0\nalarm = whenever t > 30 {print \"hot \" + t}\ncalm() := { remove alarm from listeners of t }\nt = 35\ncalm()\nt = 40");
	let printed: String = output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect();
	assert_eq!(printed, "hot 35\n");
}
