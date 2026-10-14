//! Card dump-text: `(p).a` is `p.a`, parentheses around a variable before a dot only group it
use crate::common::fails_with;
use crate::is;

#[test]
fn a_grouped_receiver_reads_its_field() {
	is!("p = {a: 1}\n(p).a", 1);
	is!("t = \"abc\"\n(t).count", 3);
}

#[test]
fn a_grouped_text_without_the_field_names_it() {
	fails_with("for t in [\"a\" \"b\"]: print (t).dump", "no field dump");
}

#[test]
fn a_call_without_arguments_stays_a_call() {
	is!("f() := {a:1 b:2}; f().b", 2);
	is!("use time\ntoday().year > 2025", true);
	is!("mk = () => ({a: 1}); mk().a", 1);
	is!("class point{x:int=0; y:int=3}; point().y", 3);
}
