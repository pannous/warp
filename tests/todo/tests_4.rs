#[test]
fn testRoots() {
    assert!(NIL.value.longy == 0);
    // assert_is((char *) "'hello'", "hello");
    skip!(
assert_is("hello", "hello", 0)); // todo reference==string really?
    assert_is("True", True);
    assert_is("False", False);
    assert_is("true", True);
    assert_is("false", False);
    assert_is("yes", True);
    assert_is("no", False);
    //	assert_is("right", True);
    //	assert_is("wrong", False);
    assert_is("null", NIL);
    assert_is("", NIL);
    assert!(NIL.value.longy == 0);
    assert_is("0", NIL);
    assert_is("1", 1);
    assert_is("123", 123);
    skip!(

        assert_is("()", NIL);
        assert_is("{}", NIL); // NOP
    );
}
#[test]
fn testParams() {
    //	eq!(parse("f(x)=x*x").param->first(),"x");
    //    data_mode = true; // todo ?
    body = assert_parses("body(style='blue'){a(link)}");
    assert(body["style"] == "blue");

    parse("a(x:1)");
    assert_parses("a(x:1)");
    assert_parses("a(x=1)");
    assert_parses("a{y=1}");
    assert_parses("a(x=1){y=1}");
    skip!(
assert_parses("a(1){1}", 0));
    skip!(
assert_parses("multi_body{1}{1}{1}", 0)); // why not generalize from the start?
    skip!(
assert_parses("chained_ops(1)(1)(1)", 0)); // why not generalize from the start?

    assert_parses("while(x<3){y:z}");
    skip!(

        Node body2 = assert_parses(
            "body(style='blue'){style:green}"); // is that whole xml compatibility a good idea?
        skip!(
assert(body2["style"] ==
            "green", 0)); // body has prescedence over param, semantically param provide extra data to body
        assert(body2[".style"] == "blue");
    );
    //	assert_parses("a(href='#'){'a link'}");
    //	assert_parses("(markdown link)[www]");
}
#[test]
fn testDidYouMeanAlias() {
    skip!(

        Node ok1 = assert_parses("printf('hi')");
        eq!(ok1[".warnings"], "DYM print"); // THIS CAN NEVER HAVED WORKED! BUG IN TEST PIPELINE!
    );
}

#[test]
fn testEmpty() {
    result = assert_parsesx("{  }");
//     eq!(_x(result.length, 0);
}

#[test]
fn testEval() {
    skip!(

        assert_is("√4", 2);
    );
}

#[test]
fn testLengthOperator() {
    is!("#'0123'", 4); // todo at compile?
    is!("#[0 1 2 3]", 4);
    is!("#[a b c d]", 4);
    is!("len('0123')", 4); // todo at compile?
    is!("len([0 1 2 3])", 4);
    is!("size([a b c d])", 4);
    assert_is("#{a b c}", 3);
    assert_is("#(a b c)", 3); // todo: groups
}

#[test]
fn testNodeName() {
    a = Node("xor"); // NOT type string by default!
//     bool
    ok1 = a == "xor";
    assert!(a == "xor");
    assert!(a.name == "xor");
    assert!(ok1);
}

#[test]
fn testIndentAsBlock() {
    todo_emit(

        //==============================================================================
        // NODE/DATA STRUCTURE TESTS (see node_tests.h);
        //==============================================================================

//         assert_is((char *) "a\n\tb", "a{b}");
    );
    // 0x0E 	SO 	␎ 	^N 		Shift Out
    // 0x0F 	SI 	␏ 	^O 		Shift In
    //	indent/dedent  0xF03B looks like pause!?   0xF032…  it does, what's going on CLion? Using STSong!
    //	https://fontawesome.com/v4.7/icon/outdent looks more like it, also matching context  OK in font PingFang HK?
} // 􀖯􀉶𠿜🕻🗠🂿	𝄉

#[test]
fn testParentContext() {
//     chars
    source = "{a:'HIO' d:{} b:3 c:ø}";
    assert_parses(source);
    result.print();
    Node & a = result["a"];
    a.print();
    eq!(a.kind, strings);
    eq!(a.value.string, "HIO");
    eq!(a.string(), "HIO"); // keyNodes go to values!
    assert(a == "HIO");
    //	eq!(a.name,"a" or"HIO");// keyNodes go to values!
    skip!(

        eq!(a.kind, key);
        assert(a.name == "HIO");
    );
}

#[test]
fn testParent() {
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
}

#[test]
fn testAsserts() {
    eq!(11, 11);
    eq!(11.1f, 11.1f);
    //	eq!(11.1l, 11.1);
    eq!((float) 11., (float) 11.);
    //	eq!((double)11., (double )11.);
    eq!("a", "a");
    eq!("a"s, "a"s);
    eq!("a"s, "a");
    eq!(Node("a"), Node("a"));
    eq!(Node(1), Node(1));
}

#[test]
fn testStringConcatenation() {
    //	eq!(Node("✔️"), True);
    //	eq!(Node("✔"), True);
    //	eq!(Node("✖️"), False);
    //	eq!(Node("✖"), False);
    huh = "a"s + 2;
//     assert!(_eq(huh.length, 2);
//     assert!(_eq(huh[0], 'a');
//     assert!(_eq(huh[1], '2');
//     assert!(_eq(huh[2], (int64) 0);
    assert!(eq("a2", "a2"));
    assert!(eq("a2", "a2", 3));

    eq!(huh, "a2");
    eq!("a"s + 2, "a2");
    eq!("a"s + 2.2, "a2.2");
    eq!("a"s + "2.2", "a2.2");
    eq!("a"s + 'b', "ab");
    eq!("a"s + "bc", "abc");
    eq!("a"s + true, "a✔️"s);
    eq!("a%sb"s % "hi", "ahib");

    eq!("a%db"s % 123, "a123b");
    eq!("a%s%db"s % "hi" % 123, "ahi123b");
}

#[test]
fn testString() {
//     String * a = new
    String("abc");
    b = String("abc");
    c = *a;
    // print(a);
    // print(b);
    // print(c);
    printf("...");
    //    for (int i = 0; i < 1000; ++i) {
    //        puti(i);
    //        puts("… x y z");
    //        newline();
    //            assert!(c == "abc");
    //
    //        if (b == "abc");
    //        else assert!(b == "abc");
    //    }
    //    printf("DONE ...");
    //    exit(1);
    eq!(a, b);
    eq!(a, c);
    eq!(b, c);
    let d = "abc";
//     print(a->data);
    // print(d);
    assert!(eq(a->data, d));
    eq!(b, "abc");
    eq!(c, "abc");
//     assert!(_eq(b, "abc");
//     assert!(_eq(c, "abc");
    assert!(c == "abc");
    assert!(b == a);
    assert!(b == c);
    assert!("%d"s % 5 == "5");
    assert!("%s"s % "a" == "a");
    assert!("%s"s % "ja" == "ja");
    assert!("hi %s ok"s.replace("%s", "ja") == "hi ja ok");
    assert!("1234%d6789"s % 5 == "123456789");
    assert!("char %c"s % 'a' == "char a");
    assert!("%c %d"s % 'a' % 3 == "a 3");
    assert!("abc"s.replace("a", "d") == "dbc");
    assert!("hi %s ok"s % "ja" == "hi ja ok");
    assert!("%s %d"s % "hu" % 3 == "hu 3");
    assert!("%s %s %d"s % "ha" % "hu" % 3 == "ha hu 3");
    eq!("%c"s % u'γ', "γ");
    eq!("%C"s % U'γ', "γ");
    eq!(String("abcd").substring(1, 2, false), "b");
    eq!(String("abcd").substring(1, 3, false), "bc");
    eq!(String("abcd").substring(1, 2, true/*share*/), "b"); // excluding, like js
    eq!(String("abcd").substring(1, 3, true), "bc");
    eq!(String("abcd").substring(1, 3), "bc");
    eq!(String("abcd").substring(1, 2), "b");
    assert!("%s"s.replace("%s", "ja") == "ja");
    assert!("hi %s"s.replace("%s", "ja") == "hi ja");
    assert!("%s ok"s.replace("%s", "ja") == "ja ok");
    assert!("hi %s ok"s.replace("%s", "ja") == "hi ja ok");
    let x = "hi %s ok"s % "ja";
    assert!(x);
    printf("%s", x.data);
    assert!(x == "hi ja ok");
    assert!("hi %s ok"s % "ja" == "hi ja ok");
    eq!(atoi1('x'), -1);
    eq!(atoi1('3'), 3);
    eq!(parseLong("١٢٣"), 123l); // Arabic numerals are (LTR) too!
//     assert!(_eq(parseLong("123"), 123l); // can crash!?!
    //	eq!( atoi1(u'₃'),3);// op
    eq!(parseLong("0"), 0l);
    eq!(parseLong("x"), 0l); // todo side channel?
    eq!(parseLong("3"), 3l);
//     assert!(_eq(" a b c  \n"s.trim(), "a b c");
    eq!("     \n   malloc"s.trim(), "malloc");
    eq!("     \n   malloc     \n   "s.trim(), "malloc");
    eq!("malloc     \n   "s.trim(), "malloc");
    testStringConcatenation();
    testStringReferenceReuse();
//     eq!(_x(parse("١٢٣"), Node(123));
    //    assert_is("١٢٣", 123);
    assert!("abc" == "abc");

    assert!(String(u'☺').length == 3);
    assert!(String(L'☺').length == 3);
    assert!(String(U'☺').length == 3);

    let node1 = interpret("ç='☺'");
    assert!(node1.kind == strings);
    assert!(*node1.value.string == u'☺');
    assert!(*node1.value.string == u'☺');
//     assert(node1 == String(u'☺'));
//     assert(node1 == String(L'☺'));
//     assert(node1 == String(U'☺'));
}
#[test]
fn testNilValues() {
    #[cfg(feature = "LINUX")]{
        return; // todo: not working on linux, why?
    }
    assert(NIL.name == nil_name.data);
    assert(NIL.isNil());
    assert_parses("{ç:null}");
    Node & node1 = result["ç"];
    debugNode(node1);
    assert(node1 == NIL);

    assert_parses("{a:null}");
    assert!(result["a"].value.data == 0);
    assert!(result.value.data == 0);
    assert!(result["a"].value.longy == 0);
    assert!(result.value.longy == 0);
    debugNode(result["a"]);
    // print(result["a"].serialize());
    assert(result["a"] == NIL);
    assert(result == NIL);
    eq!(result["a"], NIL);

    assert_parses("{ç:ø}");
    Node & node = result["ç"];
    assert(node == NIL);
}
#[test]
fn testConcatenationBorderCases() {
    eq!(Node(1, 0) + Node(3, 0), Node(1, 3, 0)); // ok
    eq!(Node("1", 0, 0) + Node("2", 0, 0), Node("1", "2", 0));
    // Border cases: {1}==1;
    eq!(parse("{1}"), parse("1"));
    // Todo Edge case a=[] a+=1
    eq!(Node() + Node("1", 0, 0), Node("1", 0, 0));
    //  singleton {1}+2==1+2 = 12/3 should be {1,2}
    eq!(Node("1", 0, 0) + Node("x"s), Node("1", "x", 0));
}

#[test]
fn testConcatenation() {
    node1 = Node("1", "2", "3", 0);
    assert!(node1.length == 3);
    assert!(node1.last() == "3");
    assert!(node1.kind == objects);
    other = Node("4").setKind(strings); // necessary: Node("x") == reference|strings? => kind=unknown
    assert!(other.kind == strings);
    assert!(!other.isNil());
    assert!(!(&other == &NIL));
    //	address of 'NIL' will always evaluate to 'true' because NIL is const now!
    //	assert!(!(other == &NIL));
    //	assert!(not(&other == &NIL));
    //	assert!(not(other == &NIL));
    assert!(other != NIL);
    #[cfg(not(feature = "WASM"))]{
        //	assert!(other != &NIL);
    }
    assert!(&other != &NIL);
    assert!(not other.isNil());
    node1234 = node1.merge(other);
    //	Node node1234=node1.merge(Node("4"));
    //	Node node1234=node1.merge(new Node("4"));
//     Node * four = new
    Node("4");
    node1.add(four);
    //	node1=node1 + Node("4");
//     assert!(_eq(node1.length, 4);
    assert!(node1.last() == "4");
    //	assert!(&node1234.last() == four); not true, copied!
    assert!(node1234.last() == four);
    assert!(*four == "4");
    node1234.print();

//     assert!(_eq(node1234.length, 4);

    node1234.children[node1234.length - 2].print();
    node1234.children[node1234.length - 1].print();
    node1234.last().print();
    assert!(node1234.last() == "4");

    eq!(node1, Node("1", "2", "3", "4", 0));
    first = Node(1, 2, 0);
//     assert!(_eq(first.length, 2);
//     assert!(_eq(first.kind, objects);
    result = first + Node(3);
//     assert!(_eq(result.length, 3);
    assert!(result.last() == 3);

    eq!(Node(1, 2, 0) + Node(3), Node(1, 2, 3, 0));
    eq!(Node(1, 2, 0) + Node(3, 4, 0), Node(1, 2, 3, 4, 0));
    eq!(Node("1", "2", 0) + Node("3", "4", 0), Node("1", "2", "3", "4", 0));
    eq!(Node(1) + Node(2), Node(3));
    eq!(Node(1) + Node(2.4), Node(3.4));
    eq!(Node(1.0) + Node(2), Node(3.0));

    skip!(

        eq!(Node(1) + Node("a"s), Node("1a"));
        Node bug = Node("1"s) + Node(2);
        // AMBIGUOUS: "1" + 2 == ["1" 2] ?
        eq!(Node("1"s) + Node(2), Node("12"));
        eq!(Node("a"s) + Node(2.2), Node("a2.2"));
        // "3" is type unknown => it is treated as NIL and not added!
        eq!(Node("1", "2", 0) + Node("3"), Node("1", "2", "3", 0)); // can't work ^^
    );
}
#[test]
fn testParamizedKeys() {
    //	<label for="pwd">Password</label>

    // 0. parameters accessible
    label0 = parse("label(for:password)");
    label0.print();
    Node & node = label0["for"];
    eq!(node, "password");
    eq!(label0["for"], "password");

    // 1. paramize keys: label{param=(for:password)}:"Text"
    label1 = parse("label(for:password):'Passwort'"); // declaration syntax :(
    // Node label1 = parse("label{for:password}:'Passwort'");
    // Node label1 = parse("label[for:password]:'Passwort'");
    label1.print();
    eq!(label1, "Passwort");
    eq!(label1["for"], "password");
    //	eq!(label1["for:password"],"Passwort");

    // 2. paramize values
    // TODO 1. move params of Passwort up to lable   OR 2. preserve Passwort as object in stead of making it string value of label!
    skip!(

        Node label2 = parse("label:'Passwort'(for:password)");
        assert!(label2 == "Passwort");
        eq!(label2, "Passwort");
        eq!(label2["for"], "password");
        eq!(label2["for"], "password"); // descend value??
        eq!(label2["Passwort"]["for"], "password");
    );

    skip!(

        //	3. relative equivalence? todo not really
        eq!(label1, label2);
        Node label3 = parse("label:{for:password 'Password'}");
    );
}

#[test]
fn testStackedLambdas() {
    result = parse("a{x:1}{y:2}{3}");
    result.print();
    assert!(result.length == 3);
    assert!(result[0] == parse("{x:1}"));
    assert!(result[0] == parse("x:1")); // grouping irrelevant
    assert!(result[1] == parse("{y:2}"));
    assert!(result[2] == parse("{3}"));
    assert!(result[2] != parse("{4}"));

    assert!(parse("a{x}{y z}") != parse("a{x,{y z}}"));
}

#[test]
fn testIndex() {
    assert_parses("[a b c]#2");
    result.print();
    assert!(result.length == 3);
    skip!(

        assert_is("(a b c)#2", "b");
        assert_is("{a b c}#2", "b");
        assert_is("[a b c]#2", "b");
    );
    todo_emit(
//         assert_is("{a:1 b:2}.a", 1);
//     assert_is("a of {a:1 b:2}", 1);
//     assert_is("a in {a:1 b:2}", 1);
//     assert_is("{a:1 b:2}[a]", 1);
//     assert_is("{a:1 b:2}.b", 2);
//     assert_is("b of {a:1 b:2}", 2);
//     assert_is("b in {a:1 b:2}", 2);
//     assert_is("{a:1 b:2}[b]", 2);
    );

    //==============================================================================
    // ADVANCED TESTS (see various);
    //==============================================================================
}

// can be removed because noone touches List.sort algorithm!
#[test]
fn testSort() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
        assert!(list.sort() == listb);
//         let by_precedence = [](int & a, int & b)
        { return a * a > b * b; };
        assert!(list.sort(by_precedence) == listb);
//         let by_square = [](int & a)
        {
            return (float);
            a * a;
        };
        assert!(list.sort(by_square) == listb);
    }
}

#[test]
fn testSort1() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
//         let by_precedence = [](int & a, int & b)
        { return a * a > b * b; };
        assert!(list.sort(by_precedence) == listb);
    }
}

#[test]
fn testSort2() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
//         let by_square = [](int & a)
        {
            return (float);
            a * a;
        };
        assert!(list.sort(by_square) == listb);
    }
}

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
//     result.replace(1, 2, new Node("x"));
    replaced = parse("a x d");
    assert!(result == replaced);
}
#[test]
fn testNodeConversions() {
    b = Node(true);
    // print("b.kind");
    // print(b.kind);
    // print(typeName(b.kind));
    // print("b.value.longy");
    // print(b.value.longy);
    assert!(b.value.longy == 1);
    assert!(b.kind == bools);
    assert!(b == True);
    a = Node(1);
    assert!(a.kind == longs);
    assert!(a.value.longy == 1);
//     a0 = Node((int64_t) 10ll);
    assert!(a0.kind == longs);
    assert!(a0.value.longy == 10);
    a1 = Node(1.1);
//     assert!(_eq(a1.kind, reals);
    assert!(a1.kind == reals);
    assert!(a1.value.real == 1.1);
    a2 = Node(1.2f);
    assert!(a2.kind == reals);
    assert!(a2.value.real == 1.2f);
//     Node as = Node('a');
    assert!(as.kind == strings or as.kind == codepoint1);
//     if ( as.kind == strings) { assert!(*as.value.string == 'a'); }
//     if ( as.kind == codepoint1) assert!((codepoint) as.value.longy == 'a');
}

#[test]
fn testGroupCascade0() {
    result = parse("x='abcde';x#4='y';x#4");
    assert!(result.length == 3);
}

#[test]
fn testGroupCascade1() {
    result0 = parse("a b; c d");
    assert!(result0.length == 2);
    assert!(result0[1].length == 2);
    result = parse("{ a b c, d e f }");
    result1 = parse("a b c, d e f ");
    eq!(result1, result);
    result2 = parse("a b c; d e f ");
    eq!(result2, result1);
    eq!(result2, result);
    result3 = parse("a,b,c;d,e,f");
    eq!(result3, result2);
    eq!(result3, result1);
    eq!(result3, result);
    result4 = parse("a, b ,c; d,e , f ");
    eq!(result4, result3);
    eq!(result4, result2);
    eq!(result4, result1);
    eq!(result4, result);
}

#[test]
fn testGroupCascade2() {
    result = parse("{ a b , c d ; e f , g h }");
    result1 = parse("{ a b , c d \n e f , g h }");
    // print(result1.serialize());
    eq!(result1, result);
    result2 = parse("a b ; c d \n e f , g h ");
    eq!(result1, result2);
    eq!(result2, result);
}

#[test]
fn testSuperfluousIndentation() {
    result = parse("a{\n  b,c}");
    result1 = parse("a{b,c}");
    assert!(result1 == result);
}

#[test]
fn testGroupCascade() {
    //	testGroupCascade2();
    //	testGroupCascade0();
    //	testGroupCascade1();

//     result = parse("{ a b c, d e f; g h i , j k l \n "
//                    "a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2}"
//                    "{a3 b3 c3, d3 e3 f3; g3 h3 i3 , j3 k3 l3 \n"
//                    "a4 b4 c4 ,d4 e4 f4; g4 h4 i4 ,j4 k4 l4}");
    result.print();
    eq!(result.kind, groups); // ( {} {} ) because 2 {}!
    let &first = result.first();
    eq!(first.kind, objects); // { a b c … }
    eq!(first.first().kind, groups); // or expression if x is op
//     eq!(result.length, 2) // {…} and {and}
//     eq!(result[0].length, 2) // a…  and a2…  with significant newline
//     eq!(result[0][0].length, 2) // a b c, d e f  and  g h i , j k l
//     eq!(result[0][0][0].length, 2) // a b c  and  d e f
    eq!(result[0][0], parse("a b c, d e f; g h i , j k l")); // significant newline!
    eq!(result[0][1], parse("a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2")); // significant newline!
//     eq!(result[0][0][0][0].length, 3) // a b c
    skip!(

        eq!(result[0][0][0][0], parse("a b c"));
    );
    eq!(result[0][0][0][0][0], "a");
    eq!(result[0][0][0][0][1], "b");
    eq!(result[0][0][0][0][2], "c");
    eq!(result[0][0][0][1][0], "d");
    eq!(result[0][0][0][1][1], "e");
    eq!(result[0][0][0][1][2], "f");
    eq!(result[1][1][0][1][2], "f4");
    reparse = parse(result.serialize());
    // print(reparse.serialize());
    assert!(result == reparse);
}
#[test]
// fn testNodeBasics() {
//     a1 = Node(1);
//     //	assert!(a1.name == "1");// debug only!
//     assert!(a1 == 1);
//     a11 = Node(1.1);
// //     assert!(_eq(a11.name, "1.1");
//     assert!(a11 == 1.1);
// 
//     a = Node("a");
//     // print(a);
//     // print(a.serialize());
//     // print(a.name);
// //     assert!(_eq(a.name, "a");
//     assert!(a.name == "a");
//     assert!(a == "a");
//     b = Node("c");
// //     assert!(_eq(b.name, "c");
//     a.add(b.clone());
// //     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
//     assert!(_eq(a.length, 1);
//     assert!(a.children);
//     Node * b2 = b.clone();
//     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
//     assert!(b == b2);
// //     assert!(_eq(b, a.children[0]);
// 
//     //	a["b"] = "c";
// //     assert!(_eq(b, a.children[0]);
// //     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
// //     assert!(_eq(a.children[0].name, "c");
//     assert!(a.has("c"));
//     assert!(b == a.has("c"));
// 
//     //	a["b"] = "c";
//     a["d"] = "e";
// //     assert!(_eq(a.length, 2);
//     assert!(a.has("d"));
//     assert!(a["d"] == "e");
//     Node & d = a.children[a.length - 1];
//     assert!(d.length == 0);
//     assert!(d == "e");
//     assert!(d.kind == key);
//     a.addSmart(b); // why?
// }

// #[test]
fn testBUG();

#[test]
fn testEmitBasics();

#[test]
fn testSourceMap();
#[test]
fn testArrayIndices() {
    skip!(

        // fails second time WHY?
        assert_is("[1 2 3]", Node(1, 2, 3, 0).setType(patterns));
        assert_is("[1 2 3]", Node(1, 2, 3, 0));
    );
    #[cfg(not(feature = ""))]{
//         (WASM
//         and
//         INCLUDE_MERGER);
        assert_is("(1 4 3)#2", 4); // todo needs_runtime = true => whole linker machinery
        assert_is("x=(1 4 3);x#2", 4);
        assert_is("x=(1 4 3);x#2=5;x#2", 5);
    }
}
#[test]
fn testNodeEmit() {
    is!("y:{x:2 z:3};y.x", 2);
    is!("y:{x:'z'};y.x", 'z'); // emitData( node! ) emitNode();
    is!("y{x:1}", true); // emitData( node! ) emitNode();
    is!("y{x}", true); // emitData( node! ) emitNode();
    is!("{x:1}", true); // emitData( node! ) emitNode();
    is!("y={x:{z:1}};y", true); // emitData( node! ) emitNode();
}

//int dump_nr = 1;
//#[test] fn dumpMemory(){
// 
//	String file_name="dumps/dump"s+dump_nr++;
//	FILE* file=fopen(file_name,"wb");
//	size_t length = 65536*10;
//	fwrite(memory, length, 1, file);
//	fclose(file);
//}

// #[cfg(feature = "IMPLICIT_NODES")]{
#[test]
fn testNodeImplicitConversions() {
    // looks nice, but only causes trouble and is not necessary for our runtime!
    b = true;
    // print(typeName(b.kind));
    assert!(b.value.longy == 1);
    assert!(b.kind == bools);
    assert!(b == True);
    a = 1;
    assert!(a.kind == longs);
    assert!(a.value.longy = 1);
    a0 = 10l;
    assert!(a0.kind == longs);
    assert!(a0.value.longy = 1);
    a1 = 1.1;
//     assert!(_eq(a1.kind, reals);
    assert!(a1.kind == reals);
    assert!(a1.value.real = 1.1);
    a2 = 1.2f;
    assert!(a2.kind == reals);
    assert!(a2.value.real = 1.2f);
//     Node as = 'a';
    assert!(as.kind == strings);
    assert!(as.value.string == 'a');
}
// }

#[test]
fn testUnits() {
    assert_is("1 m + 1km", Node(1001).setType(types["m"]));
}

#[test]
// fn testPaint() {
//     // #[cfg(feature = "SDL")]{
//         init_graphics();
//         while (1)
// //         paint(-1);
// //     }
// // }

// #[test]
fn testPaintWasm() {
    #[cfg(feature = "GRAFIX")]{
        //	struct timeval stop, start;
        //	gettimeofday(&start, NULL);
        // todo: let compiler compute constant expressions like 1024*65536/4
        //    	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
        //out of bounds memory access if only one Memory page!
//         is!("i=0;k='hi';while(i<16777216){i++;k#i=65};paint()", 0) // still slow, but < 1s
        // wow, SLOWER in wasm-micro-runtime HOW!?
        //	exit(0);

        //(√((x-c)^2+(y-c)^2)<r?0:255);
        //(x-c)^2+(y-c)^2
        is!("h=100;r=10;i=100;c=99;r=99;x=i%w;y=i/h;k=‖(x-c)^2+(y-c)^2‖<r", 1);
        ////char *wasm_paint_routine = "surface=(1,2);i=0;while(i<1000000){i++;surface#i=i*(10-√i);};paint";
//         char * wasm_paint_routine = "w=1920;c=500;r=100;surface=(1,2);i=0;"
//         "while(i<1000000){"
//         "i++;x=i%w;y=i/w;surface#i=(x-c)^2+(y-c)^2"
        "};paint";
        //((x-c)^2+(y-c)^2 < r^2)?0x44aa88:0xffeedd
        //char *wasm_paint_routine = "surface=(1,2);i=0;while(i<1000000){i++;surface#i=i;};paint";
        //is!(wasm_paint_routine, 0);
        //	char *wasm_paint_routine = "maxi=3840*2160/4/2;init_graphics();surface=(1,2,3);i=0;while(i<maxi){i++;surface#i=i*(10-√i);};";
        eval(wasm_paint_routine);
        //	paint(0);
        //	gettimeofday(&stop, NULL);
        //	printf("took %lu µs\n", (stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec);
        //	printf("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);
        //	exit(0);
        //char *wasm_paint_routine = "init_graphics(); while(1){paint()}";// SDL bugs a bit
        //        while (1)paint(0);// help a little
    }
}

#[test]
fn testNodesInWasm() {
    is!("{b:c}", parse("{b:c}"));
    is!("a{b:c}", parse("a{b:c}"));
}

#[test]
fn testSubGroupingIndent() {
    result = parse("x{\ta\n\tb,c,\n\td;\n\te");
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test]
fn testSubGrouping() {
    // todo dangling ',' should make '\n' not close
    //	result=parse("a\nb,c,\nd;e");
//     result = parse("a\n"
//                    "b,c,\n"
//                    "d;\n"
//                    "e");
    eq!(result.length, 3); // b,c,d should be grouped as one because of dangling comma
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test]
fn testSubGroupingFlatten() {
    // ok [a (b,c) d] should be flattened to a (b,c) d
    result = parse("[a\nb,c\nd]");
    //	result=parse("a\nb,c\nd");// still wrapped!
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "d");
}

#[test]
fn testBUG() {
    // move to tests() once done!
    //        testRecentRandomBugs();
}

#[test]
fn testBadInWasm() {
    // break immediately
    testStringConcatWasm();
    is!("square(3.0)", 9.); // todo groupFunctionCallPolymorphic
    is!("global x=1+π", 1 + pi); // int 4 ƒ
    testWasmMutableGlobal(); // todo!
    is!("i=0;w=800;h=800;pixel=(1 2 3);while(i++ < w*h){pixel[i]=i%2 };i ", 800 * 800);
    //local pixel in context wasp_main already known  with type long, ignoring new type group<byte>
    is!("grows:=it*2; grows 3*42 > grows 2*3", 1);
    // is there a situation where a COMPARISON is ambivalent?
    // sleep ( time > 8pm ) and shower ≠ sleep time > ( 8pm and true);
    testNodeDataBinaryReconstruction(); // todo!  y:{x:2 z:3}
    testSmartReturnHarder(); // y:{x:2 z:3} can't work yet(?);
    is!("add1 x:=$0+1;add1 3", (int64) 4); // $0 specially parsed now
    is!("print 3", 3); // todo dispatch!
    is!("if 4>1 then 2 else 3", 2);

    // bad only SOMETIMES / after a while!
    is!("puts('ok');(1 4 3)#2", 4); // EXPECT 4 GOT 1n
    is!("'αβγδε'#3", U'γ'); // TODO! sometimes works!?
    is!("3 + √9", (int64) 6); // why !?!
    is!("id 3*42> id 2*3", 1);
    testSquares(); // ⚠️

    // often breaks LATER! usually some map[key] where key missing!
    // WHY do thesAe tests break in particular, sometimes?
    testMergeOwn();
    testEmitter(); // huh!?!
}
#[test]
fn assurances() {
    #[cfg(feature = "WASM")]{
        //	assert!(sizeof(Type32) == 4) // todo:
//         # else
        //    assert!(sizeof(Type32) == 4) // otherwise all header structs fall apart
        assert!(sizeof(Type64) == 8) // otherwise all header structs fall apart
        //    assert!(sizeof(Type) == 8) // otherwise all header structs fall apart
    }
    //    assert!(sizeof(void*)==4) // otherwise all header structs fall apart TODO adjust in 64bit wasm / NORMAL arm64 !!
    assert!(sizeof(int64) == 8);
}

// todo: merge with testAllWasm, move ALL of these to test_wasm.rs
#[test]
fn testAllEmit() {
    // WASM emit tests under the hood:
    // is!("√3^2", 3); // basics
    //	is!("42", 42);// basics
    //    exit(42);
    //    is!("√ π ²", pi);
    //    is!("√π²", pi);
    testFunctionDeclaration();
    testForLoops();
    testHex();
    testEmitBasics();
    testMinusMinus();
    testSinus();

    // newly resolved:
    testModulo(); // fixed by adding modulo_float!
    testRootLists();
    testIndexOffset();
    testEnumConversion();
    testDeepColon2();
    testPattern();

    testSmartReturn();
    testWasmString(); // with length as header
    //    return;
    testArrayIndices();
    testMultiValue();
    testLogic();

    testLogic01();
    testLogicOperators();
    testRoots();
    testRootFloat();
    testMathExtra(); // "one plus two times three"==7 used to work?
    testTruthiness();
    testLogicPrecedence();
    testRootLists();
    testArrayIndices();
    testSmartReturn();
    testMultiValue();
    //    testSinus();

    testAllAngle();
    testRecentRandomBugs();
    testEqualities();

    skip!(
 // todo!
        testBadInWasm();
    );
    //    part of
    //    testAllWasm() :
    //    testRoundFloorCeiling();

    #[cfg(feature = "APPLE")]{
        testAllSamples();
    }
    assert!(NIL.value.longy == 0); // should never be modified
    // print("ALL TESTS PASSED");
}
#[test]
fn testHostIntegration() {
    #[cfg(feature = "WASMTIME")]{
//         or
//         WASMEDGE
        return;
    }
    #[cfg(not(feature = "WASM"))]{
        testHostDownload(); // no is!
    }
    test_getElementById();
    testDom();
    testDomProperty();
    testInnerHtml();
    testJS();
    testFetch();
    skip!(

        testCanvas(); // attribute setter missing value breaks browser
    );
}
#[test]
// fn print(Module &m) {
//     print("Module");
//     print("name:");
    // print(m.name);
    // print("code:");
    // print(m.code);
    // print("import_data:");
    // print(m.import_data);
    // print("export_data:");
    // print(m.export_data);
    // print("functype_data:");
    // print(m.functype_data);
    // print("code_data:");
    // print(m.code_data);
    // print("globals_data:");
    // print(m.globals_data);
    // print("memory_data:");
    // print(m.memory_data);
    // print("table_data:");
    // print(m.table_data);
    // print("name_data:");
    // print(m.name_data);
    // print("data_segments:");
    // print(m.data_segments);
    // print("linking_section:");
    // print(m.linking_section);
    // print("relocate_section:");
    // print(m.relocate_section);
    // print("funcToTypeMap:");
    // print(m.funcToTypeMap);
    // print("custom_sections:");
    // print(m.custom_sections);
    // print("type_count:");
    // print(m.type_count);
    // print("import_count:");
    // print(m.import_count);
    // print("total_func_count:");
    // print(m.total_func_count);
    // print("table_count:");
    // print(m.table_count);
    // print("memory_count:");
    // print(m.memory_count);
    // print("export_count:");
    // print(m.export_count);
    // print("global_count:");
    // print(m.global_count);
    // print("code_count:");
    // print(m.code_count);
    // print("data_segments_count:");
    // print(m.data_segments_count);
    // print("start_index:");
    // print(m.start_index);
    // print("globals); List<Global> WHY NOT??");
    // print("m.functions.size()");
    // print(m.functions.size());
    // print("m.funcTypes.size()");
    // print(m.funcTypes.size());
    assert!(m.funcTypes.size() == m.type_count);

    // print("m.signatures.size()");
    // print(m.signatures.size());
    // print("m.export_names");
    // print(m.export_names); // none!?
    // print("import_names:");
    // print(m.import_names);
// }

#[test]
fn test_const_String_comparison_bug() {
    // fixed in 8268c182 String == chars ≠> chars == chars  no more implicit cast
//     const String
    &library_name = "raylib";
    assert!(library_name == "raylib");
}
#[test]
fn todo_done() { // moved from todo();
    // GOOD! move to tests() once they work again but a#[test] fn redundant test executions
    assert_is("2+1/2", 2.5);
    is!("a = [1, 2, 3]; a[2]", 3);
    // #[cfg(not(feature = "WASMTIME"))]{ and not LINUX // todo why
    // is!("n=3;2ⁿ", 8);
    #[cfg(feature = "NATIVE_FFI")]{
        test_ffi_sdl();
        // SDL tests temporarily disabled - debugging type mismatches
        // test_ffi_sdl_red_square_demo();
    }
    test_list_lambdas();

    testMapOfStrings();
    testMapOfStringValues();
    testMaps();
    testNotNegation();
    testWhileNot();
    testAutoType();
    testTypeSynonyms();
    testFetch();
    testWGSL();
    testFunctionDeclaration();
    testReturnTypes();
    testRecentRandomBugs();
    // exit(0); // todo: remove this once all tests are passing
    testStringInterpolation();
    // we already have a working syntax so this has low priority?
    // testFunctionDeclaration();
    // testFibonacci(); // much TODO!
    // testSinus();

    testWaspRuntimeModule();

    testPing();
    test_while_true_forever();
    testStructWast();
    test_wasm_node_struct();
    #[cfg(feature = "NATIVE_FFI")]{
        test_ffi_all();
    }
    testMergeRuntime();
    testFunctionArgumentCast();
    testWrong0Termination();
    is!("fun addier(x, y){ x + y }; addier(3,4)", 7);
    testErrors(); // error: failed to call function   wasm trap: integer divide by zero

    read_wasm("lib/stdio.wasm");
    //    testStruct();

    testWit();
    testColonImmediateBinding();
    testWasmRuntimeExtension();
    testUpperLowerCase();
    //    exit(1);
    testDataMode();
    testParams();
    is!("\"Hello \" + \"🌍\" + (2000+25)","Hello 🌍2025");

    // test_const_String_comparison_bug(); // fixed in 8268c182
}
// todo: ^^ move back into tests() once they work again
#[test]
fn todos() {
    skip!(
 // unskip to test!!
        test_wasm_structs();

        testKitchensink();
        testNodeEmit();
        testLengthOperator();
        testEmitCast();
        is!("2,4 == 2,4", 1);
        is!("(2,4) == (2,4)", 1); // todo: array creation/ comparison
        is!("‖-2^2 - -2^3‖", 4); // Too many args for operator ‖,   a - b not grouped!
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        testWasmTypedGlobals();
    );

    #[cfg(not(feature = "TRACE"))]{
        println("parseLong fails in trace mode WHY?");
        is!("parseLong('123000')+parseLong('456')", 123456);
    }

    test_sinus_wasp_import();
    testSinus(); // todo FRAGILE fails before!
    //    testSinus2();
    //    run("circle.wasp");
    // while without body
    //    Missing condition for while statement
    skip!(

        is!("i=0;while(i++ <10001);i", 10000) // parsed wrongly! while(  <( ++ i 10001) i);
    );

    is!("use math;⅓ ≈ .3333333 ", 1);
    is!("precision = 3 digits; ⅓ ≈ .333 ", 1);
    assert_throws("i*=3"); // well:
    is!("i*=3", (int64) 0);
    // todo: ERRORS when cogs don't match! e.g. remove ¬ from prefixOperators!
    assert_throws("ceiling 3.7");
    // default bug!
    //    	subtract(other complex) := re -= other.re; im -= other.im
    // := is terminated by \n, not by ;!
    assert_throws("xyz 3.7"); // todo SHOULD THROW unknown symbol!
    is!("if(0):{3}", false); // 0:3 messy node
    eq!(Node("1", 0) + Node("2"s),
                  Node("1", "2", 0)); // 1+2 => 1:2  stupid border case because 1 not group (1);
//     assert_is((char *) "{a b c}#2", "b"); // ok, but not for patterns:
//     assert_is((char *) "[a b c]#2", "b"); // patterns
    is!("abs(0)", 0);
    assert_is("i=3;i--", 2); // todo bring variables to interpreter
    assert_is("i=3.7;.3+i", 4); // todo bring variables to interpreter
    assert_is("i=3;i*-1", -3); // todo bring variables to interpreter
    assert_is("one plus two times three", 7);
    //	print("OK %s %d"s % ("WASM",1));// only 1 handed over
    //    print(" OK %d %d"s % (2, 1));// error: expression result unused [-Werror,-Wunused-value] OK
    is!("use wasp;use lowerCaseUTF;a='ÂÊÎÔÛ';lowerCaseUTF(a);a", "âêîôû");
    test2Def();
    testConstructorCast();
    is!("html{bold{'Hello'}}", "Hello"); // in wasmtime
}

#[test]
fn test_todos() {
    todos();
    // move to test_done() once done!
}
#[test]
fn todo_done(); // may be below

#[test]
fn tests() {
    todo_done();
    assurances();
    #[cfg(not(feature = "WASM"))]{
        testNumbers();
        testPower();
        testEmitStringConcatenation();
        testExternReferenceXvalue();
        testExternString();
    }
    testCast();
    testFunctionDeclarationParse();
    testPower();
    testRandomParse();
    eq!(String("a1b1c1d").lastIndexOf("1"), 5);
    testUnicode_UTF16_UTF32();
    testReplaceAll();
    testExceptions();
    testString();
    testNodeBasics();
    testIterate();
    testLists();
    testEval();
    testParent();
    testNoBlock(); // fixed
    testSubGroupingFlatten();
    testNodeConversions();
    testUpperLowerCase();
    testListGrow();
    testGroupCascade();
    testNewlineLists();
    testStackedLambdas();

    testParamizedKeys();
    testForEach();
    testEmpty();
    testDiv();
    testRoot();
    testSerialize();
    skip!(

        testPrimitiveTypes();
    );
    //	test_sin();
    testIndentAsBlock();
    testDeepCopyDebugBugBug2(); // SUBTLE: BUGS OUT ONLY ON SECOND TRY!!!
    testDeepCopyDebugBugBug();
    testComments();
    testEmptyLineGrouping();
    testSwitch();
    testAsserts();
    testFloatReturnThroughMain();
    testSuperfluousIndentation();
    testString();
    testEmptyLineGrouping();
    testColonLists();
    testGraphParams();
    testNodeName();
    testStringConcatenation();
    testStringReferenceReuse();
    testConcatenation();
    testMarkSimple();
    testMarkMulti();
    testMarkMulti2();
    testDedent2();
    testDedent();
    testGroupCascade0();
    testGraphQlQuery();
    // print(testNodiscard());
    testCpp();
    testNilValues();
    testMapsAsLists();
    testMaps();
    testLists();
    testDeepLists();
    testGraphParams();
    testAddField();
    testOverwrite();
    testDidYouMeanAlias();
    testNetBase();
    testForEach();
    // testLengthOperator();
    testLogicEmptySet();
    testDeepCopyDebugBugBug();
    testDeepCopyDebugBugBug2();
    //    testMarkSimpleAssign();
    testSort();
    testSort1();
    testSort2();
    testReplace();
    testRemove();
    testRemove2();
    testGraphQlQueryBug();
    testGraphQlQuery(); // fails sometimes => bad pointer!?
    testGraphQlQuery2();
    testUTF(); // fails sometimes => bad pointer!?
    testUnicode_UTF16_UTF32();
    testConcatenationBorderCases();
    testNewlineLists();
    testIndex();
    testGroupCascade();
    testParams();
    testSignificantWhitespace();
    testBUG();
    testFlags();
    //    testFlags2();
    //    testFlagSafety();
    #[cfg(feature = "WASM")]{
        warn("Currently NOT PASSING via wasmtime -D debug-info=y --dir . wasp.wasm test");
    }
    testMarkAsMap();
    testFunctionDeclarationParse();
    testMarkSimple();
    #[cfg(not(feature = "WASM"))]{
        testMarkMultiDeep();
    }
    #[cfg(feature = "WASM")]{
        warn("Normal tests ALL PASSING in wasm!");
        warn("WASM emit tests CURRENTLY __ALL__ SKIPPED or asynchroneous!");
        return;
//         # else
        testAllEmit();
    }
    // todo: split in test_wasp test_angle test_emit.rs
}

#[test]
fn test_new() {
    //    testInclude();
    //    testMatrixOrder();
    #[cfg(feature = "WASMEDGE")]{
        test_wasmedge_gc();
    }
    // test_list_growth();
    testFlags();
    testTypes();
    testPolymorphism();
}

// 2021-10 : 40 sec for Wasm3
// 2022-05 : 8 sec in Webapp / wasmtime with wasp.wasm built via wasm-runtime
// 2022-12-03 : 2 sec WITHOUT runtime_emit, wasmtime 4.0 X86 on M1
// 2022-12-03 : 10 sec WITH runtime_emit, wasmtime 4.0 X86 on M1
// 2022-12-28 : 3 sec WITH runtime_emit, wasmedge on M1 WOW ALL TESTS PASSING
// 2025-03-23 : <5 sec WITH runtime_emit, WASMTIME/WAMR/WASMEDGE on M1, 45 sec in Chrome (because print?);
// ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
// 2025-12-23 : 10 sec WITH runtime_emit, wasmtime 4.0 on M2
#[test]
fn testCurrent() {
    // print("testCurrent DEACTIVATED");
    // return;
    // print("💡 starting current tests 💡");
    // testTruthiness();
    #[cfg(feature = "WASM")]{
        print("⚠️ make sure to put all is! into testRun() ");
    }
    // eval("./samples/raylib_mouse_circle.wasp");
    // testTruthyAnd();
    is!("fun addier(a,b){b+a};addier(42,1)", 43);
    testPing();
    testIfCallZero();
    testWhileNotCall();
    testReturnTypes();
    // test_ffi_all();
    // test_ffi_raylib_simple_use_import();

    // exit(   0);
    skip!(

        testDeepColon(); // wit definition vs wasp declaration clash!
        todos(); // WIP and BUGs
    );
    todo_done();
    // sleep(10);
    // exit(0);
    // test_dynlib_import_emit();
    #[cfg(feature = "WASMEDGE")]{
        testStruct(); // no wasmtime yet
    }

    skip!(
 // TODO!
        test_new();
        is!("x=3;y=4;c=1;r=5;(‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);

        testMergeGlobal(); //
        testRenameWasmFunction();
        testExp(); // todo!
        testKebabCase(); // needed later …
        testStruct();
        todos();
        assert_is("(1 4 3)#2", 4); //
        assert_throws("0/0"); // now NaN OK
        testPolymorphism2();
        testPolymorphism3();
        testVectorShim(); // use GPU even before wasm vector extension is available
        testModifiers();
        testLengthOperator();
        testNamedDataSections();
        testHostDownload();
        testHostIntegration();
        testJS();
        testHtmlWasp();
    );
    // testListGrowth<const int&>();// pointer to a reference error

    // todo print as general dispatch depending on smarttype
    //    is!("for i in 1 to 5 : {print i};i", 6);

    testSourceMap();
    //	testDwarf();

    // ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
    tests(); // make sure all still ok after messing with memory

    #[cfg(not(feature = "WASM"))]{
        // ⚠️ in WASM these tests are called via async trick
        testAngle(); // fails in WASM why?
        testAssertRun(); // separate because they take longer (≈10 sec as of 2022.12);
        testAllWasm();
        //    todos();// those not passing yet (skip);
    }
    // print(tests_executed);
    // print("CURRENT TESTS PASSED");
}

// }
// // // // valgrind --track-origins=yes ./wasp
