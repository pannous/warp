// `name!` glued to its name and followed by a space is the required-field mark, also between spaced fields:
// `class person{name! email?}` is `class person{name!; email?}`
use crate::common::fails_with;
use warp::is;

#[test]
fn spaced_fields_keep_the_required_mark() {
	fails_with("class person{name! email?}; person{email:'a@b'}", "person needs field name");
	is!("class person{name! email?}; p=person{name:\"Jo\"}; p.name", "Jo");
	is!("class person{name! email?}; person{name:\"Jo\" email:\"j@x\"}.email", "j@x");
}
