use crate::is;

#[test]
fn test_trailing_decrement_is_not_part_of_the_name() {
	is!("i=3;i--;i", 2);
}

#[test]
fn test_decrement_inside_a_loop_body() {
	is!("i=1;while(i<9 and i > -10){i+=2;i--};i+1", 10);
}

#[test]
fn test_hyphen_between_words_stays_kebab_case() {
	is!("kebab-case=4;kebab-case", 4);
}
