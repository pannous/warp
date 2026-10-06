// Phrases from wiki/thread.md (cards wiki-thread, after-precedence, notes/statement_phrases.md): `and` between two
// statements runs them one after the other, `after C return V` reads C up to the `return`
use crate::is;
use warp::{parse, Node, Op};

const AFTER_RETURN: &str = "after·return";

#[test]
fn and_between_statements_sequences_them() {
	assert_eq!(parse("sleep 1s and print \"second\"").serialize(), parse("sleep 1s; print \"second\"").serialize());
	assert_eq!(parse("go { sleep 1s and print \"second\" }").serialize(), parse("go { sleep 1s; print \"second\" }").serialize());
	is!("f := it*2; f 3 and f 5", 10);
}

#[test]
fn and_after_a_value_stays_logical() {
	assert_eq!(parse("x and print \"x\"").serialize(), parse("x and (print \"x\")").serialize());
	is!("1 and 0", false);
	is!("f := it*2; f(3) and f(5) > 9", true);
}

#[test]
fn after_reads_its_condition_up_to_return() {
	let Node::Key(target, Op::Assign, phrase) = parse("five = after n == 5 return v").drop_meta().clone() else { panic!("expected an assignment") };
	assert_eq!(target.serialize(), "five");
	let Node::List(items, _, _) = phrase.drop_meta() else { panic!("expected the after·return call: {phrase:?}") };
	assert_eq!(items[0].serialize(), AFTER_RETURN);
	assert_eq!(items[1].serialize(), parse("n == 5").serialize());
	assert_eq!(items[2].serialize(), "v");
}

#[test]
fn after_without_return_stays_a_call_listener() {
	assert!(!parse("after tested: print \"ok\"").serialize().contains(AFTER_RETURN));
}
