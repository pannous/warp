// card if-instance-branch: an instance in an if-branch stays an instance (the emitter kept its Instance mark only
// outside branches, so `print` showed the data `{T:{…}}`), and two instances or maps as branches are no text
// (found printing an ORM row whose foreign key was loaded: `team:{Team:{…}}`)
use crate::is;

#[test]
fn an_instance_in_one_branch_prints_as_an_instance() {
	is!("class T{n:text}; a = if 1>2 then ø else T{n:\"x\"}; str(a)", "T{n:\"x\"}");
	is!("class T{n:text}; a = [(if 1>2 then ø else T{n:r}) for r in [\"x\"]]; str(a)", "[T{n:\"x\"}]");
}

#[test]
fn two_instance_branches_are_an_instance() {
	is!("class T{n:text}; a = if 1>2 then T{n:\"y\"} else T{n:\"x\"}; str(a)", "T{n:\"x\"}");
	is!("a = if 1>2 then {n:\"y\"} else {n:\"x\"}; str(a)", "{n:\"x\"}");
}
