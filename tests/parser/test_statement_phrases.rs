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
	assert!(matches!(parse("x and print \"x\"").drop_meta(), Node::Key(_, Op::And, _)), "a value on the left: logical and");
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

#[test]
fn after_without_parentheses_waits_for_its_condition() {
	is!("shared n = 0; go { for i in 1 to 5 { n += 1 } }; five = after n == 5 return n * 2; await five", 10);
}

/// card let-if: `and not c` continues the condition (`not` is an operator, not a statement), also after `let`
#[test]
fn and_not_continues_the_condition_after_let() {
	is!("a=1;b=0;c=0; let shown = if (a or b) and not c then 7 else 8; shown", 7);
	is!("a=1;c=1; let shown = if a and not c then 7 else 8; shown", 8);
}

/// card if-then-chain: `and n mod 100 <= 13` continues the condition: a word followed by an infix word (mod, div,
/// contains) is an operand, not a statement `n` with an argument
#[test]
fn and_before_an_infix_word_continues_the_condition() {
	is!("def s(n) = if n mod 100 >= 11 and n mod 100 <= 13 then 1 else 2; s(12)", 1);
	is!("def s(n) = n >= 11 and n div 2 <= 6; s(12)", true);
	is!("def suffix(n) = if n mod 100 >= 11 and n mod 100 <= 13 then \"th\" else if n mod 10 == 1 then \"st\" else if n mod 10 == 2 then \"nd\" else if n mod 10 == 3 then \"rd\" else \"th\"
out = \"\"
for n in [1 2 3 4 11 12 13 21 22 101] {
	if out.length > 0 { out += \" \" }
	out += \"$(n)$(suffix(n))\"
}
out", "1st 2nd 3rd 4th 11th 12th 13th 21st 22nd 101st");
}
