use wasp::node::Node;
use wasp::wasp_parser::parse;

#[test]
fn test_remove() {
    let result = parse("a b c d");
    result.remove(1, 2);
    let replaced = parse("a d");
    assert!(result == replaced);
}


#[test]
fn test_remove2() {
    let result = parse("a b c d");
    result.remove(2, 10);
    let replaced = parse("a b");
    assert!(result == replaced);
}

#[test]
fn test_replace() {
    let result = parse("a b c d");
    // result.replace(1, 2, Node("x"));
    let replaced = parse("a x d");
    assert!(result == replaced);
}


#[test]
fn test_mark_as_map() {
    let compare = Node::new();
    //	compare["d"] = Node();
    compare["b"] = 3.into();
    compare["a"] = "HIO".into();
    let dangling : Node = compare["c"];
    assert!(dangling.isNil());
    //     assert!(Nil();
    assert!(dangling == Empty);
    assert!(&dangling != &Empty); // not same pointer!
    dangling = Node(3);
    //	dangling = 3;
    assert!(dangling == 3);
    assert!(compare["c"] == 3);
    eq!(compare["c"], Node(3));
    let node : Node = compare["a"];
    assert!(node == "HIO");
    //     chars
    source = "{b:3 a:'HIO' c:3}"; // d:{}
    marked = parse(source);
    let node1 : Node = marked["a"];
    assert!(node1 == "HIO");
    assert!(compare["a"] == "HIO");
    assert!(marked["a"] == "HIO");
    assert!(node1 == compare["a"]);
    assert!(marked["a"] == compare["a"]);
    assert!(marked["b"] == compare["b"]);
    assert!(compare == marked);
}
