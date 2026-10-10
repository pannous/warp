// Cards sequence-value and block-data-sound: a function body of several statements gives its last statement's value,
// whatever its type: `f() := {⏎1⏎ x⏎}` gave the data `{1 x}` when x was no number (`{beep()⏎ play(440)}` gave `{0 0}`,
// `{sound_samples(…)⏎ std_io(…)}` gave `{ø 1}`); an object `{a: 1⏎ b: 2}` stays data
use crate::is;

#[test]
fn a_body_of_statements_gives_its_last_value() {
	is!("f(x) := {\n1\nx\n}; f(\"a\")", "a");
	is!("f(x) := {\nx\n[2]\n}; count(f(1))", 1);
	is!("f() := {\n1\nstd_io(\"sound\", \"queued\", [])\n}; f()", 0);
	is!("f = (x) => {\n1\nx\n}; f(\"a\")", "a");
}

#[test]
fn a_body_of_fields_stays_an_object() {
	is!("f() := {\na: 1\nb: 2\n}; f().b", 2);
}
