// Metadata tests
// Migrated from tests_*.rs files

#[test]
fn test_meta_field() {
    tee = parse("tee{a:1}");
    tee["a"]["@attrib"] = 42;
    tee["a"]["@attrib2"] = 43;
    // tee["a"].setMeta("attrib2",(Node) 43);
    // tee["a"].metas()["attrib2"]=(Node) 43;
    eq!(tee.name, "tee");
    assert!(tee["a"]["@attrib"]);
    assert!(tee["a"]["@attrib2"]);
    assert!(tee["a"] == 1);
    assert!(tee.length() == 1);
    assert!(tee["a"]["@attrib"].value.longy == 42);
    assert!(tee["a"]["@attrib2"].value.longy == 43);
    eq!(tee.serialize(), "tee{@attrib(42) @attrib2(43) a:1}");
}

#[test]
fn test_meta() {
    tee = parse("tee{a:1}");
    tee["@attrib"] = 42;
    tee["@attrib2"] = 43;
    eq!(tee.name, "tee");
    eq!(tee.serialize(), "@attrib(42) @attrib2(43) tee{a:1}");
    assert!(tee["@attrib"]);
    assert!(tee["@attrib2"]);
    assert!(tee["a"] == 1);
    assert!(tee.length() == 1);
    assert!(tee["@attrib"].value.longy == 42);
    assert!(tee["@attrib2"].value.longy == 43);
}

#[test]
fn test_meta_at() {
    eq!(parse("tee{a:1}").name, "tee");
    eq!(parse("tee{a:1}").serialize(), "tee{a:1}");
    let code = "@attrib tee{a:1}";
    let node = parse(code);
    assert!(node.name == "tee");
    assert!(node.length() == 1);
    assert!(node["a"] == 1);
    assert!(node["@attrib"]);
}

#[test]
fn test_meta_at2() {
    let code = "@attrib(1) @attrib2(42) tee{a:1}";
    let node = parse(code);
    assert!(node.name == "tee");
    assert!(node.length() == 1);
    assert!(node["a"] == 1);
    // eq!(node.serialize(),code); // todo ok except order!
    assert!(node["@attrib"]);
    assert!(node["@attrib2"]);
    eq!(node["@attrib"], 1);
    eq!(node["@attrib2"], 42);
}

#[test]
fn test_parent_context() {
    //     chars
    source = "{a:'HIO' d:{} b:3 c:ø}";
    assert_parses(source);
    result.print();
    Node & a = result["a"];
    a.print();
    eq!(a.kind(), strings);
    eq!(a.value.string, "HIO");
    eq!(a.string(), "HIO"); // keyNodes go to values!
    assert!(a == "HIO");
    //	eq!(a.name,"a" or"HIO");// keyNodes go to values!
    skip!(

        eq!(a.kind(), key);
        assert!(a.name == "HIO");
    );
}

#[test]
fn test_parent() {
    skip!( // not in rust!
        //	chars source = "{a:'HIO' d:{} b:3 c:ø}";
    //     chars
        source = "{a:'HIO'}";
        assert_parses(source);
        Node & a = result["a"];
        // print(a);
        assert!(a.kind == key or a.kind == strings);
        assert!(a == "HIO");
        assert!(a.parent == 0); // key is the highest level
    //     Node * parent = a.value.node -> parent;
        assert!(parent);
        // print(parent); // BROKEN, WHY?? let's find out:
        assert!(*parent == result);
        skip!(

            // pointer identity broken by flat() ?
            assert!(parent == &result);
        );
        testParentContext(); // make sure parsed correctly
    );
}
