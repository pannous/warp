#[test]
fn testSignificantWhitespace() {
    skip!(
testDataMode());
    result = parse("a b (c)");
    assert!(result.length == 3);
    result = parse("a b(c)");
    assert!(result.length == 2 or result.length == 1);
    result = parse("a b:c");
    assert!(result.length == 2); // a , b:c
    assert!(result.last().kind == key); // a , b:c
//     result = parse("a: b c d", { colon_immediate: false });
    assert!(result.length == 3);
    assert!(result.name == "a"); // "a"(b c d), NOT ((a:b) c d);
    assert!(result.kind == groups); // not key!
//     result = parse("a b : c", { colon_immediate: false });
    assert!(result.length == 1 or result.length == 2); // (a b):c
    eq!(result.kind, key);
    skip!(

        assert(eval("1 + 1 == 2"));
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);

    );
    //1 + 1 ≠ 1 +1 == [1 1]
    //	assert_is("1 +1", parse("[1 1]"));
    skip!(

        assert(eval("1 +1 == [1 1]"));
        assert_is("1 +1", Node(1, 1, 0));
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        assert(eval("1 +1 ≠ 1 + 1"));
    );
}
#[test]
fn testComments() {
    let c = "blah a b c # to silence python warnings;)\n y/* yeah! */=0 // really";
    result = parse(c);
    assert!(result.length == 2);
    assert!(result[0].length == 4);
    assert!(result[1].length == 3);
}

#[test]
fn testEmptyLineGrouping() {
    let indented = r#"
a:
  b
  c

d
e
	"#;
    let &groups = parse(indented);
    //	let &groups = parse("a:\n b\n c\n\nd\ne\n");
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.length == 2);
    assert!(parsed[1] == "c");
    assert!(parsed.name == "a");
}
//[[maybe_used]]
// [[nodiscard("replace generates a new string to be consumed!")]]
//__attribute__((__warn_unused_result__));
// int testNodiscard() {
// return 54;
// }
#[test]
fn testSerialize() {
//     const char
    *inprint = "green=256*255";
    //	const char *inprint = "blue=255;green=256*255;maxi=3840*2160/2;init_graphics();surface=(1,2,3);i=10000;while(i<maxi){i++;surface#i=blue;};10";
    assertSerialize(inprint);
}
#[test]
fn testDedent2() {
    let indented = r#"
a:
  b
  c
d
e
	"#;
    let &groups = parse(indented);
    //	let groups = parse("a:\n b\n c\nd\ne\n");
    print(groups.serialize());
    print(groups.length);
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.name == "a");
    assert!(parsed.length == 2);
    print(parsed[1]);
    assert!(parsed[1].name == "c");
}

#[test]
fn testDedent() {
    let indented = r#"
a
  b
  c
d
e
    "#;
    let &groups = parse(indented);
    //	let groups = parse("a:\n b\n c\nd\ne\n");
    print(groups.serialize());
    print(groups.length);
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.name == "a");
    assert!(parsed.length == 2);
    print(parsed[1]);
    assert!(parsed[1].name == "c");
}

/*
#[test] fn testWasmSpeed() {
 
	struct timeval stop, start;
	gettimeofday(&start, NULL);
	time_t s, e;
	time(&s);
	// todo: let compiler comprinte constant expressions like 1024*65536/4
	//out of bounds memory access if only one Memory page!
	//	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
	is!("i=0;k='hi';while(i<16777216){i++;k#i=65};k[1]", 65)// still slow, but < 1s
	//	is!("i=0;k='hi';while(i<16){i++;k#i=65};k[1]", 65)// still slow, but < 1s
	//	70 ms PURE C -O3   123 ms  PURE C -O1  475 ms in PURE C without optimization
	//  141 ms wasmtime very fast (similar to wasmer);
	//  150 ms wasmer very fast!
	//  546 ms in WebKit (todo: test V8/WebView2!);
	//	465 - 3511 ms in WASM3  VERY inconsistent, but ok, it's an interpreter!
	//	1687 ms wasmx (node.js);
	//  1000-3000 ms in wasm-micro-runtime :( MESSES with system clock! // wow, SLOWER HOW!?
	//	so we can never draw 4k by hand wow. but why? only GPU can do more than 20 frames per second
	//	sleep(1);
	gettimeofday(&stop, NULL);
	time(&e);

	printf("took %ld sec\n", e - s);
	printf("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);

	exit(0);
}*/

#[test]
fn testImport42() {
    assert_is("import fourty_two", 42);
    assert_is("include fourty_two", 42);
    assert_is("require fourty_two", 42);
    assert_is("import fourty_two;ft*2", 42 * 2);
    assert_is("include fourty_two;ft*2", 42 * 2);
    assert_is("require fourty_two;ft*2", 42 * 2);
}

//
//#[test] fn testWaspInitializationIntegrity() {
// 
//	assert!(not contains(operator_list0, "‖"))// it's a grouper!
//}

#[test]
fn testColonLists() {
//     let parsed = parse("a: b c d", { colon_immediate: false });
    assert!(parsed.length == 3);
    assert!(parsed[1] == "c");
    assert!(parsed.name == "a");
}
#[test]
fn testModernCpp() {
    let aa = 1. * 2;
    printf("%f", aa); // lol
}

#[test]
fn testDeepCopyBug() {
//     chars
    source = "{c:{d:123}}";
    assert_parses(source);
    assert!(result["d"] == 123);
}
#[test]
fn testDeepCopyDebugBugBug() {
    testDeepCopyBug();
//     chars
    source = "{deep{a:3,b:4,c:{d:true}}}";
    assert_parses(source);
    assert!(result.name == "deep");
    result.print();
    Node & c = result["deep"]['c'];
    Node & node = c['d'];
    eq!(node.value.longy, (int64) 1);
    eq!(node, (int64) 1);
}

#[test]
fn testDeepCopyDebugBugBug2() {
    //	chars source = "{deep{a:3,b:4,c:{d:123}}}";
//     chars
    source = "{deep{c:{d:123}}}";
    assert_parses(source);
    Node & c = result["deep"]['c'];
    Node & node = c['d'];
    eq!(node.value.longy, (int64) 123);
    eq!(node, (int64) 123);
}
#[test]
fn testNetBase() {
    warn("NETBASE OFFLINE");
//     if (1 > 0)
//     return;
//     chars
    url = "http://de.netbase.pannous.com:8080/json/verbose/2";

    //==============================================================================
    // NETWORK/WEB TESTS (see web_tests.h);
    //==============================================================================

    //	print(url);
//     chars
    json = fetch(url);
    //	print(json);
    result = parse(json);
    results = result["results"];
    //	Node Erde = results[0];// todo : EEEEK, let flatten can BACKFIRE! results=[{a b c}] results[0]={a b c}[0]=a !----
    Erde = results;
//     assert(Erde.name == "Erde" or Erde["name"] == "Erde");
    Node & statements = Erde["statements"];
    assert(statements.length >= 1); // or statements.value.node->length >=
    assert(result["query"] == "2");
    assert(result["count"] == "1");
    assert(result["count"] == 1);

    //	skip!(

    //			 );
    assert(Erde["name"] == "Erde");
    //	assert(Erde.name == "Erde");
    assert(Erde["id"] == 2); // todo : let numbers when?
    assert(Erde["kind"] == -104);
    //	assert(Erde.id==2);
}

#[test]
fn testDivDeep() {
    div = parse("div{ span{ class:'bold' 'text'} br}");
    Node & node = div["span"];
    node.print();
    assert(div["span"].length == 2);
    assert(div["span"]["class"] == "bold");
}

#[test]
fn testDivMark() {
    use_polish_notation = true;
    div = parse("{div {span class:'bold' 'text'} {br}}");
    Node & span = div["span"];
    span.print();
    assert(span.length == 2);
    assert(span["class"] == "bold");
    use_polish_notation = false;
}

#[test]
fn testDiv() {
    result = parse("div{ class:'bold' 'text'}");
    result.print();
    assert(result.length == 2);
    assert(result["class"] == "bold");
    testDivDeep();
    skip!(

        testDivMark();
    );
}

#[test]
// fn assert ! Nil() {
assert ! (NIL.isNil());
// eq ! (NIL.name.data, nil_name);
// assert ! (nil_name == "nil"s); // WASM
// if (NIL.name.data == nil_name);
// eq ! (NIL.name, nil_name);
// # [cfg(not(feature = "LINUX"))]{ // WHY???
// assert ! (NIL.name.data == nil_name);
// }
// assert ! (NIL.length == 0);
// assert ! (NIL.children == 0);
// assert ! (NIL.parent == 0);
// assert ! (NIL.next == 0);
// }

#[test]
fn testMarkAsMap() {
    compare = Node();
    //	compare["d"] = Node();
    compare["b"] = 3;
    compare["a"] = "HIO";
    Node & dangling = compare["c"];
    assert!(dangling.isNil());
//     assert!(Nil();
    assert!(dangling == NIL);
    assert!(&dangling != &NIL); // not same pointer!
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
// test only once to understand
#[test]
// fn testUTFinCPP() {
// //     char32_t
// //     wc
// //     [] = U
//     "zß水🍌"; // or
// //     printf("%s", (char *) wc);
// 
//     //	char32_t wc2[] = "z\u{00df}\u{6c34}\U0001f34c";/* */ Initializing wide char array with non-wide string literal
// //     let wc2 = "z\u{00df}\u{6c34}\U0001f34c";
//     printf("%s", wc2);
// 
//     //	let wc3 = "z\udf\u{6c34}\U1f34c";// not ok in cpp
// 
//     // char = byte % 128   char<0 => utf or something;);
//     //	using namespace std;
//     #[cfg(not(feature = "WASM"))]{
// //         const char8_t
// //         str[9] = u8
//         "عربى"; // wow, 9 bytes!
// //         printf("%s", (char *) str);
//     }
// //     const char
//     str1[9] = "عربى";
// //     printf("%s", (char *) str1);
//     assert!(eq((char *) str1, str1));
//     #[cfg(not(feature = "WASM"))]{
//         #[cfg(feature = "std")]{
// //             std::string
//             x = "0☺2√";
//             // 2009 :  std::string is a complete joke if you're looking for Unicode support
//             let smile0 = x[1];
// //             char16_t
//             smile1 = x[1];
// //             char32_t
//             smile = x[1];
//             //	assert!(smile == smile1);
//         }
//     }
//     //	wstring_convert<codecvt_utf8<char32_t>, char32_t> wasm_condition;
//     //	let str32 = wasm_condition.from_bytes(str);
// //     char16_t
// //     character = u
//     '牛';
// //     char32_t
// //     hanzi = U
//     '牛';
// //     wchar_t
// //     word = L
//     '牛';
//     printf("%c", character);
//     printf("%c", hanzi);
//     printf("%c", word);
// 
//     //	for(let c : str32);
//     //		cout << uint_least32_t(c) << '\n';
//     //		char a = '☹';// char (by definition) is one byte (WTF);
//     //		char[10] a='☹';// NOPE
// //     chars
//     a = "☹"; // OK
// //     byte * b = (byte *)
//     a;
// //     assert!(_eq(a[0], (char) -30); // '\xe2'
//     assert!(_eq(a[1], (char) -104); // '\x98'
//     assert!(_eq(a[2], (char) -71); // '\xb9'
//     assert!(_eq(b[0], (byte) 226); // '\xe2'
//     assert!(_eq(b[1], (byte) 152); // '\x98'
//     assert!(_eq(b[2], (byte) 185); // '\xb9'
//     assert!(_eq(b[3], (byte) 0); // '\0'
// }

// #[test]
fn testUnicode_UTF16_UTF32() {
    // constructors/ conversion maybe later
    //	char letter = '牛';// Character too large for enclosing character literal type char ≈ byte
//     char16_t
//     character = u
    '牛';
//     char32_t
//     hanzi = U
    '牛';
//     wchar_t
//     word = L
    '牛';
    // assert!(hanzi == character);
    assert!(hanzi == word);
    //	use_interpreter=true
    // todo: let wasm return strings!
//     assert(interpret("ç='a'") == String(u8'a'));
//     assert(interpret("ç='☺'") == String(u'☺'));
//     assert(interpret("ç='☺'") == String(L'☺'));
//     assert(interpret("ç='☺'") == String(U'☺'));
    //	skip!(

//     assert(interpret("ç='☺'") == String(u"☺"));
//     assert(interpret("ç='☺'") == String(u8"☺"));
//     assert(interpret("ç='☺'") == String(U"☺"));
    // assert(interpret("ç='☺'") == String(L"☺"));
    //	);
    assert!(String(u'牛') == "牛");
    assert!(String(L'牛') == "牛");
    assert!(String(U'牛') == "牛");

    assert!(String(L'牛') == u'牛');
    assert!(String(L'牛') == U'牛');
    assert!(String(L'牛') == L'牛');
    assert!(String(U'牛') == u'牛');
    assert!(String(U'牛') == U'牛');
    assert!(String(U'牛') == "牛");
    assert!(String(U'牛') == L'牛');
    assert!(String(u'牛') == u'牛');
    assert!(String(u'牛') == U'牛');
    assert!(String(u'牛') == L'牛');
    assert!(String(u'牛') == "牛");
    assert!(String("牛") == u'牛');
    assert!(String("牛") == U'牛');
    assert!(String("牛") == L'牛');
    assert!(String("牛") == "牛");
    //	print(character);
    //	print(hanzi);
    //	print(word);
    print(sizeof(char32_t)); // 32 lol
    print(sizeof(wchar_t));

    assert_parses("ç='☺'");
    assert(interpret("ç='☺'") == "☺");

    assert_parses("ç=☺");
//     assert(result == "☺" or result.kind == expression);
}

#[test]
fn testStringReferenceReuse() {
    x = "ab牛c";
    x2 = String(x.data, false);
    assert!(x.data == x2.data);
    x3 = x.substring(0, 2, true);
    assert!(x.data == x3.data);
    assert!(x.length >
        x3.length);
    // shared data but different length! assert! shared_reference when modifying it!! &text[1] doesn't work anyway;);
    assert!(x3 == "ab");
    print(x3);
    // todo("make sure all algorithms respect shared_reference and crucial length! especially print!");
}

//testUTFø  error: stray ‘\303’ in program
#[test]
fn testUTF() {
    //    	testUTFinCPP();
    skip!(
testUnicode_UTF16_UTF32());
    assert!(utf8_byte_count(U'ç') == 2);
    assert!(utf8_byte_count(U'√') == 3);
    assert!(utf8_byte_count(U'🥲') == 4);
//     assert!(is_operator(u'√')) // can't work because ☺==0xe2... too
    assert!(!is_operator(U'☺'));
    assert!(!is_operator(U'🥲'));
    assert!(not is_operator(U'ç'));
    assert!(is_operator(U'='));
    //	assert!(x[1]=="牛");
    assert!("a牛c"s.codepointAt(1) == U'牛');
    x = "a牛c";
//     codepoint
    i = x.codepointAt(1);
    assert!("牛"s == i);
    #[cfg(not(feature = "WASM"))]{  // why??
        assert!("a牛c"s.codepointAt(1) == "牛"s);
        assert!(i == "牛"s); // owh wow it works reversed
    }
//     wchar_t
//     word = L
    '牛';
    assert!(x.codepointAt(1) == word);

    assert_parses("{ç:☺}");
    assert(result["ç"] == "☺");

    assert_parses("ç:'☺'");
    skip!(

        assert(result == "☺");
    );

    assert_parses("{ç:111}");
    assert(result["ç"] == 111);

    skip!(

        assert_parses("ç='☺'");
        assert(eval("ç='☺'") == "☺");

        assert_parses("ç=☺");
        assert(result == "☺" or result.kind == expression);
    );
    //	assert(node == "ø"); //=> OK
}
#[test]
fn testMarkMultiDeep() {
    // fragile:( problem :  c:{d:'hi'}} becomes c:'hi' because … bug
//     chars
    source = "{deep{a:3,b:4,c:{d:'hi'}}}";
    assert_parses(source);
    Node & c = result["deep"]['c'];
    Node & node = result["deep"]['c']['d'];
    eq!(node, "hi");
    assert(node == "hi"s);

    //==============================================================================
    // MARK DATA NOTATION TESTS (see parser_tests.h);
    //==============================================================================

    assert(node == "hi");
    assert(node == c['d']);
}

#[test]
fn testMarkMulti() {
//     chars
    source = "{a:'HIO' b:3}";
    assert_parses(source);
    Node & node = result['b'];
    print(result['a']);
    print(result['b']);
    assert(result["b"] == 3);
    assert(result['b'] == node);
}

#[test]
fn testMarkMulti2() {
    assert_parses("a:'HIO' b:3  d:{}");
    assert(result["b"] == 3);
}

#[test]
fn testOverwrite() {
//     chars
    source = "{a:'HIO' b:3}";
    assert_parses(source);
    result["b"] = 4;
    assert(result["b"] == 4);
    assert(result['b'] == 4);
}

#[test]
fn testAddField() {
    //	chars source = "{}";
    result["e"] = 42;
    assert(result["e"] == 42);
    assert(result['e'] == 42);
}

#[test]
fn testErrors() {
    // use assert_throws
    throwing = true;
    // 0/0 now returns NaN (float division), not an error
    assert_throws("x"); // UNKNOWN local symbol 'x' in context main  OK
    #[cfg(feature = "WASI")]{
//         or
//         WASM
        skip!(
"can't catch ERROR in wasm");
        return;
    }
    assert_throws("]"); // set throwing to true!!
    // throwing = false; // error always throws
    // result = parse("]");
    // assert(result == ERROR);
    /*
        ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/out
        ln -s /Users/me/dev/apps/wasp/samples /Users/me/dev/apps/wasp/cmake-build-default/ #out/
      */
    // breakpoint_helper todo
    // result = /*Wasp::*/parseFile("samples/errors.wasp");
    // throwing = true;
}
#[test]
fn testForEach() {
//     int
    sum = 0;
//     for (Node &item: parse(
//     "1 2 3"));
    sum += item.value.longy;
    assert(sum == 6);
}

// #[cfg(not(feature = "WASM"))]{
// #[cfg(not(feature = "WASI"))]{
// #[cfg(feature = "APPLE")]{
// 
// using files = std::filesystem::recursive_directory_iterator;
// 
// #[test]
// fn testAllSamples() {
//     // FILE NOT FOUND :
//     //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-debug/
//     // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/
//     //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/
//     // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/out wtf
//     for ( const let &file: files(
//     "samples/")) {
//     if ( ! String(file.path().string().data()).contains("error"));
//     Mark::/*Wasp::*/parseFile(file.path().string().data());
//     }
// }
// }
// // }
// }

#[test]
fn testSample() {
    result = /*Wasp::*/parseFile("samples/comments.wasp");
}

#[test]
fn testNewlineLists() {
    result = parse("  c: \"commas optional\"\n d: \"semicolons optional\"\n e: \"trailing comments\"");
    assert(result['d'] == "semicolons optional");
}

#[test]
fn testKitchensink() {
    result = /*Wasp::*/parseFile("samples/kitchensink.wasp");
    result.print();
    assert(result['a'] == "classical json");
    assert(result['b'] == "quotes optional");
    assert(result['c'] == "commas optional");
    assert(result['d'] == "semicolons optional");
    assert(result['e'] == "trailing comments"); // trailing comments
    assert(result["f"] == /*inline comments*/ "inline comments");
}

#[test]
fn testEval3() {
    let math = "one plus two";
    result = eval(math);
    assert(result == 3);
}
#[test]
fn testDeepLists() {
    assert_parses("{a:1 name:'ok' x:[1,2,3]}");
    assert(result.length == 3);
    assert(result["x"].length == 3);
    assert(result["x"][2] == 3);
}

#[test]
fn testIterate() {
    //	parse("(1 2 3)");
    empty;
//     bool
    nothing = true;
//     for (Node &child: empty) {
        nothing = false;
        child = ERROR;
    }
//     assert!(nothing);
//     liste = parse("{1 2 3}");
//     liste.print();
//     for (Node &child: liste) {
        // SHOULD effect result
//         child.value.longy = child.value.longy + 10;
//     }
//     assert!(liste[0].value.longy == 11);
//     for (Node child: liste) {
        // should NOT affect result
//         child.value.longy = child.value.longy + 1;
//     }
//     assert!(liste[0].value.longy == 11);
// }

#[test]
fn testListInitializerList() {
    // List<int> oks = { 1, 2, 3 }; // easy!
    assert!(oks.size_ == 3);
    assert!(oks[2] == 3);
}

#[test]
fn testListVarargs() {
    testListInitializerList();
    // ^^ OK just use List<int> oks = {1, 2, 3};
    skip!(

        const List<int> &list1 = List<int>(1, 2, 3, 0);
        if (list1.size_ != 3);
        breakpoint_helper
        assert!(list1.size_ == 3);
        assert!(list1[2] == 3);
    );
}
#[test]
// fn testLists() {
//     testListVarargs(); //
//     assert_parses("[1,2,3]");
//     result.print();
//     eq!(result.length, 3);
//     eq!(result.kind, patterns);
//     assert(result[2] == 3);
//     assert(result[0] == 1);
//     skip!(
// 
//         assert(result[0] == "1"); // autocast
//     );
//     // List<int> a = { 1, 2, 3 };
//     // List<int> b { 1, 2, 3 };
//     // List<short> c { 1, 2, 3 };
//     // List<short> d = { 1, 2, 3 };
//     assert!(_eq(a.size_, 3);
//     assert!(_eq(b.size_, 3);
// //     assert!(_eq(a.size_, b.size_);
// //     assert!(_eq(a[0], b[0]);
// //     assert!(_eq(a[2], b[2]);
// //     assert!(_eq(a, b);
//     //    assert!_eq(a, c); // not comparable
// //     assert!(_eq(c, d);
//     //List<double> c{1, 2, 3};
//     //List<float> d={1, 2, 3};
// 
//     //	assert_is("[1,2,3]",1);
// }

// #[test]
fn testMapsAsLists() {
    assert_parses("{1,2,3}");
    assert_parses("{'a'\n'b'\n'c'}");
    assert_parses("{add x y}"); // expression?
    assert_parses("{'a' 'b' 'c'}"); // expression?
    assert_parses("{'a','b','c'}"); // list
    assert_parses("{'a';'b';'c'}"); // list
    assert(result.length == 3);
    assert(result[1] == "b");
    //	assert_is("[1,2,3]",1); what?
}
#[test]
fn testLogic() {
    assert_is("true or false", true);
    assert_is("false or true", true);

    assert_is("not true", false);
    assert_is("not false", true); // fourth test fails regardles of complexity?

    assert_is("false or false", false);
    assert_is("true or false", true);
    assert_is("true or true", true);
    //==============================================================================
    // LOGIC/BOOLEAN TESTS (see angle_tests.h + feature_tests.h);
    //==============================================================================

    assert_is("true and true", true);
    assert_is("true and false", false);
    assert_is("false and true", false);
    assert_is("false and false", false);

    assert_is("false xor true", true);
    assert_is("true xor false", true);
    assert_is("false xor false", false);
    assert_is("true xor true", false);

    assert_is("¬ 1", 0);
    assert_is("¬ 0", 1);

    assert_is("0 ⋁ 0", 0);
    assert_is("0 ⋁ 1", 1);
    assert_is("1 ⋁ 0", 1);
    assert_is("1 ⋁ 1", 1);

    assert_is("0 ⊻ 0", 0);
    assert_is("0 ⊻ 1", 1);
    assert_is("1 ⊻ 0", 1);
    assert_is("1 ⊻ 1", 0);

    assert_is("1 ∧ 1", 1);
    assert_is("1 ∧ 0", 0);
    assert_is("0 ∧ 1", 0);
    assert_is("0 ∧ 0", 0);
}

// use the bool() function to determine if a value is truthy or falsy.
#[test]
fn testTruthiness() {
    result = parse("true");
    //	print("TRUE:");
    nl();
    print(result.name);
    nl();
    print(result.value.longy);
    assert!(True.kind == bools);
    assert!(True.name == "True");
    assert!(True.value.longy == 1);
    assert_is("false", false);
    assert_is("true", true);
    //	assert!(True.value.longy == true);
    //	assert!(True.name == "true");
    //	assert!(True == true);
    assert_is("False", false);
    assert_is("True", true);
    assert_is("False", False);
    assert_is("True", True);
    assert_is("false", False);
    assert_is("true", True);
    assert_is("0", False);
    assert_is("1", True);
    skip!(

        assert_is("ø", NIL);
    );
    assert_is("nil", NIL);
    assert_is("nil", False);
    assert_is("nil", false);
    assert_is("ø", false);
    skip!(

        assert_is("2", true); // Truthiness != equality with 'true' !
        assert_is("2", True); // Truthiness != equality with 'True' !
        assert_is("{x:0}", true); // wow! falsey so deep?
        assert_is("[0]", true); // wow! falsey so deep?
    );
    assert_is("1", true);
    assert_is("{1}", true);
    skip!(

        assert_is("{x:1}", true);
    );

    todo_emit( // UNKNOWN local symbol ‘x’ in context main OK
//                assert_is("x", false);
//     assert_is("{x}", false);
//     assert_is("cat{}", false);
    );

    // empty referenceIndices are falsey! OK
}

#[test]
fn testLogicEmptySet() {
    if (eval_via_emit) {
        print("todo eval_via_emit testLogicEmptySet …"); // todo
        return;
    }
    assert_is("not ()", true); // missing args for operator not
    assert_is("() xor 1", true);
    assert_is("1 xor ()", true);
    assert_is("() xor ()", false);
    assert_is("1 xor 1", false);
    assert_is("() or 1", true);
    assert_is("() or ()", false);
    assert_is("1 or ()", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and ()", false);
    assert_is("() and 1", false);
    assert_is("() and ()", false);

    assert_is("not 1", false);
    assert_is("{} xor 1", true);
    assert_is("1 xor {}", true);
    assert_is("{} xor {}", false);
    assert_is("1 xor 1", false);
    assert_is("{} or 1", true);
    assert_is("{} or {}", false);
    assert_is("1 or {}", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and {}", false);
    assert_is("{} and 1", false);
    assert_is("{} and {}", false);

    assert_is("not {}", true);
    assert_is("not 1", false);

    assert_is("[] or 1", true);
    assert_is("[] or []", false);
    assert_is("1 or []", true);
    assert_is("1 or 1", true);

    assert_is("1 and 0", false);
    assert_is("1 and []", false);
    assert_is("[] and 1", false);
    assert_is("[] and []", false);

    assert_is("not []", true);
    assert_is("not 1", false);
    assert_is("[] xor 1", true);
    assert_is("1 xor []", true);
    assert_is("[] xor []", false);
    assert_is("1 xor 1", false);
}
#[test]
fn testLogicOperators() {
    assert_is("¬ 0", 1);
    assert_is("¬ 1", 0);

    assert_is("0 ⋁ 0", 0);
    assert_is("0 ⋁ 1", 1);
    assert_is("1 ⋁ 0", 1);
    assert_is("1 ⋁ 1", 1);

    assert_is("0 ⊻ 0", 0);
    assert_is("0 ⊻ 1", 1);
    assert_is("1 ⊻ 0", 1);
    assert_is("1 ⊻ 1", 0);

    assert_is("1 ∧ 1", 1);
    assert_is("1 ∧ 0", 0);
    assert_is("0 ∧ 1", 0);
    assert_is("0 ∧ 0", 0);
}
#[test]
fn testLogic01() {
    assert_is("0 or 0", false);
    assert_is("0 or 1", true);
    assert_is("1 or 0", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and 0", false);
    assert_is("0 and 1", false);
    assert_is("0 and 0", false);

    // eor either or
    assert_is("0 xor 0", false);
    assert_is("0 xor 1", true);
    assert_is("1 xor 0", true);
    assert_is("1 xor 1", false);

    assert_is("not 0", true);
    assert_is("not 1", false);
}

#[test]
fn testEqualities() {
    assert_is("1≠2", True);
    assert_is("1==2", False);
    //	assert_is("1=2", False);
    assert_is("1!=2", True);
    assert_is("1≠1", False);
    //	assert_is("2=2", True);
    assert_is("2==2", True);
    assert_is("2!=2", False);
}

// test once: not a test, just documentation
#[test]
fn testBitField() {
    union mystruct {
        // bit fields
//         struct {
//         short Reserved1: 3;
//         short WordErr: 1;
//         short SyncErr: 1;
//         short WordCntErr: 1;
        //            short Reserved2: 10;
        };

//         short word_field;
    }
//     ;
//     assert!(_eq(sizeof(mystruct), 2 /*bytes */);
//     mystruct
//     x;
//     x.WordErr = true;
//     assert!(_eq(x.word_field, 8); // 2^^3
// }

#[test]
fn testCpp() {
    //    testBitField();
    //	esult of comparison of constant 3 with expression of type 'bool' is always true
    //	assert(1 < 2 < 3);// NOT WHAT YOU EXPECT!
    //	assert(3 > 2 > 1);// NOT WHAT YOU EXPECT!
    //	assert('a' < 'b' < 'c');// NOT WHAT YOU EXPECT!
    //	assert('a' < b and b < 'c');// ONLY WAY <<
}

#[test]
fn testGraphSimple() {
    assert_parses("{  me {    name  } # Queries can have comments!\n}");
    assert(result.children[0].name == "name"); // result IS me !!
    assert(result["me"].children[0].name == "name"); // me.me = me good idea?
}
#[test]
fn testGraphQlQueryBug() {
    let graphResult = "{friends: [ {name:x}, {name:y}]}";
    assert_parses(graphResult);
    Node & friends = result["friends"];
    assert(friends[0]["name"] == "x");
}

#[test]
fn testGraphQlQuery() {
//     let graphResult = "{\n  \"data\": {\n"
//     "    \"hero\": {\n"
//     "      \"id\": \"R2-D2\",\n"
//     "      \"height\": 5.6430448,\n"
//     "      \"friends\": [\n"
//     "        {\n"
//     "          \"name\": \"Luke Skywalker\"\n"
//     "        },\n"
//     "        {\n"
//     "          \"name\": \"Han Solo\"\n"
//     "        },\n"
//     "      ]" /* todo \n nextNonWhite */
//     "    }\n"
//     "  }\n"
    "}";
    assert_parses(graphResult);
    result.print();
    Node & data = result["data"];
    data.print();
    Node & hero = data["hero"];
    hero.print();
    Node & height = data["hero"]["height"];
    height.print();
    Node & id = hero["id"];
    id.print();
    assert(id == "R2-D2");
    assert(height == 5.6430448);
    //	assert(height==5.643);
    Node & friends = result["data"]["hero"]["friends"];
    assert(friends[0]["name"] == "Luke Skywalker");
    //todo	assert(result["hero"] == result["data"]["hero"]);
    //	assert(result["hero"]["friends"][0]["name"] == "Luke Skywalker")// if 1-child, treat as root
}

#[test]
fn testGraphQlQuery2() {
//     assert_parses("{\n"
//                   "  human(id: \"1000\"){\n"
//                   "    name\n"
//                   "    height(unit: FOOT)\n"
//                   "  }\n"
//                   "}");
    assert(result["human"]["id"] == 1000);
    skip!(
assert(result["id"] == 1000, 0)); // if length==1 descend!
}

#[test]
fn testGraphQlQuerySignificantWhitespace() {
    // human() {} != human(){}
//     assert_parses("{\n"
//                   "  human(id: \"1000\") {\n"
//                   "    name\n"
//                   "    height(unit: FOOT)\n"
//                   "  }\n"
//                   "}");
    assert(result["human"]["id"] == 1000);
    skip!(
assert(result["id"] == 1000, 0)); // if length==1 descend!
}

#[test]
fn testGraphParams() {
//     assert_parses("{\n  empireHero: hero(episode: EMPIRE){\n    name\n  }\n"
//                   "  jediHero: hero(episode: JEDI){\n    name\n  }\n}");
    Node & hero = result["empireHero"];
    hero.print();
    assert(hero["episode"] == "EMPIRE");
//     assert_parses("\nfragment comparisonFields on Character{\n"
//                   "  name\n  appearsIn\n  friends{\n    name\n  }\n }");
    assert_parses("\nfragment comparisonFields on Character{\n  name\n  appearsIn\n  friends{\n    name\n  }\n}");
    // VARIAblE: { "episode": "JEDI" }
//     assert_parses("query HeroNameAndFriends($episode: Episode){\n"
//                   "  hero(episode: $episode){\n"
//                   "    name\n"
//                   "    friends{\n"
//                   "      name\n"
//                   "    }\n"
//                   "  }\n"
//                   "}");
}

#[test]
fn testRootLists() {
    // vargs needs to be 0 terminated, otherwise pray!
    assert_is("1 2 3", Node(1, 2, 3, 0));
    assert_is("(1 2 3)", Node(1, 2, 3, 0));
    assert_is("(1,2,3)", Node(1, 2, 3, 0));
    assert_is("(1;2;3)", Node(1, 2, 3, 0));
//     assert_is("1;2;3", Node(1, 2, 3, 0, 0)) //ok
    assert_is("1,2,3", Node(1, 2, 3, 0));
    assert_is("[1 2 3]", Node(1, 2, 3, 0).setKind(patterns));
    assert_is("[1 2 3]", Node(1, 2, 3, 0));
    assert_is("[1,2,3]", Node(1, 2, 3, 0));
    assert_is("[1,2,3]", Node(1, 2, 3, 0).setKind(patterns));
    assert_is("[1;2;3]", Node(1, 2, 3, 0));
    todo_emit( // todo ?
//                assert_is("{1 2 3}", Node(1, 2, 3, 0));
//     assert_is("{1,2,3}", Node(1, 2, 3, 0));
//     assert_is("{1;2;3}", Node(1, 2, 3, 0));
    );
    todo_emit( // todo symbolic wasm
//                assert_is("(a,b,c)", Node("a", "b", "c", 0));
//     assert_is("(a;b;c)", Node("a", "b", "c", 0));
//     assert_is("a;b;c", Node("a", "b", "c", 0));
//     assert_is("a,b,c", Node("a", "b", "c", 0));
//     assert_is("{a b c}", Node("a", "b", "c", 0));
//     assert_is("{a,b,c}", Node("a", "b", "c", 0));
//     assert_is("[a,b,c]", Node("a", "b", "c", 0));
//     assert_is("(a b c)", Node("a", "b", "c", 0));
//     assert_is("[a;b;c]", Node("a", "b", "c", 0));
//     assert_is("a b c", Node("a", "b", "c", 0, 0));
//     assert_is("{a;b;c}", Node("a", "b", "c", 0));
//     assert_is("[a b c]", Node("a", "b", "c", 0));
    );
}
