// A trailing `@name` / `@name(value)` with nothing after it annotates the expression before it (wiki/Purpose.md
// `ys = xs.map(f) @parallel`, card parallel-map); a leading one still annotates the atom that follows
use warp::{parse, Node, Op};

fn assigned_value(code: &str) -> Node {
	let Node::Key(_, Op::Assign, value) = parse(code).drop_meta().clone() else { panic!("expected an assignment: {code}") };
	*value
}

#[test]
fn a_trailing_annotation_marks_the_expression_before_it() {
	let mapped = assigned_value("ys = xs.map(f) @parallel");
	assert_eq!(mapped.attribute("parallel"), Some(&Node::True));
	assert!(matches!(mapped.drop_meta(), Node::Key(_, Op::Dot, _)), "the map call itself: {mapped:?}");
	assert_eq!(assigned_value("x = 1 @unit(cm)").attribute("unit").map(Node::serialize), Some("cm".to_string()));
	let Node::List(statements, _, _) = parse("ys = xs.map(f) @parallel; 3").drop_meta().clone() else { panic!("expected two statements") };
	let Node::Key(_, Op::Assign, mapped) = statements[0].drop_meta().clone() else { panic!("expected an assignment first") };
	assert_eq!(mapped.attribute("parallel"), Some(&Node::True));
}

#[test]
fn a_leading_annotation_still_marks_what_follows() {
	assert_eq!(parse("@draft tee{a:1}").attribute("draft"), Some(&Node::True));
}
