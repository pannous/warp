// `dir(time)` (Python's dir): the names a standard module defines, as texts, in the order it defines them
// (card dir-introspection)
use crate::is;

#[test]
fn dir_lists_the_names_of_a_standard_module() {
	is!("(\"weekday\" in dir(time)) > 0", true);
	is!("dir(time)#1", "day_number");
	is!("names = dir text; names#1", "repeat");
	is!("use time; #dir(time) > 5", true);
}

#[test]
fn a_program_s_own_dir_wins() {
	is!("dir(x) := x + 1; dir(3)", 4);
	is!("time = 5; dir(x) := x * 2; dir(time)", 10);
}
