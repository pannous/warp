// A match subject named like a variable that elsewhere holds a function is matched, not called with the arms block
use crate::is;

#[test]
fn a_match_subject_is_no_braceless_closure_call() {
	is!("passes = [x => x + 1]\nfor view in passes { view(1) }\nkind(view: any) := match view {\n\t[1, ...rest] => \"one\"\n\t_ => \"other\"\n}\nkind([1, 2])", "one");
}
