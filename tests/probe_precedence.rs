use wasp::wasp_parser::WaspParser;
use wasp::node::Node;

#[test]
fn test_precedence() {
    // Issue 1: Precedence - should parse as (a:int)=7 not a:(int=7)
    let node = WaspParser::parse("a:int=7");
    println!("\na:int=7 serializes as: {}", node.serialize());
    println!("Structure: {:?}", node);

    // Check if it's Key at top level (correct: (a:int)=7)
    // vs nested Key inside (wrong: a:(int=7))
    match &node {
        Node::Key(k, v) => {
            println!("✓ Top level is Key");
            println!("  Key part: {}", k.serialize());
            println!("  Value part: {}", v.serialize());
        }
        _ => println!("✗ Not a Key at top level"),
    }
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
