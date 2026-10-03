//! Which operator the parser puts on top: precedence, associativity, prefix/suffix and glyph forms
//! (condensed from probe_operators.rs, probe_precedence.rs and probe_increment.rs)
use warp::node::Node;
use warp::wasp_parser::parse;
use warp::{is, Op};

fn top_operator(node: &Node) -> Option<Op> {
	match node {
		Node::Key(_, op, _) => Some(*op),
		Node::Meta { node, .. } => top_operator(node),
		_ => None,
	}
}

fn assert_top_operator(code: &str, expected: Op) {
	assert_eq!(top_operator(&parse(code)), Some(expected), "top operator of {code}");
}

#[test]
fn test_arithmetic_basic() {
	assert_top_operator("1 + 2", Op::Add);
}

#[test]
fn test_arithmetic_precedence() {
	assert_top_operator("1 + 2 * 3", Op::Add);
}

#[test]
fn test_comparison() {
	assert_top_operator("a < b", Op::Lt);
}

#[test]
fn test_comparison_eq() {
	assert_top_operator("x == y", Op::Eq);
}

#[test]
fn test_prefix_neg() {
	// -5 is a negative literal, -x a prefix negation
	assert_top_operator("-x", Op::Neg);
}

#[test]
fn test_suffix_square() {
	assert_top_operator("x²", Op::Square);
}

#[test]
fn test_mixed_prefix_infix() {
	assert_top_operator("-5 + 3", Op::Add);
}

#[test]
fn test_logical_and() {
	assert_top_operator("a and b", Op::And);
}

#[test]
fn test_logical_or() {
	assert_top_operator("x or y", Op::Or);
}

#[test]
fn test_unicode_le() {
	assert_top_operator("a ≤ b", Op::Le);
}

#[test]
fn test_unicode_mul() {
	assert_top_operator("x × y", Op::Mul);
}

#[test]
fn test_power_right_assoc() {
	assert_top_operator("2^3^4", Op::Pow);
}

#[test]
fn test_assignment_right_assoc() {
	assert_top_operator("a = b = c", Op::Assign);
}

#[test]
fn test_range() {
	assert_top_operator("1..10", Op::Range);
}

#[test]
fn test_hash_index() {
	assert_top_operator("list#3", Op::Hash);
}

#[test]
fn test_division_type_upgrade() {
	is!("1/2", 0.5);
	is!("3/2", 1.5);
	is!("-1/2", -0.5);
	is!("10/4", 2.5);
	is!("-1/6", -1.0 / 6.0);
}

#[test]
fn typed_name_binds_before_assignment() {
	// (a:int)=7, not a:(int=7)
	let parsed = parse("a:int=7");
	let node = parsed.drop_meta();
	match node {
		Node::Key(key, op, value) => {
			assert_eq!(*op, Op::Assign);
			assert_eq!(top_operator(key), Some(Op::Colon));
			assert_eq!(value.serialize(), "7");
		}
		_ => panic!("Expected Key, got {:?}", node),
	}
}

#[test]
fn equals_and_colon_make_keys_with_their_own_operator() {
	assert_top_operator("a=7", Op::Assign);
	assert_top_operator("a:7", Op::Colon);
}

#[test]
fn keys_with_different_names_differ() {
	assert_ne!(parse("a=7"), parse("b=7"));
}

#[test]
fn test_type_annotation_with_block() {
	// a:{body} is Key("a", Colon, Block({body}))
	let parsed = parse("a:{body}");
	let node = parsed.drop_meta();
	match node {
		Node::Key(key, op, value) => {
			assert!(matches!(key.as_ref().drop_meta(), Node::Symbol(_)), "Key should be Symbol");
			assert_eq!(*op, Op::Colon);
			assert!(matches!(value.as_ref().drop_meta(), Node::List(..)), "Value should be List/Block");
		}
		_ => panic!("Expected Key, got {:?}", node),
	}
}

#[test]
fn test_type_annotation_block_with_assignment() {
	// a:{x}=7 is Key(Key("a", Colon, Block), Assign, 7)
	let parsed = parse("a:{x}=7");
	let node = parsed.drop_meta();
	match node {
		Node::Key(key, op, value) => {
			assert_eq!(*op, Op::Assign);
			assert!(matches!(value.as_ref().drop_meta(), Node::Number(_)), "Value should be Number 7");
			match key.as_ref().drop_meta() {
				Node::Key(inner_key, inner_op, inner_value) => {
					assert_eq!(*inner_op, Op::Colon);
					assert!(matches!(inner_key.as_ref().drop_meta(), Node::Symbol(_)));
					assert!(matches!(inner_value.as_ref().drop_meta(), Node::List(..)));
				}
				other => panic!("Expected nested Key, got {:?}", other),
			}
		}
		_ => panic!("Expected Key, got {:?}", node),
	}
}

#[test]
fn call_with_an_expression_argument_is_name_and_key() {
	let parsed = parse("fib(it-1)");
	let node = parsed.drop_meta();
	match node {
		Node::List(items, _, _) => {
			assert_eq!(items.len(), 2);
			assert_eq!(items[0].serialize(), "fib");
			assert_eq!(top_operator(&items[1]), Some(Op::Sub));
		}
		_ => panic!("Expected List, got {:?}", node),
	}
}

#[test]
fn increment_suffix_and_while_condition_spacing() {
	assert_top_operator("i++", Op::Inc);
	assert_eq!(parse("{i++}").serialize(), "{i++ø}");
	assert_eq!(parse("while(1){2}"), parse("while (1) {2}"));
	assert_eq!(parse("while(i<9){i++}").serialize(), "ø while (i<9) do {i++ø}");
}
