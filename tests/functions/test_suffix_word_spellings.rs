//! P24 (user, 2026-10-05): the suffix form calls a user function, `4 doubled` like `4.double`. Every English past form
//! of the name counts: +d, +ed, a doubled final consonant (`stop` → `stopped`) and y → ied (`copy` → `copied`).
use crate::common::fails_with;
use crate::is;

#[test]
fn test_suffix_form_calls_a_user_function() {
	is!("twice(x):=x*2; 4 twiced", 8);
	is!("halve:=it/2; 8 halved", 4);
	is!("inc(x):=x+1; x=4; x inced", 5);
	is!("square:=it*it; 3 squared", 9);
}

#[test]
fn test_english_spellings_of_the_suffix_form() {
	is!("stop(x):=x+1; 4 stopped", 5);
	is!("trim(x):=x-1; 4 trimmed", 3);
	is!("copy(x):=x*10; 4 copied", 40);
	is!("bump(x):=x+1; 4 bumped", 5); // a final consonant after a consonant is not doubled
}

#[test]
fn test_suffix_forms_chain_and_mix_like_before() {
	is!("stop(x):=x+1; 4 stopped stopped", 6);
	is!("stop(x):=x+1; 4 stopped+1", 6);
	fails_with("stop(x):=x+1; 1+4 stopped", "too ambiguous to guess");
}
