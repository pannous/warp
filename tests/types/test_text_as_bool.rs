//! `t as bool` of a text known only at run time: no for a falsy word ("no", "false", "0", "", …) as for a literal,
//! yes otherwise; it failed "not an int" (card yes-print)
use crate::is;

#[test]
fn a_run_time_text_converts_to_bool() {
	is!("t = \"yes\"\nprint t\nt as bool", true);
	is!("t = \"No\"\nprint t\nt as bool", false);
	is!("t = \"abc\"\nprint t\nt as bool", true);
	is!("m = {debug: \"false\"}\nm.debug as bool", false);
}
