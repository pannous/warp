

#[test]
fn test_remove() {
    let result = parse("a b c d");
    result.remove(1, 2);
    replaced = parse("a d");
    assert!(result == replaced);
}


#[test]
fn test_remove2() {
    let result = parse("a b c d");
    result.remove(2, 10);
    replaced = parse("a b");
    assert!(result == replaced);
}

#[test]
fn test_replace() {
    let result = parse("a b c d");
    result.replace(1, 2, Node("x"));
    replaced = parse("a x d");
    assert!(result == replaced);
}


#[test]
fn test_mark_as_map() {
    compare = Node();
    //	compare["d"] = Node();
    compare["b"] = 3;
    compare["a"] = "HIO";
    Node & dangling = compare["c"];
    assert!(dangling.isNil());
    //     assert!(Nil();
    assert!(dangling == Empty);
    assert!(&dangling != &Empty); // not same pointer!
    dangling = Node(3);
    //	dangling = 3;
    assert!(dangling == 3);
    assert!(compare["c"] == 3);
    eq!(compare["c"], Node(3));
    Node & node = compare["a"];
    assert!(node == "HIO");
    //     chars
    source = "{b:3 a:'HIO' c:3}"; // d:{}
    marked = parse(source);
    Node & node1 = marked["a"];
    assert!(node1 == "HIO");
    assert!(compare["a"] == "HIO");
    assert!(marked["a"] == "HIO");
    assert!(node1 == compare["a"]);
    assert!(marked["a"] == compare["a"]);
    assert!(marked["b"] == compare["b"]);
    assert!(compare == marked);
}
