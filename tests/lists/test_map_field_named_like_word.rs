use crate::is;

// card map-field: a map's field named like a builtin list word (first, last, count …) reads the field when the
// receiver may hold a map (a parameter, a loop's element); a list there still gets the word

#[test]
fn a_parameter_s_field_named_like_a_list_word_is_the_field() {
	is!("f(m) := m.first; f({first: 9})", 9);
	is!("f(m) := m.last; f({last: \"z\"})", "z");
	is!("f(m) := m.first + 1; f({first: 9})", 10);
}

#[test]
fn a_parameter_holding_a_list_still_gets_the_word() {
	is!("f(m) := m.first; f([4 5])", 4);
	is!("f(m) := m.first + 1; f([4 5])", 5);
}

#[test]
fn a_loop_element_s_field_named_like_a_list_word_is_the_field() {
	is!("r = 0; for p in [{first: 2}] { r = p.first }; r", 2);
	is!("r = 0; for p in [{first: 2}, {first: 3}] { r += p.first }; r", 5);
}
