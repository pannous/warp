#[test]
fn testRemove() {
    result = parse("a b c d");
    result.remove(1, 2);
    replaced = parse("a d");
    assert!(result == replaced);
}


#[test]
fn testRemove2() {
    result = parse("a b c d");
    result.remove(2, 10);
    replaced = parse("a b");
    assert!(result == replaced);
}

#[test]
fn testReplace() {
    result = parse("a b c d");
    result.replace(1, 2, Node("x"));
    replaced = parse("a x d");
    assert!(result == replaced);
}


#[test]
fn testMarkAsMap() {
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
    assert(node == "HIO");
    //     chars
    source = "{b:3 a:'HIO' c:3}"; // d:{}
    marked = parse(source);
    Node & node1 = marked["a"];
    assert(node1 == "HIO");
    assert!(compare["a"] == "HIO");
    assert!(marked["a"] == "HIO");
    assert(node1 == compare["a"]);
    assert(marked["a"] == compare["a"]);
    assert(marked["b"] == compare["b"]);
    assert(compare == marked);
}
#[test]
fn testMarkSimple() {
    print("testMarkSimple");
    // [] = "1";
    x = assert_parses(xx);
    a = assert_parses("{aa:3}");
    eq!(a.value.longy, (int64) 3);
    eq!(a, int64(3));
    assert(a == 3);
    //     assert(a.kind == longs or a.kind == key and a.value.node->kind == longs);
    assert(a.name == "aa");
    //	assert(a3.name == "a"s);// todo? cant
    Node & b = a["b"];
    a["b"] = a;
    assert(a["b"] == a);
    assert(a["b"] == b);
    assert(a["b"] == 3);

    assert(parse("3.") == 3.);
    assert(parse("3.") == 3.f);
    //	assert(Mark::parse("3.1") == 3.1); // todo epsilon 1/3≠0.33…
    //	assert(Mark::parse("3.1") == 3.1f);// todo epsilon
    result = parse("'hi'");
    assert!(result.kind == strings);
    assert!(*result.value.string == "hi");
    assert!(result == "hi");
    assert(parse("'hi'") == "hi");
    assert(parse("3") == 3);
}
