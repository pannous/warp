//! An error inside an expanded library template names the value the word was given, not the template's own column
//! (card template-positions: `sum 1` said 'not a list at 1:78')
use crate::common::fails_with;

#[test]
fn template_errors_name_the_value_given() {
	fails_with("sum 1", "not a list at 1:5");
	fails_with("x = 2\ny = sum x", "not a list at 2:9");
	fails_with("square x = x*x; map 3 square", "not a list at 1:21");
	fails_with("each 3 print", "not a list at 1:6");
}
