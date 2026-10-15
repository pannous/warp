// card braceless-call: a braceless call before a logical chain takes the first comparand, as before a comparison
// (P143): `sq 2 == 4 and yes` is (sq 2) == 4 and yes; an untyped parameter asks which reading is meant
use crate::common::fails_with;
use crate::is;

#[test]
fn a_braceless_call_takes_the_first_comparand_of_a_logical_chain() {
	is!("sq(x):=x*x; sq 2 == 4 and yes", true);
	is!("sq(x):=x*x; sq 2 == 5 or yes", true);
	is!("sq(x):=x*x; sq 2 + 1 == 9 and yes", true);
}

#[test]
fn an_untyped_call_before_a_logical_chain_is_ambiguous() {
	fails_with("say(x) := x\nsay 3 == 3 and yes", "write (say 3) == 3 and yes or say(3 == 3 and yes)");
}
