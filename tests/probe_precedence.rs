use wasp::wasp_parser::WaspParser;
use wasp::*;
use wasp::Node::Number;
use wasp::Number::Int;

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
		Node::Key(k, v) => {
			println!("✓ Top level is Key");
			println!("  Key part: {}", k.serialize());
			println!("  Value part: {}", v.serialize());

			// Check if key is also a Key (nested)
			if matches!(k.as_ref(), Node::Key(_, _)) {
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
	// a:{body} should parse as Key("a", Block({body}))
	eq!(    parse("a:{body}"),
        Key(
            Box::new(Symbol("a".s())),
            Box::new(Node::List(vec![Symbol("body".s())], Bracket::Curly, Separator::None))
        )
    );
}

#[test]
fn test_type_annotation_block_with_assignment() {
	let node = WaspParser::parse("a:{x}=7");
	eq!(node, Key(
        Box::new(Symbol("a".s())),
        Box::new(Key(
            Box::new(Node::List(vec![Symbol("x".s())], Bracket::Curly, Separator::None)),
            Box::new(Number(Int(7)))
        ))
    ));
}
