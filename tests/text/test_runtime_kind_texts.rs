// Two values whose kinds are known only at run time add as their kinds do: two texts concatenate (they were "not a
// number"), two numbers add, two lists concatenate
use crate::is;

#[test]
fn two_texts_of_run_time_kind_concatenate() {
	is!("xs = [\"a\", 1]; xs#1 + xs#1", "aa");
	is!("xs = [\"a\", 1]; xs#2 + xs#2", 2);
}
