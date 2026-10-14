use crate::is;

// card number-methods: x.word is word(x), with or without parentheses (user: "x.word is x.word(), no special
// logic"); x.sqrt() and x.abs() were "undefined function", x.round without parentheses too

#[test]
fn test_rounding_methods_without_parentheses() {
	is!("x=2.6; x.round", 3);
	is!("x=2.6; x.floor", 2);
	is!("x=2.2; x.ceil", 3);
}

#[test]
fn test_prefix_operator_words_as_methods() {
	is!("x=4.0; x.sqrt()", 2);
	is!("x=4.0; x.sqrt", 2);
	is!("x=-3; x.abs()", 3);
	is!("x=-3; x.abs", 3);
	is!("x=8.0; x.cbrt", 2);
	is!("x=-2.5; x.norm()", 2.5);
}

#[test]
fn test_a_field_named_like_a_number_method_stays_a_field() {
	is!("p={round:7}; p.round", 7);
	is!("p={\"abs\":5}; p.abs", 5);
}
