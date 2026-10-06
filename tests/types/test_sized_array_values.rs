// `surface = height*width int` (issue #15, the circle demo): a computed count before a type word is a zero-filled typed
// array, as `x : 100 int` and `x = h*w * int` are; the parser reads it as `(surface = height*width) int`
use crate::is;

#[test]
fn a_computed_count_before_a_type_word_is_a_typed_array() {
	is!("height=4; width=4; surface=height*width int; #surface", 16);
	is!("height=4; width=4; surface=height*width int; surface[3]=1; surface[3] + surface[2]", 1);
	is!("n=3; x : n*2 float; #x", 6);
	is!("surface = 16 int; #surface", 16);
}
