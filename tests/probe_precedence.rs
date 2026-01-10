use warp::wasp_parser::WaspParser;
use warp::wasp_parser::parse;
use warp::*;
use warp::Node::Number;

#[test]
fn test_precedence() {
	// Issue 1: Precedence - should parse as (a:int)=7 not a:(int=7)
	let node = WaspParser::parse("a:int=7");
	println!("\na:int=7 serializes as: {}", node.serialize());
	println!("Node kind: {:?}", node.kind());

	// Unwrap Meta if needed
	let actual_node = node.drop_meta();
	println!("After drop_meta kind: {:?}", actual_node.kind());
	println!("Structure: {:?}", actual_node);

	// Check if it's Key at top level (correct: (a:int)=7)
	// vs nested Key inside (wrong: a:(int=7))
	match actual_node {
		Node::Key(k, _op, v) => {
			println!("✓ Top level is Key");
			println!("  Key part: {}", k.serialize());
			println!("  Value part: {}", v.serialize());

			// Check if key is also a Key (nested)
			if matches!(k.as_ref(), Node::Key(..)) {
				println!("  ✓ Key is nested Key - CORRECT PRECEDENCE!");
			}
		}
		_ => println!("✗ Not a Key at top level - WRONG!"),
	}

	println!("Full debug: {:?}", node);
}

#[test]
fn test_key_operators() {
	// Issue 2: Different operators
	let eq = WaspParser::parse("a=7");
	let colon = WaspParser::parse("a:7");

	println!("\na=7 => {}", eq.serialize());
	println!("a:7 => {}", colon.serialize());

	// Both create Keys but with different operators
	// Should we store operator type?
}

#[test]
fn test_key_equality() {
	// Issue 3: Key equality semantics
	let a = WaspParser::parse("a=7");
	let b = WaspParser::parse("b=7");

	println!("\na=7: {:?}", a);
	println!("b=7: {:?}", b);
	println!("Are they equal? {}", a == b);

	// Currently: false (different keys)
	// Should they be equal because both equal 7?
}

#[test]
fn test_type_annotation_with_block() {
	// a:{body} should parse as Key("a", Colon, Block({body}))
	let node = parse("a:{body}");
	let actual = node.drop_meta();
	match &actual {
		Node::Key(k, op, v) => {
			assert!(matches!(k.as_ref().drop_meta(), Symbol(_)), "Key should be Symbol");
			assert_eq!(*op, Op::Colon, "Op should be Colon");
			assert!(matches!(v.as_ref().drop_meta(), Node::List(..)), "Value should be List/Block");
		}
		_ => panic!("Expected Key, got {:?}", actual),
	}
}

#[test]
fn test_type_annotation_block_with_assignment() {
	// a:{x}=7 should parse as Key(Key("a", Colon, Block), Assign, 7)
	let node = WaspParser::parse("a:{x}=7");
	let actual = node.drop_meta();
	match &actual {
		Node::Key(k, op, v) => {
			// Outer Key should have Assign op
			assert_eq!(*op, Op::Assign, "Outer op should be Assign");
			assert!(matches!(v.as_ref().drop_meta(), Number(_)), "Value should be Number 7");

			// Inner key should be Key with Colon op
			match k.as_ref().drop_meta() {
				Node::Key(inner_k, inner_op, inner_v) => {
					assert_eq!(*inner_op, Op::Colon, "Inner op should be Colon");
					assert!(matches!(inner_k.as_ref().drop_meta(), Symbol(_)));
					assert!(matches!(inner_v.as_ref().drop_meta(), Node::List(..)));
				}
				other => panic!("Expected nested Key, got {:?}", other),
			}
		}
		_ => panic!("Expected Key, got {:?}", actual),
	}
}

#[test]
fn probe_fib_parsing() {
	println!("\n=== fib(it-1) ===");
	let node = parse("fib(it-1)");
	println!("kind: {:?}, serialized: {}", node.kind(), node.serialize());

	if let Node::List(items, _, _) = node.drop_meta() {
		println!("List with {} items:", items.len());
		for (i, item) in items.iter().enumerate() {
			println!("  [{}] kind={:?}: {}", i, item.kind(), item.serialize());
		}
	}
}
